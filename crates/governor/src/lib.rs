//! Connector for the Ohio governor's veto messages.
//!
//! The executive counterpart to [`lsc`](../../lsc/). LSC publishes what the legislature did;
//! this reads what the governor undid, and why, in his own words.
//!
//! A veto message is issued under Article II, Section 16 and is highly regular: `ITEM NUMBER
//! n`, one or more `On page N, delete …` instructions, a heading, and a reason closing
//! "Therefore, a veto of this item is in the public interest."
//!
//! # Why the parse is line-based rather than geometric
//!
//! The [`lsc`](../../lsc/) connector reconstructs tables from glyph coordinates because a PDF
//! holds characters at positions and a table's structure is spatial. This document has no
//! tables. Its structure is in the reading order, which a text extraction already preserves,
//! so the geometry machinery would add a failure mode without buying anything.
//!
//! # What this connector never produces
//!
//! A figure. Across HB 96's 67 items not one deletes an appropriation amount, and
//! [`corpus_schema::VetoItemRow`] has no field for one. That is deliberate and load-bearing:
//! striking a set-aside, an earmark, or a recipient restriction redirects an appropriation
//! without changing its total, so a veto that moves millions leaves every line item untouched.
//! See the `appropriation-is-not-distribution` decision record.

#[cfg(feature = "pdf")]
pub mod pdf;

use anyhow::{bail, Result};
use corpus_schema::{Deletion, DeletionExtent, Provenance, VetoItemRow};

/// Curly and straight quote marks, in the forms this document actually uses.
const OPEN_QUOTES: [char; 2] = ['\u{201c}', '"'];
const CLOSE_QUOTES: [char; 2] = ['\u{201d}', '"'];

/// Phrases that open a range deletion. Three are typographical errors in the source —
/// `begging with`, `stating with` — and are matched because refusing them would drop real
/// deletions over a misspelling in a document nobody is going to correct.
const RANGE_OPENERS: [&str; 4] = [
    "beginning with",
    "begging with",
    "stating with",
    "starting with",
];

/// Phrases that separate a range's opening words from its closing words.
///
/// `or ending with` is a slip for `and` in one instruction and means the same thing.
const RANGE_CONNECTORS: [&str; 3] = [
    "and ending with",
    "or ending with",
    "and continuing through",
];

/// Rejoins a line that the text extraction broke mid-word.
///
/// The extractor preserves the document's visual line breaks, so a word hyphenated by
/// justification arrives in two pieces. Whether the hyphen is part of the word or an artefact
/// of setting it cannot be decided in general — but it can be decided here, and the rule was
/// chosen after looking at every one of the 14 breaks in this message rather than in advance:
///
/// | before the hyphen | after the break | example | hyphen |
/// |---|---|---|---|
/// | digit | anything | `90-` `credit-hour` | kept |
/// | letter | uppercase or digit | `DeWine-` `Tressel`, `ADD-` `ON` | kept |
/// | letter | lowercase | `func-` `tions`, `end-` `ing` | dropped |
///
/// All 14 come out right. The residual risk is a genuine compound broken at its own hyphen
/// with a lowercase continuation — `career-` `technical` — which this rule would fuse. This
/// document contains none; the LSC comparison document contains several, which is why
/// [`lsc`](../../lsc/) keeps every hyphen and this does not. Same surface problem, opposite
/// correct answers, because the two publishers set text differently.
fn join_line(acc: &mut String, next: &str) {
    let next = next.trim();
    if next.is_empty() {
        return;
    }
    if let Some(stripped) = acc.strip_suffix('-') {
        let before = stripped.chars().last();
        let after = next.chars().next();
        let soft = before.is_some_and(|c| c.is_alphabetic())
            && after.is_some_and(|c| c.is_lowercase() && c.is_alphabetic());
        if soft {
            acc.truncate(acc.len() - 1);
        }
        acc.push_str(next);
        return;
    }
    if !acc.is_empty() {
        acc.push(' ');
    }
    acc.push_str(next);
}

/// The formula every item's reason closes with.
pub const CLOSING_FORMULA: &str = "is in the public interest";

// ─── text preparation ────────────────────────────────────────────────────────

/// Removes the running foot so it does not land inside an item.
///
/// `Page 12 of 57` appears mid-item wherever an item spans a page break, which is often. Left
/// in, it would be read as a line of the deletion block or of the reason.
pub fn strip_page_furniture(text: &str) -> String {
    text.replace('\u{c}', "\n")
        .lines()
        .filter(|l| !is_running_foot(l))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_running_foot(line: &str) -> bool {
    let t = line.trim();
    let Some(rest) = t.strip_prefix("Page ") else {
        return false;
    };
    // `Page 12 of 57` and nothing else.
    match rest.split_once(" of ") {
        Some((a, b)) => {
            !a.is_empty()
                && a.chars().all(|c| c.is_ascii_digit())
                && !b.is_empty()
                && b.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

// ─── deletion instructions ───────────────────────────────────────────────────

fn starts_instruction(line: &str) -> bool {
    let l = line.trim().to_ascii_lowercase();
    l.starts_with("on page")
}

/// Is this accumulated instruction a complete sentence?
///
/// Two traps, both live in the source:
///
/// - a deletion's quoted text often ends in an **ellipsis** — `“…means a patient...”` — and a
///   naive "ends with a period" test reads that as the end of the sentence, then takes the
///   `and ending with …` continuation to be the item's heading;
/// - the closing quote may or may not be followed by a period, so the check has to look past
///   trailing quote marks.
///
/// The grammar gives a third, stronger signal: an instruction that has opened a range but not
/// closed it is unfinished whatever its punctuation says.
fn is_complete(acc: &str) -> bool {
    let lower = acc.to_ascii_lowercase();
    if RANGE_OPENERS.iter().any(|o| lower.contains(o))
        && !RANGE_CONNECTORS.iter().any(|c| lower.contains(c))
    {
        return false;
    }
    let s = acc.trim_end();
    let s = s.trim_end_matches(CLOSE_QUOTES).trim_end();
    if s.ends_with("...") || s.ends_with('\u{2026}') {
        return false;
    }
    s.ends_with('.')
}

/// Splits the leading run of deletion instructions from the rest of an item.
///
/// Returns the instructions and the index of the first line after them.
pub fn deletion_block(lines: &[&str]) -> (Vec<String>, usize) {
    let mut out: Vec<String> = Vec::new();
    let mut acc = String::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        if starts_instruction(line) {
            if !acc.is_empty() {
                out.push(std::mem::take(&mut acc));
            }
            acc.push_str(line);
            i += 1;
            continue;
        }
        if !acc.is_empty() && !is_complete(&acc) {
            join_line(&mut acc, line);
            i += 1;
            continue;
        }
        break;
    }
    if !acc.is_empty() {
        out.push(acc);
    }
    (out, i)
}

/// The text between the first opening quote and the last closing quote.
///
/// Taking the outermost pair rather than the first matching one is what handles nesting. The
/// message quotes bill text that is itself quoted — `““Charity care” means free or...”` — and
/// pairing the first open with the first close would return `Charity care` and silently drop
/// the definition it introduces.
fn outermost_quoted(s: &str) -> Option<(String, bool)> {
    let open = s.find(OPEN_QUOTES);
    let close = s.rfind(CLOSE_QUOTES);
    let (inner, repaired) = match (open, close) {
        (Some(a), Some(b)) if b > a => (&s[a + s[a..].chars().next()?.len_utf8()..b], false),
        // The message drops a quotation mark three times — `and ending with the effective date
        // of this section.”` opens nothing, `and ending with “at large.` closes nothing. The
        // words are unambiguous from the grammar around them, so they are recovered and the
        // repair is recorded. Discarding a real deletion over a missing mark would understate
        // what the governor struck, which is the error that matters here.
        (Some(a), _) => (&s[a + s[a..].chars().next()?.len_utf8()..], true),
        (None, Some(b)) => (&s[..b], true),
        (None, None) => return None,
    };
    // No punctuation is trimmed. The period in `“entity's medicare cost report.”` is part of
    // the bill text being quoted, not the instruction's own sentence-ending mark. Stray quote
    // marks are trimmed only where a repair was needed, since that is where the source left an
    // unmatched one.
    let t = inner.trim();
    let t = if repaired {
        t.trim_matches(|c| OPEN_QUOTES.contains(&c) || CLOSE_QUOTES.contains(&c))
            .trim()
    } else {
        t
    };
    if t.is_empty() {
        None
    } else {
        Some((t.to_string(), repaired))
    }
}

fn find_any(hay_lower: &str, needles: &[&str]) -> Option<(usize, usize)> {
    needles
        .iter()
        .filter_map(|n| hay_lower.find(n).map(|i| (i, n.len())))
        .min_by_key(|(i, _)| *i)
}

/// Classifies one instruction by how much it removes.
///
/// Classification is by **structure**, not by phrasing. The message says `delete the following
/// boxed text`, `delete the boxed text`, `delete the following text`, `delete the following
/// boxed test`, and plain `delete` for the same act, and matching on those strings would turn
/// a typesetter's slip into a dropped deletion.
pub fn parse_deletion(instruction: &str) -> Deletion {
    let bill_page = instruction
        .split_whitespace()
        .skip_while(|w| !w.eq_ignore_ascii_case("page"))
        .nth(1)
        .and_then(|w| w.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok());

    let lower = instruction.to_ascii_lowercase();
    let mut quotes_repaired = false;
    let extent = match find_any(&lower, &RANGE_OPENERS) {
        Some((at, len)) => {
            let after = &instruction[at + len..];
            match find_any(&after.to_ascii_lowercase(), &RANGE_CONNECTORS) {
                Some((c, clen)) => {
                    match (
                        outermost_quoted(&after[..c]),
                        outermost_quoted(&after[c + clen..]),
                    ) {
                        (Some((begins, r1)), Some((ends, r2))) => {
                            quotes_repaired = r1 || r2;
                            DeletionExtent::Range { begins, ends }
                        }
                        _ => DeletionExtent::Unrecognised,
                    }
                }
                None => DeletionExtent::Unrecognised,
            }
        }
        None => match outermost_quoted(instruction) {
            Some((text, r)) => {
                quotes_repaired = r;
                DeletionExtent::Text { text }
            }
            // `On page 274, delete the boxed text.` — nothing quoted, so the instruction takes
            // whatever is boxed on that page. Real, and not the same as an unrecognised form.
            None => DeletionExtent::Whole,
        },
    };

    Deletion {
        bill_page,
        extent,
        quotes_repaired,
        instruction: instruction.split_whitespace().collect::<Vec<_>>().join(" "),
    }
}

// ─── items ───────────────────────────────────────────────────────────────────

/// A heading longer than this many characters may have wrapped to a second line.
///
/// Headings are set in a bold face and so wrap earlier than the body text around them, which
/// rules out reading the wrap width off the document as a whole. The threshold is only used to
/// decide whether to *look* at the next line; the decision to join is made on that line being
/// short, and a joined heading is flagged.
const HEADING_FILL: usize = 75;
const HEADING_CONTINUATION_MAX: usize = 40;

/// Reads one item's heading, and whether it had to be rejoined across lines.
fn heading(rest: &[&str]) -> (String, bool) {
    let Some(first) = rest.first() else {
        return (String::new(), false);
    };
    let first = first.trim();
    if first.len() >= HEADING_FILL {
        if let Some(next) = rest.get(1) {
            let next = next.trim();
            if !next.is_empty() && next.len() < HEADING_CONTINUATION_MAX {
                return (format!("{first} {next}"), true);
            }
        }
    }
    (first.to_string(), false)
}

/// What a message cannot tell you about itself.
#[derive(Debug, Clone)]
pub struct MessageContext {
    pub bill_number: String,
    pub general_assembly: String,
    pub catalog_slug: String,
    pub retrieved: String,
}

/// Coverage of one parse.
#[derive(Debug, Clone, Default)]
pub struct VetoReport {
    pub items: Vec<VetoItemRow>,
    /// Items whose reason did not end in the closing formula. Reported rather than trimmed:
    /// a reason cut short is a reason misquoted.
    pub items_missing_closing_formula: Vec<u32>,
    /// Items that carried no deletion instruction at all.
    pub items_without_deletions: Vec<u32>,
    /// Instructions whose extent could not be classified.
    pub unrecognised_deletions: Vec<(u32, String)>,
    /// Instructions whose extent was read past a missing quotation mark.
    pub deletions_with_repaired_quotes: Vec<(u32, String)>,
    /// Gaps in the message's own numbering.
    pub missing_item_numbers: Vec<u32>,
}

impl VetoReport {
    pub fn deletion_count(&self) -> usize {
        self.items.iter().map(|i| i.deletions.len()).sum()
    }
    pub fn is_clean(&self) -> bool {
        self.items_missing_closing_formula.is_empty()
            && self.items_without_deletions.is_empty()
            && self.unrecognised_deletions.is_empty()
            && self.missing_item_numbers.is_empty()
    }
}

fn item_number_of(line: &str) -> Option<u32> {
    line.trim()
        .strip_prefix("ITEM NUMBER")?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Parses a whole veto message.
pub fn parse(text: &str, ctx: &MessageContext) -> Result<VetoReport> {
    let cleaned = strip_page_furniture(text);
    let lines: Vec<&str> = cleaned.lines().collect();

    let starts: Vec<(usize, u32)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| item_number_of(l).map(|n| (i, n)))
        .collect();
    if starts.is_empty() {
        bail!(
            "no `ITEM NUMBER` heading found; this is not a veto message, or its text layer did \
             not extract"
        );
    }

    let mut report = VetoReport::default();
    for (k, (at, number)) in starts.iter().enumerate() {
        let end = starts.get(k + 1).map(|(i, _)| *i).unwrap_or(lines.len());
        let body = &lines[at + 1..end];

        let (instructions, after) = deletion_block(body);
        if instructions.is_empty() {
            report.items_without_deletions.push(*number);
        }
        let deletions: Vec<Deletion> = instructions.iter().map(|s| parse_deletion(s)).collect();
        for d in &deletions {
            if d.extent == DeletionExtent::Unrecognised {
                report
                    .unrecognised_deletions
                    .push((*number, d.instruction.clone()));
            }
            if d.quotes_repaired {
                report
                    .deletions_with_repaired_quotes
                    .push((*number, d.instruction.clone()));
            }
        }

        let rest: Vec<&str> = body[after.min(body.len())..]
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        let (title, title_rejoined) = heading(&rest);
        let skip = if title_rejoined { 2 } else { 1 };
        let mut rationale = String::new();
        for line in rest.iter().skip(skip) {
            join_line(&mut rationale, line);
        }
        if !rationale.to_ascii_lowercase().contains(CLOSING_FORMULA) {
            report.items_missing_closing_formula.push(*number);
        }

        report.items.push(VetoItemRow {
            bill_number: ctx.bill_number.clone(),
            general_assembly: ctx.general_assembly.clone(),
            item_number: *number,
            title,
            title_rejoined,
            deletions,
            rationale,
            provenance: Provenance {
                catalog_slug: ctx.catalog_slug.clone(),
                document_ref: format!("item {number}"),
                locator: None,
                retrieved: ctx.retrieved.clone(),
            },
        });
    }

    // The message numbers its own items, so a gap means one was missed rather than that the
    // governor skipped a number.
    let seen: std::collections::BTreeSet<u32> =
        report.items.iter().map(|i| i.item_number).collect();
    if let (Some(lo), Some(hi)) = (seen.iter().next(), seen.iter().next_back()) {
        report.missing_item_numbers = (*lo..=*hi).filter(|n| !seen.contains(n)).collect();
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> MessageContext {
        MessageContext {
            bill_number: "HB 96".into(),
            general_assembly: "136th".into(),
            catalog_slug: "governor-hb96-veto-messages".into(),
            retrieved: "2026-08-08".into(),
        }
    }

    #[test]
    fn the_running_foot_is_removed_wherever_it_lands() {
        let t = "ITEM NUMBER 1\nOn page 1, delete the boxed text.\nPage 12 of 57\nA Title\n";
        assert!(!strip_page_furniture(t).contains("Page 12 of 57"));
        // A line that merely starts with the word Page is not furniture.
        assert!(strip_page_furniture("Page layout rules apply.").contains("Page layout"));
    }

    #[test]
    fn a_quoted_deletion_keeps_its_nested_quotes() {
        // The message quotes bill text that is itself quoted. Pairing the first open quote
        // with the first close returns `Charity care` and drops the definition it introduces.
        let d = parse_deletion(
            "On page 1430, delete the boxed text beginning with \u{201c}\u{201c}Charity care\u{201d} \
             means free or...\u{201d} and ending with \u{201c}entity\u{2019}s medicare cost report.\u{201d}.",
        );
        assert_eq!(d.bill_page, Some(1430));
        match d.extent {
            DeletionExtent::Range { begins, ends } => {
                assert!(begins.starts_with('\u{201c}'), "{begins}");
                assert!(begins.contains("means free or"), "{begins}");
                assert_eq!(ends, "entity\u{2019}s medicare cost report.");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_three_forms_are_told_apart_by_structure_not_phrasing() {
        let text =
            parse_deletion("On page 1, delete the following boxed text, \u{201c}131.02,\u{201d}.");
        assert_eq!(
            text.extent,
            DeletionExtent::Text {
                text: "131.02,".into()
            }
        );

        // No quoted passage: the instruction takes whatever is boxed on that page.
        let whole = parse_deletion("On page 274, delete the boxed text.");
        assert_eq!(whole.extent, DeletionExtent::Whole);
        assert_eq!(whole.bill_page, Some(274));

        // Same act, four different wordings in the source — two of them misspelled.
        for s in [
            "On page 9, delete the following text, \u{201c}3513.05,\u{201d}.",
            "On page 2437, delete the boxed text, \u{201c}(D)(E)\u{201d}.",
            "On page 2708, delete the following boxed test, \u{201c}5168.25,\u{201d}.",
            "On page 2708, delete \u{201c}5705.233,\u{201d}.",
        ] {
            assert!(
                matches!(parse_deletion(s).extent, DeletionExtent::Text { .. }),
                "{s}"
            );
        }
        for s in [
            "On page 2501, delete the boxed text begging with \u{201c}That person is not...\u{201d} \
             and ending with \u{201c}the printed matter.\u{201d}.",
            "On page 520, delete the boxed text stating with \u{201c}Sec. 505.37. (A)...\u{201d} \
             and ending with \u{201c}within the township;\u{201d}.",
            "On page 2450, delete the boxed text beginning with \u{201c}Sec. 5705.60.\u{201d} \
             and continuing through \u{201c}the current year.\u{201d}.",
        ] {
            assert!(
                matches!(parse_deletion(s).extent, DeletionExtent::Range { .. }),
                "{s}"
            );
        }
    }

    #[test]
    fn an_ellipsis_does_not_end_a_deletion_instruction() {
        // The trap that cost two items their headings: a quoted passage ending in `...` reads
        // as a finished sentence, so the `and ending with …` continuation became the heading.
        let lines = vec![
            "On page 1430, delete the boxed text beginning with \u{201c}\u{201c}Low-income \
             patient\u{201d} means a patient...\u{201d}",
            "and ending with \u{201c}of the federal poverty line.\u{201d}.",
            "340B Reporting Requirements",
            "This item would ... is in the public interest.",
        ];
        let (dels, after) = deletion_block(&lines);
        assert_eq!(dels.len(), 1, "{dels:?}");
        assert_eq!(after, 2, "the heading must not be swallowed");
        assert!(matches!(
            parse_deletion(&dels[0]).extent,
            DeletionExtent::Range { .. }
        ));
    }

    #[test]
    fn a_closing_quote_without_a_period_still_ends_an_instruction() {
        // Two items end a deletion `…Project.\u{201d}` with no trailing period. Requiring one
        // absorbed the heading into the deletion block.
        let lines = vec![
            "On page 2911, delete the boxed text beginning with \u{201c}The Director...\u{201d}",
            "and ending with \u{201c}support the Indian Lake Watershed Project.\u{201d}",
            "Waterways Improvement and Cash Transfer to the GRF",
            "This item establishes three earmarks ... is in the public interest.",
        ];
        let (dels, after) = deletion_block(&lines);
        assert_eq!(dels.len(), 1);
        assert_eq!(after, 2, "heading swallowed: {dels:?}");
    }

    #[test]
    fn a_capitalised_page_still_opens_an_instruction() {
        let lines = vec![
            "On Page 3040, delete the boxed text beginning with \u{201c}On July 1...\u{201d} and",
            "ending with \u{201c}to the General Revenue Fund.\u{201d}.",
            "Retain important cash funds",
        ];
        let (dels, after) = deletion_block(&lines);
        assert_eq!(dels.len(), 1, "{dels:?}");
        assert_eq!(after, 2);
    }

    #[test]
    fn a_heading_that_wrapped_is_rejoined_and_flagged() {
        let long =
            "Transfers from the Health and Human Services Reserve Fund to the General Revenue";
        assert!(long.len() >= HEADING_FILL);
        let (t, rejoined) = heading(&[long, "Fund", "This section permits the director..."]);
        assert_eq!(t, format!("{long} Fund"));
        assert!(rejoined, "a reconstructed heading must say so");

        // A short heading is complete, and the paragraph after it is not part of it.
        let (t, rejoined) = heading(&["Senior Community Services", "This item would limit ..."]);
        assert_eq!(t, "Senior Community Services");
        assert!(!rejoined);
    }

    #[test]
    fn a_word_broken_by_the_extractor_is_rejoined_the_way_the_document_meant_it() {
        // Every one of the 14 breaks in the real message, by shape.
        let cases = [
            // hyphen kept: digit before it, or an uppercase/digit continuation
            (
                "enrollment of students in 90-",
                "credit-hour degree",
                "90-credit-hour",
            ),
            ("MEDICAID ADD-", "ON\u{201d} and ending", "ADD-ON"),
            (
                "While the DeWine-",
                "Tressel Administration",
                "DeWine-Tressel",
            ),
            ("fiscal year 2026-", "2027 biennium", "2026-2027"),
            // hyphen dropped: a letter before it and a lowercase continuation
            ("designated a re-", "placement levy", "replacement"),
            (
                "ending with \u{201c}func-",
                "tions related to it.",
                "functions",
            ),
            ("and end-", "ing with \u{201c}(B)(1)", "ending"),
            ("unless other-", "wise provided", "otherwise"),
        ];
        for (a, b, want) in cases {
            let mut acc = a.to_string();
            join_line(&mut acc, b);
            assert!(
                acc.contains(want),
                "{a:?} + {b:?} -> {acc:?}, wanted {want:?}"
            );
        }
    }

    #[test]
    fn a_hyphenated_break_does_not_reach_across_a_normal_join() {
        let mut acc = "the boxed text".to_string();
        join_line(&mut acc, "beginning with");
        assert_eq!(acc, "the boxed text beginning with");
    }

    #[test]
    fn or_ending_with_is_read_as_the_slip_it_is() {
        // One instruction says `or ending with` where every other says `and`.
        let d = parse_deletion(
            "On page 64, delete the boxed text beginning with \u{201c}is filed thirty-five...\u{201d} \
             or ending with \u{201c}after the proposed rule was filed.\u{201d}.",
        );
        assert!(matches!(d.extent, DeletionExtent::Range { .. }), "{d:?}");
        assert!(!d.quotes_repaired);
    }

    #[test]
    fn a_missing_quotation_mark_is_repaired_and_recorded() {
        // Closing mark present, opening mark absent.
        let d = parse_deletion(
            "On page 259, delete the boxed text beginning with \u{201c}Sec. 126.17.(A) As used...\u{201d} \
             and ending with the effective date of this section.\u{201d}.",
        );
        assert!(d.quotes_repaired, "the repair must be recorded");
        match d.extent {
            DeletionExtent::Range { ends, .. } => {
                assert_eq!(ends, "the effective date of this section.")
            }
            other => panic!("{other:?}"),
        }

        // Opening mark present, closing mark absent.
        let d = parse_deletion(
            "On page 1329, delete the boxed text beginning with \u{201c}Within the rectangular \
             space ...\u{201d} and ending with \u{201c}at large.",
        );
        assert!(d.quotes_repaired);
        match d.extent {
            DeletionExtent::Range { ends, .. } => assert_eq!(ends, "at large."),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_well_formed_instruction_is_never_marked_repaired() {
        let d =
            parse_deletion("On page 1, delete the following boxed text, \u{201c}131.02,\u{201d}.");
        assert!(!d.quotes_repaired);
    }

    #[test]
    fn a_full_item_parses_into_its_parts() {
        let msg = "\
VETO MESSAGES
ITEM NUMBER 1
On page 2724, delete the following boxed text, \u{201c}The Department shall not use any of these funds
for administrative expenses.\u{201d}.
Senior Community Services
This item would limit the Ohio Department of Aging's ability to administer senior programming.
Therefore, a veto of this item is in the public interest.
ITEM NUMBER 2
On page 274, delete the boxed text.
Notice for the Debt Payable to the State
This item would require additional steps. Therefore, a veto of this item is in the public interest.
";
        let r = parse(msg, &ctx()).unwrap();
        assert!(r.is_clean(), "{r:?}");
        assert_eq!(r.items.len(), 2);
        assert_eq!(r.deletion_count(), 2);

        let one = &r.items[0];
        assert_eq!(one.item_number, 1);
        assert_eq!(one.title, "Senior Community Services");
        assert!(one.rationale.starts_with("This item would limit"));
        assert!(one.rationale.ends_with("in the public interest."));
        assert_eq!(one.provenance.document_ref, "item 1");
        match &one.deletions[0].extent {
            DeletionExtent::Text { text } => {
                assert!(text.contains("administrative expenses"), "{text}")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_gap_in_the_numbering_is_reported() {
        let msg = "ITEM NUMBER 1\nOn page 1, delete the boxed text.\nA\nB is in the public interest.\n\
                   ITEM NUMBER 3\nOn page 2, delete the boxed text.\nC\nD is in the public interest.\n";
        let r = parse(msg, &ctx()).unwrap();
        assert_eq!(r.missing_item_numbers, vec![2]);
        assert!(!r.is_clean());
    }

    #[test]
    fn an_item_whose_reason_is_cut_short_is_reported() {
        let msg = "ITEM NUMBER 1\nOn page 1, delete the boxed text.\nA Title\nA reason with no formula.\n";
        let r = parse(msg, &ctx()).unwrap();
        assert_eq!(r.items_missing_closing_formula, vec![1]);
    }

    #[test]
    fn a_document_with_no_items_is_refused_by_name() {
        let e = parse("Some other document entirely.", &ctx())
            .unwrap_err()
            .to_string();
        assert!(e.contains("not a veto message"), "{e}");
    }
}
