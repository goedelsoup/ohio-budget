//! Reads an LSC comparison document into provisions and the position each stage took.
//!
//! [`crate::ruled`] recovers the page's geometry. This turns that geometry into the thing the
//! corpus is short of: **why** a figure changed. The appropriation spreadsheet says foundation
//! funding rose by $92,250,000 at the House substitute; only this document says the House kept
//! the executive's foundation aid calculations but used them "only for purposes of calculating
//! a district's temporary foundation funding" — retaining the formula as an input to a
//! different distribution rather than as the distribution itself.
//!
//! # The four columns are coarser than the nine stages
//!
//! This is the limit to keep in view, because it is easy to over-read what comes out of here.
//! The spreadsheet carries nine stages. The comparison document carries four:
//!
//! | comparison column | stage |
//! |---|---|
//! | Executive | as-introduced |
//! | As Passed By House | as-passed-house |
//! | As Passed By Senate | as-passed-senate |
//! | As Enacted | as-enacted |
//!
//! So `As Passed By House` describes the House's *cumulative* position — substitute, committee
//! report, and floor together — while the money moved at a nameable one of those. A
//! justification taken from here attributes to a chamber, not to a stage, and saying otherwise
//! would be reading precision into the document that it does not carry.
//!
//! For HB 96 specifically that costs less than it sounds, and the reason is measured rather
//! than assumed: `stage-delta` found that **both chamber floors moved zero dollars**, so the
//! House column's content and the House substitute's arithmetic describe the same event. That
//! is a property of this bill. It is not a licence to collapse the two in general.
//!
//! # `Executive` and `Introduced` name the same version
//!
//! The spreadsheet's first stage column is headed `Introduced`; this document heads the same
//! column `Executive`. They are the executive's proposal as introduced — the corpus's
//! `as-introduced`. Mapping them together is what lets a figure and its justification meet.

use anyhow::{bail, Result};
use corpus_schema::{BillStage, LscProvisionRow, Provenance};

use crate::geometry::Glyph;
use crate::ruled::{
    default_running_threshold, detect_running_lines, to_ruled_table, to_ruled_table_with_rules,
    RuleOptions, RuledLine, RuledTable, RunningLines,
};

/// One column of a comparison document.
#[derive(Debug, Clone, PartialEq)]
pub struct StageColumn {
    /// The label as printed, e.g. `As Passed By House`.
    pub label: String,
    /// `None` where the label is not one this connector recognises. Reported, never guessed:
    /// a column silently mapped to the wrong stage attributes a chamber's decision to another.
    pub stage: Option<BillStage>,
}

/// Maps a comparison-document column heading to a bill stage.
///
/// Deliberately separate from [`BillStage::parse_label`], which reads the *spreadsheet's*
/// vocabulary. The two publications label the same stages differently and the difference is
/// not cosmetic — `Executive` has no spreadsheet equivalent at all, and reading it as a stage
/// requires knowing that LSC uses it for the bill as introduced.
pub fn stage_of_column(label: &str) -> Option<BillStage> {
    let n: String = label
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let n = n.split_whitespace().collect::<Vec<_>>().join(" ");
    match n.as_str() {
        // The executive's proposal is what gets introduced; the two publications differ only
        // in which end of that they name.
        "executive" | "executive proposal" | "as introduced" | "introduced" => {
            Some(BillStage::AsIntroduced)
        }
        "as passed by house" | "as passed by the house" | "house passed" => {
            Some(BillStage::AsPassedHouse)
        }
        "as passed by senate" | "as passed by the senate" | "senate passed" => {
            Some(BillStage::AsPassedSenate)
        }
        "conference report" | "as reported by conference committee" => {
            Some(BillStage::ConferenceReport)
        }
        "as enacted" | "enacted" => Some(BillStage::AsEnacted),
        _ => None,
    }
}

/// One comparison entry: what each column says about a single point of a provision.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// Page the entry began on.
    pub page: u32,
    /// Prose per column, parallel to [`ComparisonDocument::columns`].
    pub cells: Vec<String>,
}

impl Position {
    /// What one stage said. `None` if that stage is not a column of this document.
    pub fn for_stage(&self, columns: &[StageColumn], stage: BillStage) -> Option<&str> {
        columns
            .iter()
            .position(|c| c.stage == Some(stage))
            .and_then(|i| self.cells.get(i))
            .map(String::as_str)
    }

    /// True where every column after the first only cross-references an earlier one.
    ///
    /// The point of this is to skip past provisions nobody touched, so that a reader looking
    /// for what changed is not wading through concurrence.
    pub fn is_unchanged_after_executive(&self) -> bool {
        self.cells
            .iter()
            .skip(1)
            .all(|c| c.trim().is_empty() || is_bare_concurrence(c))
    }
}

/// Does this cell say only "the same as an earlier stage", with no qualification?
///
/// The obvious test — does it start with `Same as` — is wrong, and wrong in the direction that
/// hides exactly what the corpus is looking for. `Same as the Executive, but makes the
/// following changes:` opens with those words and then describes an amendment; it occurs 12
/// times in HB 96's education comparison document alone, and the House's actual changes to
/// school funding are among them. A prefix test would file all of those under "nobody touched
/// this".
///
/// So the match is against the complete phrase. An unrecognised cell counts as a change, which
/// is the safe direction to be wrong in: it gets read rather than skipped.
///
/// `No provision.` is deliberately not on the list. It is a cross-reference in form only —
/// where an earlier stage had a provision and this one says `No provision`, that is the
/// difference, not the absence of one.
pub fn is_bare_concurrence(cell: &str) -> bool {
    let mut s = cell.trim();
    // Entries in a numbered list carry the number in every column: `(1) Same as the Executive.`
    if let Some(rest) = s.strip_prefix('(') {
        if let Some((n, tail)) = rest.split_once(')') {
            if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
                s = tail.trim();
            }
        }
    }
    let n: String = s
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    matches!(
        n.as_str(),
        "same as the executive"
            | "same as the executive proposal"
            | "same as the house"
            | "same as the senate"
            | "same as the conference report"
    )
}

/// A provision and the positions taken on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Provision {
    /// LSC's own identifier, e.g. `EDUCD19`. `None` where the heading did not carry one.
    pub code: Option<String>,
    pub title: String,
    /// The section heading in force, e.g. `School Funding`.
    pub section: Option<String>,
    pub page: u32,
    /// Statutory citations as printed, per column.
    pub citations: Vec<String>,
    pub positions: Vec<Position>,
}

/// A whole comparison document.
#[derive(Debug, Clone)]
pub struct ComparisonDocument {
    pub columns: Vec<StageColumn>,
    pub provisions: Vec<Provision>,
    /// Pages that neither drew rules nor could inherit them. Reported so a run can state its
    /// own coverage rather than quietly returning less than the document held.
    pub refused_pages: Vec<(u32, String)>,
    /// Pages read with rules inherited from the page before.
    pub continuation_pages: Vec<u32>,
    /// Column headings this connector does not recognise.
    pub unmapped_columns: Vec<String>,
}

impl ComparisonDocument {
    pub fn find(&self, code: &str) -> Option<&Provision> {
        self.provisions
            .iter()
            .find(|p| p.code.as_deref() == Some(code))
    }

    /// Provenance for a provision, naming the page it was read from.
    pub fn provenance(&self, catalog_slug: &str, retrieved: &str, p: &Provision) -> Provenance {
        Provenance {
            catalog_slug: catalog_slug.to_string(),
            document_ref: match &p.code {
                Some(c) => format!("provision {c}"),
                None => format!("provision {:?}", p.title),
            },
            locator: Some(format!("page {}", p.page)),
            retrieved: retrieved.to_string(),
        }
    }
}

/// Splits `EDUCD19 Career-tech associated services funding` into its code and its title.
///
/// The shape is a run of capitals then a run of digits, which is narrow enough not to fire on
/// `R.C. 3317.014` (one capital), `Sections 265.220` (one capital), or `School Funding` (no
/// digits). Where it does not fire the heading is kept with no code rather than discarded.
pub fn split_provision_code(s: &str) -> Option<(String, String)> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_uppercase() {
        i += 1;
    }
    if !(3..=8).contains(&i) {
        return None;
    }
    let letters = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if !(1..=4).contains(&(i - letters)) {
        return None;
    }
    // No separator is required after the digits. The heading is typeset as one run and the
    // gap between code and title is often too small to register as a space, so the real
    // document yields `EDUCD26Traditional school district funding formula` as often as not.
    Some((s[..i].to_string(), s[i..].trim().to_string()))
}

/// Which column headings a line carries, if it is the header row.
fn header_columns(line: &RuledLine) -> Option<Vec<StageColumn>> {
    let filled: Vec<&String> = line.cells.iter().filter(|c| !c.trim().is_empty()).collect();
    if filled.len() < 2 {
        return None;
    }
    let mapped = filled
        .iter()
        .filter(|c| stage_of_column(c).is_some())
        .count();
    // A majority has to map. One stray cell that happens to read like a stage is not a header.
    if mapped * 2 <= filled.len() {
        return None;
    }
    Some(
        line.cells
            .iter()
            .map(|c| StageColumn {
                label: c.trim().to_string(),
                stage: stage_of_column(c),
            })
            .collect(),
    )
}

/// The document's body font size, as the most common line size.
fn body_font_size(pages: &[(u32, RuledTable)]) -> f64 {
    let mut counts: std::collections::HashMap<i64, usize> = Default::default();
    for (_, t) in pages {
        for l in t.lines.iter().filter(|l| !l.is_blank()) {
            *counts
                .entry((l.font_size * 10.0).round() as i64)
                .or_default() += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(k, _)| k as f64 / 10.0)
        .unwrap_or(12.0)
}

/// Reads a whole comparison document.
///
/// Takes pages of positioned glyphs so the PDF stack stays behind its feature flag and this
/// can be tested without a document.
pub fn assemble(pages: &[(u32, Vec<Glyph>)], opts: &RuleOptions) -> Result<ComparisonDocument> {
    if pages.is_empty() {
        bail!("no pages");
    }

    // Pass one: geometry. A page with no rules of its own inherits the last page's, which is
    // what makes an entry that overruns its page readable rather than truncated.
    let mut tables: Vec<(u32, RuledTable)> = Vec::new();
    let mut refused_pages = Vec::new();
    let mut continuation_pages = Vec::new();
    let mut carried: Option<Vec<f64>> = None;
    for (n, glyphs) in pages {
        match to_ruled_table(glyphs, opts) {
            Ok(t) => {
                carried = Some(t.rules.clone());
                tables.push((*n, t));
            }
            Err(first) => match &carried {
                Some(rules) => match to_ruled_table_with_rules(glyphs, rules, opts) {
                    Ok(t) => {
                        continuation_pages.push(*n);
                        tables.push((*n, t));
                    }
                    Err(e) => refused_pages.push((*n, format!("{e:#}"))),
                },
                None => refused_pages.push((*n, format!("{first:#}"))),
            },
        }
    }
    if tables.is_empty() {
        bail!("no page of this document drew column rules; it is not a comparison document");
    }

    // The column header repeats on every page, so it is furniture — read it before stripping.
    let columns = tables
        .iter()
        .find_map(|(_, t)| t.lines.iter().find_map(header_columns))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no row of stage headings found; without it a column cannot be attributed to a \
                 stage and every position would be anonymous"
            )
        })?;
    let unmapped_columns: Vec<String> = columns
        .iter()
        .filter(|c| c.stage.is_none() && !c.label.is_empty())
        .map(|c| c.label.clone())
        .collect();

    let running: RunningLines = detect_running_lines(
        &tables.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>(),
        default_running_threshold(tables.len()),
    );
    let body = body_font_size(&tables);

    // Pass two: one stream across page boundaries, because provisions do not respect them.
    let stream: Vec<(u32, RuledLine)> = tables
        .iter()
        .flat_map(|(n, t)| {
            t.without_running(&running)
                .lines
                .into_iter()
                .filter(|l| !l.is_blank())
                .map(move |l| (*n, l))
        })
        .collect();

    let mut provisions: Vec<Provision> = Vec::new();
    let mut section: Option<String> = None;
    let mut cur: Option<Provision> = None;
    let mut entry: Option<Position> = None;
    let mut in_citations = false;

    // Closing a provision closes its open entry first, so no entry is lost at a boundary.
    macro_rules! close_entry {
        () => {
            if let (Some(p), Some(e)) = (cur.as_mut(), entry.take()) {
                p.positions.push(e);
            }
        };
    }

    for (page, line) in stream {
        let structural = line.font_size < body - 0.5;

        if structural {
            if let Some((code, title)) = split_provision_code(line.text.trim()) {
                close_entry!();
                if let Some(p) = cur.take() {
                    provisions.push(p);
                }
                cur = Some(Provision {
                    code: Some(code),
                    title,
                    section: section.clone(),
                    page,
                    citations: Vec::new(),
                    positions: Vec::new(),
                });
                in_citations = true;
                continue;
            }
            if in_citations {
                if let Some(p) = cur.as_mut() {
                    p.citations.push(line.text.trim().to_string());
                }
                continue;
            }
            // A lone heading between provisions names the section that follows.
            if line.cells.iter().filter(|c| !c.trim().is_empty()).count() == 1 {
                close_entry!();
                if let Some(p) = cur.take() {
                    provisions.push(p);
                }
                section = Some(line.text.trim().to_string());
            }
            continue;
        }

        in_citations = false;
        if cur.is_none() {
            // Body text before any provision heading — a preface. Nothing to attach it to.
            continue;
        }
        if line.starts_entry {
            close_entry!();
            entry = Some(Position {
                page,
                cells: vec![String::new(); columns.len()],
            });
        }
        let Some(e) = entry.as_mut() else {
            // Continuation of an entry that began before this provision's heading — that is,
            // the tail of the previous provision. Already closed; dropping it here would be
            // wrong, so it is attached to the provision it belongs to.
            if let Some(prev) = provisions.last_mut() {
                if let Some(last) = prev.positions.last_mut() {
                    append_line(&mut last.cells, &line);
                }
            }
            continue;
        };
        append_line(&mut e.cells, &line);
    }
    close_entry!();
    if let Some(p) = cur.take() {
        provisions.push(p);
    }

    Ok(ComparisonDocument {
        columns,
        provisions,
        refused_pages,
        continuation_pages,
        unmapped_columns,
    })
}

/// What a document cannot tell you about itself.
#[derive(Debug, Clone)]
pub struct DocumentContext {
    pub bill_number: String,
    pub general_assembly: String,
    pub catalog_slug: String,
    pub retrieved: String,
}

/// Coverage of one extraction run.
#[derive(Debug, Clone, Default)]
pub struct ProvisionReport {
    pub rows: Vec<LscProvisionRow>,
    /// Provisions whose heading carried no LSC code. Emitting them under an invented
    /// identifier would put a node in the corpus that cannot be found again in the source.
    pub uncoded_provisions: Vec<String>,
    /// Columns whose heading did not map to a stage. Their prose is dropped, because a
    /// position with no stage cannot be attributed to anyone.
    pub unmapped_columns: Vec<String>,
    pub empty_cells: usize,
}

/// Flattens a document into one record per (provision, entry, stage).
pub fn to_rows(doc: &ComparisonDocument, ctx: &DocumentContext) -> ProvisionReport {
    let mut report = ProvisionReport {
        unmapped_columns: doc.unmapped_columns.clone(),
        ..Default::default()
    };
    for p in &doc.provisions {
        let Some(code) = p.code.clone() else {
            report.uncoded_provisions.push(p.title.clone());
            continue;
        };
        let provenance = doc.provenance(&ctx.catalog_slug, &ctx.retrieved, p);
        for (entry_index, pos) in p.positions.iter().enumerate() {
            for (i, cell) in pos.cells.iter().enumerate() {
                let Some(stage) = doc.columns.get(i).and_then(|c| c.stage) else {
                    continue;
                };
                if cell.trim().is_empty() {
                    // A column that printed nothing said nothing. Recording it as an empty
                    // position would assert that the stage took no view, which is different.
                    report.empty_cells += 1;
                    continue;
                }
                report.rows.push(LscProvisionRow {
                    bill_number: ctx.bill_number.clone(),
                    general_assembly: ctx.general_assembly.clone(),
                    provision_code: code.clone(),
                    provision_title: p.title.clone(),
                    section: p.section.clone(),
                    entry_index,
                    stage,
                    position: cell.trim().to_string(),
                    concurs: is_bare_concurrence(cell),
                    provenance: provenance.clone(),
                });
            }
        }
    }
    report
}

/// Appends a wrapped line to an entry, one column at a time.
///
/// A trailing hyphen joins without a space: these documents break at hyphens already in the
/// word, so `career-` + `technical` is `career-technical`. Keeping the hyphen is the safe
/// error in both directions — right where the break was at a real hyphen, and visible to a
/// reader where it was not, rather than silently fusing two words into one never written.
fn append_line(cells: &mut [String], line: &RuledLine) {
    for (i, add) in line.cells.iter().enumerate() {
        let add = add.trim();
        if add.is_empty() || i >= cells.len() {
            continue;
        }
        let dst = &mut cells[i];
        if !dst.is_empty() && !dst.ends_with('-') {
            dst.push(' ');
        }
        dst.push_str(add);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_publications_agree_on_the_first_stage() {
        // The spreadsheet heads it `Introduced` and the comparison document heads it
        // `Executive`. If these did not land on the same stage, a figure and its justification
        // could never be joined.
        assert_eq!(stage_of_column("Executive"), Some(BillStage::AsIntroduced));
        assert_eq!(
            BillStage::parse_label("Introduced"),
            Some(BillStage::AsIntroduced)
        );
    }

    #[test]
    fn the_printed_column_headings_map_to_stages() {
        for (label, want) in [
            ("As Passed By House", BillStage::AsPassedHouse),
            ("As Passed By Senate", BillStage::AsPassedSenate),
            ("As Enacted", BillStage::AsEnacted),
        ] {
            assert_eq!(stage_of_column(label), Some(want), "{label}");
        }
    }

    #[test]
    fn an_unrecognised_heading_is_not_guessed_at() {
        assert_eq!(stage_of_column("As Reported By House Finance"), None);
        assert_eq!(stage_of_column("Fiscal effect"), None);
    }

    #[test]
    fn a_provision_code_splits_off_its_title() {
        assert_eq!(
            split_provision_code("EDUCD19 Career-tech associated services funding"),
            Some((
                "EDUCD19".into(),
                "Career-tech associated services funding".into()
            ))
        );
        assert_eq!(
            split_provision_code("EDUCD26Traditional school district funding formula"),
            Some((
                "EDUCD26".into(),
                "Traditional school district funding formula".into()
            ))
        );
    }

    #[test]
    fn things_that_merely_look_like_codes_are_rejected() {
        // Every one of these appears as a structural line in the real document.
        for s in [
            "R.C. 3317.014",
            "Sections 265.220, 265.230, 265.450",
            "School Funding",
            "Appropriation Language",
            "Section: 265.280 IId-5486",
        ] {
            assert_eq!(split_provision_code(s), None, "{s}");
        }
    }

    // ─── end to end, on synthetic glyphs ─────────────────────────────────────

    const FS_BODY: f64 = 12.0;
    const FS_HEAD: f64 = 11.0;
    const RULE_X: [f64; 3] = [251.9, 499.4, 746.9];
    const COL_X: [f64; 4] = [10.0, 252.5, 513.0, 762.5];

    fn glyphs(y: f64, x0: f64, s: &str, fs: f64) -> Vec<Glyph> {
        let adv = fs * 0.4;
        s.chars()
            .enumerate()
            .map(|(i, c)| Glyph::new(x0 + i as f64 * adv, y, adv, fs, &c.to_string()))
            .collect()
    }

    fn row(y: f64, cells: [&str; 4], fs: f64) -> Vec<Glyph> {
        let mut g = Vec::new();
        for (i, c) in cells.iter().enumerate() {
            if !c.is_empty() {
                g.extend(glyphs(y, COL_X[i], c, fs));
            }
        }
        g
    }

    fn rules(y: f64) -> Vec<Glyph> {
        RULE_X
            .iter()
            .map(|x| Glyph::new(*x, y, 4.0, 20.0, "|"))
            .collect()
    }

    fn foot(y: f64, page: u32) -> Vec<Glyph> {
        let mut g = glyphs(y, 10.0, "Legislative Budget Office", FS_BODY);
        g.push(Glyph::new(501.5, y, 4.0, FS_BODY, "|"));
        g.extend(glyphs(y, 520.0, &format!("{page}"), FS_BODY));
        g
    }

    /// Two pages modelled on the real document: a heading, citations, entries marked by
    /// drawn rules, an entry that overruns onto a page with no rules, and a running foot.
    fn two_pages() -> Vec<(u32, Vec<Glyph>)> {
        let mut p1 = Vec::new();
        p1.extend(row(
            548.0,
            [
                "Executive",
                "As Passed By House",
                "As Passed By Senate",
                "As Enacted",
            ],
            FS_BODY,
        ));
        p1.extend(row(520.0, ["School Funding", "", "", ""], FS_HEAD));
        p1.extend(row(
            502.0,
            ["EDUCD26 Traditional school district", "", "", ""],
            FS_HEAD,
        ));
        p1.extend(row(
            490.0,
            [
                "R.C. 3317.022",
                "R.C. 3317.022",
                "R.C. 3317.022",
                "R.C. 3317.022",
            ],
            FS_HEAD,
        ));
        p1.extend(row(
            465.0,
            [
                "Extends the school",
                "Same as the Executive,",
                "Same as the House.",
                "Same as the Senate.",
            ],
            FS_BODY,
        ));
        p1.extend(rules(459.0));
        p1.extend(row(
            450.0,
            ["financing formula.", "but raises the", "", ""],
            FS_BODY,
        ));
        p1.extend(row(435.0, ["", "base cost.", "", ""], FS_BODY));
        p1.extend(row(
            400.0,
            [
                "Fiscal effect: allocates",
                "Fiscal effect: increases",
                "Same as the Executive.",
                "Same as the House.",
            ],
            FS_BODY,
        ));
        p1.extend(rules(394.0));
        p1.extend(row(
            385.0,
            ["$8.09 billion in FY", "the allocation by", "", ""],
            FS_BODY,
        ));
        p1.extend(foot(23.5, 1));

        // No rules drawn: the entry above runs on.
        let mut p2 = Vec::new();
        p2.extend(row(
            548.0,
            [
                "Executive",
                "As Passed By House",
                "As Passed By Senate",
                "As Enacted",
            ],
            FS_BODY,
        ));
        p2.extend(row(520.0, ["2026.", "$132.4 million.", "", ""], FS_BODY));
        p2.extend(foot(23.5, 2));

        vec![(1, p1), (2, p2)]
    }

    fn doc() -> ComparisonDocument {
        assemble(&two_pages(), &RuleOptions::default()).unwrap()
    }

    #[test]
    fn columns_are_read_from_the_printed_headings() {
        let d = doc();
        assert_eq!(d.columns.len(), 4);
        assert_eq!(d.columns[0].stage, Some(BillStage::AsIntroduced));
        assert_eq!(d.columns[3].stage, Some(BillStage::AsEnacted));
        assert!(d.unmapped_columns.is_empty(), "{:?}", d.unmapped_columns);
    }

    #[test]
    fn a_provision_carries_its_code_section_and_citations() {
        let d = doc();
        let p = d.find("EDUCD26").expect("provision not found");
        assert_eq!(p.title, "Traditional school district");
        assert_eq!(p.section.as_deref(), Some("School Funding"));
        assert_eq!(p.citations.len(), 1);
        assert_eq!(p.page, 1);
    }

    #[test]
    fn each_stage_position_is_reachable_by_stage_rather_than_by_index() {
        let d = doc();
        let p = d.find("EDUCD26").unwrap();
        let first = &p.positions[0];
        assert_eq!(
            first.for_stage(&d.columns, BillStage::AsIntroduced),
            Some("Extends the school financing formula.")
        );
        assert_eq!(
            first.for_stage(&d.columns, BillStage::AsPassedHouse),
            Some("Same as the Executive, but raises the base cost.")
        );
        // Asking for a stage the document does not carry gets nothing, not a neighbour.
        assert_eq!(
            first.for_stage(&d.columns, BillStage::HouseSubstitute),
            None
        );
    }

    #[test]
    fn an_entry_that_overruns_its_page_is_completed_not_truncated() {
        // Page 2 draws no rules, so it records no column boundaries of its own. Read alone it
        // would be refused and this sentence would end mid-figure.
        let d = doc();
        assert_eq!(d.continuation_pages, vec![2]);
        assert!(d.refused_pages.is_empty(), "{:?}", d.refused_pages);

        let p = d.find("EDUCD26").unwrap();
        let fiscal = &p.positions[1];
        assert_eq!(
            fiscal.for_stage(&d.columns, BillStage::AsIntroduced),
            Some("Fiscal effect: allocates $8.09 billion in FY 2026.")
        );
        assert_eq!(
            fiscal.for_stage(&d.columns, BillStage::AsPassedHouse),
            Some("Fiscal effect: increases the allocation by $132.4 million.")
        );
    }

    #[test]
    fn the_running_foot_does_not_end_up_inside_an_entry() {
        // Without suppression the last entry on every page ends with a piece of
        // `Legislative Budget Office | LSC | 2 | Office of Research and Drafting`, in
        // whichever column the piece fell in — text the legislature never wrote.
        let d = doc();
        for p in &d.provisions {
            for pos in &p.positions {
                for c in &pos.cells {
                    assert!(
                        !c.contains("Legislative Budget Office"),
                        "footer leaked into an entry: {c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_provision_nobody_touched_can_be_told_from_one_they_did() {
        let d = doc();
        let p = d.find("EDUCD26").unwrap();
        // The House cell opens `Same as the Executive,` and then amends it. Counting that as
        // untouched would hide the change this whole connector exists to find.
        assert!(!p.positions[0].is_unchanged_after_executive());

        let unchanged = Position {
            page: 1,
            cells: vec![
                "Eliminates the authorization.".into(),
                "Same as the Executive.".into(),
                "(2) Same as the Executive.".into(),
                "Same as the House.".into(),
            ],
        };
        assert!(unchanged.is_unchanged_after_executive());
    }

    #[test]
    fn a_qualified_concurrence_is_a_change_not_a_concurrence() {
        assert!(is_bare_concurrence("Same as the Executive."));
        assert!(is_bare_concurrence("(1) Same as the Executive."));
        assert!(is_bare_concurrence("Same as the Senate"));

        // Occurs 12 times in the real document, and is where the House's school funding
        // changes live.
        assert!(!is_bare_concurrence(
            "Same as the Executive, but makes the following changes:"
        ));
        assert!(!is_bare_concurrence(
            "Same as the Executive, but raises the base cost."
        ));
        // A cross-reference in form only: this is the difference, not the absence of one.
        assert!(!is_bare_concurrence("No provision."));
        assert!(!is_bare_concurrence("No provision (see OBMCD51)."));
    }

    #[test]
    fn a_document_that_draws_no_rules_is_refused_by_name() {
        let pages = vec![(
            1u32,
            glyphs(500.0, 10.0, "Running prose, no table", FS_BODY),
        )];
        let e = assemble(&pages, &RuleOptions::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("not a comparison document"), "{e}");
    }

    #[test]
    fn a_document_with_no_stage_headings_is_refused_rather_than_numbered() {
        let mut p = row(
            548.0,
            ["Column A", "Column B", "Column C", "Column D"],
            FS_BODY,
        );
        p.extend(row(502.0, ["EDUCD26 A provision", "", "", ""], FS_HEAD));
        p.extend(row(
            465.0,
            ["Does a thing.", "Same.", "Same.", "Same."],
            FS_BODY,
        ));
        p.extend(rules(459.0));
        let e = assemble(&[(1, p)], &RuleOptions::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("anonymous"), "{e}");
    }
}
