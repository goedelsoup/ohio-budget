//! Reconstructs tables that draw their own column separators.
//!
//! [`crate::geometry`] finds columns as **rivers** — vertical bands where no glyph appears.
//! That is the right algorithm when a document leaves its columns implicit, and it is what the
//! appropriation spreadsheets need. On a comparison document it does something worse than
//! fail, and the measurements are what show it. At a 12pt body font, across the first forty
//! pages of HB 96's education comparison document:
//!
//! | boundary | left column ends | right column starts | gutter |
//! |---|---|---|---|
//! | Executive → As Passed By House | 251.62 | 252.06 | **0.44** |
//! | As Passed By House → As Passed By Senate | 494.96 | 513.00 | 18.04 |
//! | As Passed By Senate → As Enacted | 737.51 | 762.48 | 24.97 |
//!
//! Only the first boundary is flush, and that asymmetry is the trap. River detection finds
//! the two wide gutters and misses the narrow one, so it returns a table of the wrong shape
//! that still looks entirely well-formed — three columns instead of four, with the executive
//! proposal and the House's position fused into a single cell. No threshold fixes it: 0.44
//! points is narrower than a space, and coming down far enough to split it shatters the
//! columns that were working.
//!
//! But the document is not withholding its structure. It *draws* the column separators, as
//! `|` glyphs in the text layer, on their own baseline under the first line of every entry.
//! In that document they sit at exactly x = 251.9, 499.4, and 746.9 on all 781 entries across
//! all 212 pages, without a single deviation.
//!
//! So this module does not infer columns at all. It reads the boundaries the typesetter
//! recorded. Where those exist they beat inference outright: they are exact, alignment-agnostic
//! for free, and they survive a column that is empty for a whole page — the case where a river
//! merges two columns into one and nothing downstream can tell.
//!
//! # The rules also delimit entries
//!
//! A second fact falls out of the same glyphs. The rule baseline appears under the *first*
//! line of each comparison entry and nowhere else, so it marks where one entry ends and the
//! next begins. Without it, entry boundaries would have to be guessed from vertical spacing,
//! which is exactly the kind of heuristic that works on the page you tested and quietly
//! mis-segments the one you did not.

use anyhow::{bail, Result};

use crate::geometry::{cell_text, group_rows, median_font_size, Glyph};

/// The glyph a document uses to draw a column separator.
pub const RULE_GLYPH: &str = "|";

#[derive(Debug, Clone)]
pub struct RuleOptions {
    /// Baselines within this multiple of font size are one row.
    pub row_tolerance_ratio: f64,
    /// Rule glyphs within this many points of each other are the same column boundary.
    ///
    /// Deliberately tight. These are drawn at an identical coordinate on every row, so any
    /// real spread means two separate marks — and on this document a footer glyph sits 2.1
    /// points from a genuine rule, which a loose tolerance would swallow.
    pub x_tolerance: f64,
    /// A gap wider than this multiple of font size inserts a space inside a cell.
    pub space_ratio: f64,
}

impl Default for RuleOptions {
    fn default() -> Self {
        Self {
            row_tolerance_ratio: 0.4,
            x_tolerance: 1.0,
            space_ratio: 0.25,
        }
    }
}

/// One line of text, split into cells at the drawn column boundaries.
#[derive(Debug, Clone, PartialEq)]
pub struct RuledLine {
    pub y: f64,
    pub cells: Vec<String>,
    /// A rule was drawn immediately below this line — the document's own mark that a new
    /// comparison entry starts here.
    pub starts_entry: bool,
    /// Median font size of the line's glyphs.
    ///
    /// Carried because these documents set structural matter — section headings, provision
    /// headings, statutory citations — one point smaller than body prose, which is the only
    /// signal available for telling a heading from the paragraph it interrupts. Font *weight*
    /// would be the natural discriminator and is not recoverable from the text layer.
    pub font_size: f64,
    /// The whole line read straight across, ignoring the column boundaries.
    ///
    /// Headings span the table rather than sitting in a column, and at the flush boundary a
    /// heading's own word is cut in two — `EDUCD26 Traditional school district funding form`
    /// ends one cell and `ula` begins the next. Rejoining the cells cannot recover that,
    /// because whether to insert a space depends on which boundary was crossed, and a heading
    /// does not know. Reading the glyphs across gets it right without the question arising.
    pub text: String,
}

impl RuledLine {
    pub fn is_blank(&self) -> bool {
        self.cells.iter().all(|c| c.trim().is_empty())
    }
}

/// A page reconstructed from its drawn rules.
#[derive(Debug, Clone, PartialEq)]
pub struct RuledTable {
    /// x of each drawn column boundary, ascending. One fewer than the column count.
    pub rules: Vec<f64>,
    pub lines: Vec<RuledLine>,
}

impl RuledTable {
    pub fn column_count(&self) -> usize {
        self.rules.len() + 1
    }
}

/// Does any content glyph share this baseline?
///
/// This is what separates a drawn rule from an incidental `|`. Every page of these documents
/// ends with `Legislative Budget OfficeLSC|1Office of Research and Drafting`, and that pipe
/// lands 2.1 points from a genuine column boundary — close enough that admitting it would
/// shift a boundary and hand a column of prose to the wrong legislative stage.
///
/// Counting pipes would not catch it: the footer carries one, and so would every rule row in
/// a two-column table. What actually distinguishes them is that a drawn rule has a baseline
/// to itself and the footer glyph does not.
fn shares_baseline_with_content(y: f64, content: &[Glyph], tolerance: f64) -> bool {
    content.iter().any(|g| (g.y - y).abs() <= tolerance)
}

/// Clusters rule x positions into distinct column boundaries.
fn cluster(mut xs: Vec<f64>, tolerance: f64) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut out: Vec<Vec<f64>> = vec![vec![xs[0]]];
    for x in xs.into_iter().skip(1) {
        let last = out.last_mut().expect("seeded above");
        // Compare against the cluster's running mean rather than its first member, so a
        // column whose glyphs drift by a fraction of a point does not split in two.
        let mean = last.iter().sum::<f64>() / last.len() as f64;
        if (x - mean).abs() <= tolerance {
            last.push(x);
        } else {
            out.push(vec![x]);
        }
    }
    out.into_iter()
        .map(|c| c.iter().sum::<f64>() / c.len() as f64)
        .collect()
}

/// Which column a glyph falls in, by its centre against the drawn boundaries.
fn column_of(centre: f64, rules: &[f64]) -> usize {
    rules.iter().take_while(|r| centre >= **r).count()
}

/// Reconstructs a page from the column rules it draws for itself.
///
/// Fails rather than falling back to river detection. A comparison document that has stopped
/// drawing its rules is a different document than the one this was written against, and
/// guessing at its columns would produce prose attributed to the wrong legislative stage —
/// an error that reads as plausible and has no downstream check.
pub fn to_ruled_table(glyphs: &[Glyph], opts: &RuleOptions) -> Result<RuledTable> {
    if glyphs.is_empty() {
        bail!("no glyphs: the page yielded no text, which usually means it is a scanned image");
    }
    let fs = median_font_size(glyphs);
    let tol = fs * opts.row_tolerance_ratio;

    // Rule glyphs are separated before rows are grouped, not after. On this document the rule
    // baseline sits 5.5 points under the line it belongs to and the font is 11pt, which puts
    // it right at the row-grouping tolerance — so whether a rule merged into its content row
    // would depend on a rounding decision. Partitioning first removes the interaction.
    let (rule_glyphs, content): (Vec<Glyph>, Vec<Glyph>) = glyphs
        .iter()
        .cloned()
        .partition(|g| g.text.trim() == RULE_GLYPH);

    // A rule row is one the typesetter gave a baseline of its own. Where the tolerance is too
    // generous this test rejects genuine rules and the page is refused by name below, which is
    // the direction to fail in: a rule wrongly discarded stops the extraction, a footer glyph
    // wrongly admitted moves a column boundary and is never seen again.
    let rule_rows: Vec<crate::geometry::Row> = group_rows(&rule_glyphs, tol)
        .into_iter()
        .filter(|r| !shares_baseline_with_content(r.y, &content, tol))
        .collect();

    let rules = cluster(
        rule_rows
            .iter()
            .flat_map(|r| r.glyphs.iter().map(|g| g.x))
            .collect(),
        opts.x_tolerance,
    );
    if rules.is_empty() {
        bail!(
            "no column rules drawn on this page: found {} rule glyph(s) but none on a row of \
             their own, so the page's column boundaries are not recorded in it",
            rule_glyphs.len()
        );
    }

    split_at_rules(&content, &rule_rows, rules, fs, opts)
}

/// Reconstructs a page using boundaries taken from elsewhere.
///
/// Needed for continuation pages. An entry that overruns its page carries on at the top of
/// the next one, and because no new entry begins there the typesetter draws no rule — so the
/// page records no boundaries of its own despite being laid out on them. Five of this
/// document's 212 pages are like this. Refusing them would discard the tail of five entries,
/// including a fiscal-effect paragraph.
pub fn to_ruled_table_with_rules(
    glyphs: &[Glyph],
    rules: &[f64],
    opts: &RuleOptions,
) -> Result<RuledTable> {
    if glyphs.is_empty() {
        bail!("no glyphs: the page yielded no text, which usually means it is a scanned image");
    }
    let fs = median_font_size(glyphs);
    let content: Vec<Glyph> = glyphs
        .iter()
        .filter(|g| g.text.trim() != RULE_GLYPH)
        .cloned()
        .collect();
    split_at_rules(&content, &[], rules.to_vec(), fs, opts)
}

fn split_at_rules(
    content: &[Glyph],
    rule_rows: &[crate::geometry::Row],
    rules: Vec<f64>,
    fs: f64,
    opts: &RuleOptions,
) -> Result<RuledTable> {
    let content_rows = group_rows(content, fs * opts.row_tolerance_ratio);
    if content_rows.is_empty() {
        bail!(
            "page draws {} column rule(s) but holds no text",
            rules.len()
        );
    }

    // A rule marks the line above it. Rows run top-down (descending y), so that is the last
    // content row whose baseline is still above the rule.
    let mut starts = vec![false; content_rows.len()];
    for rr in rule_rows {
        if let Some(i) = content_rows.iter().rposition(|c| c.y > rr.y) {
            starts[i] = true;
        }
    }

    let space_gap = fs * opts.space_ratio;
    let n = rules.len() + 1;
    let lines = content_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut cells: Vec<Vec<&Glyph>> = vec![Vec::new(); n];
            let mut across: Vec<&Glyph> = Vec::new();
            for g in &row.glyphs {
                cells[column_of((g.x + g.right()) / 2.0, &rules)].push(g);
                across.push(g);
            }
            let sized: Vec<Glyph> = row
                .glyphs
                .iter()
                .filter(|g| !g.text.trim().is_empty())
                .cloned()
                .collect();
            RuledLine {
                y: row.y,
                cells: cells.iter().map(|c| cell_text(c, space_gap)).collect(),
                starts_entry: starts[i],
                font_size: median_font_size(&sized),
                text: cell_text(&across, space_gap),
            }
        })
        .collect();

    Ok(RuledTable { rules, lines })
}

// ─── running heads and feet ──────────────────────────────────────────────────

/// Page furniture: lines the typesetter repeats on every page.
///
/// These have to be removed, and the reason is not tidiness. The running foot of these
/// documents is `Legislative Budget Office | LSC | <page> | Office of Research and Drafting`,
/// which sits below the last entry on the page and therefore joins it — every page's final
/// entry ends with a fragment of the footer, in the column the fragment happened to fall in.
/// That is 212 of 781 entries on this document, and the corruption looks like text the
/// legislature wrote.
///
/// The detector is a repetition test rather than a margin heuristic. "Below y=50 is a footer"
/// works on the pages you check and silently eats a provision on a page that runs long;
/// "appears at the same position on most pages" is a property of what a running element *is*.
#[derive(Debug, Clone, Default)]
pub struct RunningLines {
    keys: std::collections::HashSet<(i64, String)>,
}

/// Digits collapse to `#` so a page number does not make each footer unique.
fn running_key(line: &RuledLine) -> (i64, String) {
    let text: String = line
        .cells
        .join("\u{1}")
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect();
    (line.y.round() as i64, text.split_whitespace().collect())
}

impl RunningLines {
    pub fn contains(&self, line: &RuledLine) -> bool {
        self.keys.contains(&running_key(line))
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Finds lines repeated at the same position on at least `min_pages` pages.
///
/// Keying on position *and* text together is what makes this safe. Text alone would catch
/// `Same as the Executive.`, which is the single most common line in the document and is
/// exactly the content being extracted; position alone would catch whatever happened to land
/// in the same place twice. A line has to be both to count.
pub fn detect_running_lines(pages: &[RuledTable], min_pages: usize) -> RunningLines {
    let mut counts: std::collections::HashMap<(i64, String), usize> = Default::default();
    for page in pages {
        // Once per page: a line repeated within one page is content, not furniture.
        let mut seen = std::collections::HashSet::new();
        for line in page.lines.iter().filter(|l| !l.is_blank()) {
            let k = running_key(line);
            if seen.insert(k.clone()) {
                *counts.entry(k).or_default() += 1;
            }
        }
    }
    RunningLines {
        keys: counts
            .into_iter()
            .filter(|(_, n)| *n >= min_pages)
            .map(|(k, _)| k)
            .collect(),
    }
}

/// Default threshold: over half the pages, and never fewer than two.
///
/// The floor is what stops a short document from eating itself. On a two-page extract "over
/// half" is one page, and a threshold of one makes every line furniture — the whole document
/// would come back empty. Two is the least that can mean "repeated" at all.
pub fn default_running_threshold(page_count: usize) -> usize {
    (page_count / 2).max(2)
}

impl RuledTable {
    /// Drops the running head and foot.
    pub fn without_running(&self, running: &RunningLines) -> RuledTable {
        RuledTable {
            rules: self.rules.clone(),
            lines: self
                .lines
                .iter()
                .filter(|l| !running.contains(l))
                .cloned()
                .collect(),
        }
    }
}

/// One comparison entry: what each column says about a single provision.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Baseline of the entry's first line, for provenance.
    pub y: f64,
    /// One joined string per column, in document order.
    pub cells: Vec<String>,
}

/// Joins an entry's wrapped lines back into prose, one string per column.
///
/// A trailing hyphen is kept and the join is made without a space, because these documents
/// wrap at hyphens that are already in the word — `career-technical` breaks after `career-`.
/// Keeping the hyphen is the conservative choice in both directions: where the break is at a
/// real hyphen it reconstructs the term exactly, and where it is not, the artefact is visible
/// to a reader instead of silently fusing two words into one that was never written.
fn join_wrapped(parts: &[String]) -> String {
    let mut out = String::new();
    for part in parts.iter().filter(|p| !p.trim().is_empty()) {
        if !out.is_empty() && !out.ends_with('-') {
            out.push(' ');
        }
        out.push_str(part.trim());
    }
    out
}

/// Segments a reconstructed page into comparison entries.
///
/// Everything above the first drawn rule is preamble — the running title, the column headers,
/// and the section and provision headings — and is returned separately rather than folded
/// into the first entry, where it would appear as text the legislature never wrote.
pub fn group_entries(table: &RuledTable) -> (Vec<RuledLine>, Vec<Entry>) {
    let first = table.lines.iter().position(|l| l.starts_entry);
    let Some(first) = first else {
        return (table.lines.clone(), Vec::new());
    };

    let preamble: Vec<RuledLine> = table.lines[..first]
        .iter()
        .filter(|l| !l.is_blank())
        .cloned()
        .collect();

    let mut entries: Vec<Entry> = Vec::new();
    let mut buf: Vec<Vec<String>> = Vec::new();
    let mut y = 0.0;

    let flush = |entries: &mut Vec<Entry>, buf: &mut Vec<Vec<String>>, y: f64| {
        if buf.is_empty() {
            return;
        }
        let n = buf[0].len();
        let cells = (0..n)
            .map(|c| join_wrapped(&buf.iter().map(|r| r[c].clone()).collect::<Vec<_>>()))
            .collect();
        entries.push(Entry { y, cells });
        buf.clear();
    };

    for line in &table.lines[first..] {
        if line.starts_entry {
            flush(&mut entries, &mut buf, y);
            y = line.y;
        }
        if !line.is_blank() {
            buf.push(line.cells.clone());
        }
    }
    flush(&mut entries, &mut buf, y);

    (preamble, entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f64 = 10.0;

    fn text_at(y: f64, x0: f64, s: &str) -> Vec<Glyph> {
        let adv = FS * 0.5;
        s.chars()
            .enumerate()
            .map(|(i, c)| Glyph::new(x0 + i as f64 * adv, y, adv, FS, &c.to_string()))
            .collect()
    }

    fn rule_at(y: f64, xs: &[f64]) -> Vec<Glyph> {
        xs.iter()
            .map(|x| Glyph::new(*x, y, FS * 0.3, FS, RULE_GLYPH))
            .collect()
    }

    /// A page laid out with the gutter widths measured off the real document.
    ///
    /// Measured across the first forty pages of HB 96's education comparison document, at a
    /// 12pt body font:
    ///
    /// | boundary | left column ends | right column starts | gutter |
    /// |---|---|---|---|
    /// | Executive → As Passed By House | 251.62 | 252.06 | **0.44** |
    /// | As Passed By House → As Passed By Senate | 494.96 | 513.00 | 18.04 |
    /// | As Passed By Senate → As Enacted | 737.51 | 762.48 | 24.97 |
    ///
    /// Only the first is flush. That asymmetry is the whole problem, and it is why river
    /// detection is not merely imperfect here but actively misleading — see the test below.
    /// Scaled to the 10pt font these fixtures use: a 0.6 gutter at the first boundary and
    /// 21.4 at the second.
    const RULE_A: f64 = 120.2;
    const RULE_B: f64 = 250.0;
    const COL1: f64 = 120.6;
    const COL2: f64 = 252.0;

    fn comparison_page() -> Vec<Glyph> {
        let mut g = Vec::new();
        g.extend(text_at(200.0, 0.0, "Executive"));
        g.extend(text_at(200.0, COL1, "As Passed By House"));
        g.extend(text_at(200.0, COL2, "As Enacted"));

        g.extend(text_at(180.0, 0.0, "Eliminates the express"));
        g.extend(text_at(180.0, COL1, "Same as the Executive."));
        g.extend(text_at(180.0, COL2, "Same as the House."));
        g.extend(rule_at(175.0, &[RULE_A, RULE_B]));
        g.extend(text_at(170.0, 0.0, "authorization."));

        g.extend(text_at(150.0, 0.0, "Permits districts to use"));
        g.extend(text_at(150.0, COL1, "No provision."));
        g.extend(text_at(150.0, COL2, "Same as the House."));
        g.extend(rule_at(145.0, &[RULE_A, RULE_B]));
        g.extend(text_at(140.0, 0.0, "the funds."));
        g
    }

    #[test]
    fn columns_come_from_the_drawn_rule_not_from_whitespace() {
        let t = to_ruled_table(&comparison_page(), &RuleOptions::default()).unwrap();
        assert_eq!(t.column_count(), 3);
        assert!((t.rules[0] - RULE_A).abs() < 0.01, "{:?}", t.rules);
        assert!((t.rules[1] - RULE_B).abs() < 0.01, "{:?}", t.rules);
    }

    #[test]
    fn river_detection_loses_a_column_silently_on_the_same_page() {
        // The motivating case, asserted rather than described — and the failure is worse than
        // "no table found". Two of the three boundaries have generous gutters, so rivers are
        // found there and the output *looks* like a well-formed table. Only the flush
        // boundary is missed, which fuses Executive with As Passed By House into one cell.
        //
        // A reader of that output gets a table with the right shape, plausible contents, and
        // one stage's position silently attributed to another. Nothing downstream can detect
        // it. That is why this module refuses to fall back to river detection.
        use crate::geometry::{to_table, TableOptions};
        for ratio in [1.2, 0.5, 0.35, 0.2] {
            let opts = TableOptions {
                min_river_ratio: ratio,
                ..Default::default()
            };
            let merged = to_table(&comparison_page(), &opts).unwrap();
            assert_eq!(
                merged.headers.len(),
                2,
                "river ratio {ratio}: expected the flush boundary to be missed, got {:?}",
                merged.headers
            );
            assert!(
                merged.headers[0].contains("Executive") && merged.headers[0].contains("House"),
                "two stages should have fused into one cell, got {:?}",
                merged.headers[0]
            );
        }
    }

    #[test]
    fn a_rule_marks_the_line_above_it_as_an_entry_start() {
        let t = to_ruled_table(&comparison_page(), &RuleOptions::default()).unwrap();
        let starts: Vec<&RuledLine> = t.lines.iter().filter(|l| l.starts_entry).collect();
        assert_eq!(starts.len(), 2);
        assert_eq!(starts[0].cells[0], "Eliminates the express");
        assert_eq!(starts[1].cells[0], "Permits districts to use");
    }

    #[test]
    fn wrapped_lines_rejoin_within_their_own_column() {
        let t = to_ruled_table(&comparison_page(), &RuleOptions::default()).unwrap();
        let (preamble, entries) = group_entries(&t);
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].cells,
            vec![
                "Eliminates the express authorization.",
                "Same as the Executive.",
                "Same as the House."
            ]
        );
        assert_eq!(
            entries[1].cells,
            vec![
                "Permits districts to use the funds.",
                "No provision.",
                "Same as the House."
            ]
        );
        // The column header line is preamble, not the first entry's text.
        assert_eq!(preamble.len(), 1);
        assert_eq!(
            preamble[0].cells,
            vec!["Executive", "As Passed By House", "As Enacted"]
        );
    }

    #[test]
    fn a_hyphen_break_rejoins_without_inserting_a_space() {
        assert_eq!(
            join_wrapped(&["Requires each career-".into(), "technical district".into()]),
            "Requires each career-technical district"
        );
    }

    #[test]
    fn a_lone_rule_glyph_among_text_is_not_a_column_boundary() {
        // Every page of the real document ends with `Legislative Budget OfficeLSC|1Office of
        // Research and Drafting`. That pipe sits 2.1 points from a genuine rule, so treating
        // it as one would shift a column boundary and mis-attribute prose to a stage.
        let mut g = comparison_page();
        g.extend(text_at(20.0, 0.0, "Legislative Budget OfficeLSC"));
        g.extend(rule_at(20.0, &[122.3]));
        g.extend(text_at(20.0, 126.0, "1Office of Research"));

        let t = to_ruled_table(&g, &RuleOptions::default()).unwrap();
        assert_eq!(t.rules.len(), 2, "footer pipe became a rule: {:?}", t.rules);
        assert!((t.rules[0] - RULE_A).abs() < 0.01, "{:?}", t.rules);
    }

    #[test]
    fn an_empty_column_survives_instead_of_merging_away() {
        // The case river detection cannot get right even in principle: a column with no text
        // on the whole page is indistinguishable from absent whitespace, so a river-based
        // reader returns three columns where the document has four and every cell after the
        // empty one is attributed to the wrong stage.
        let mut g = text_at(200.0, 0.0, "Executive");
        g.extend(text_at(200.0, 200.0, "As Enacted"));
        g.extend(text_at(180.0, 0.0, "Adds a provision."));
        g.extend(text_at(180.0, 200.0, "Same as the Executive."));
        g.extend(rule_at(175.0, &[99.0, 199.0]));

        let t = to_ruled_table(&g, &RuleOptions::default()).unwrap();
        assert_eq!(t.column_count(), 3);
        let (_, entries) = group_entries(&t);
        assert_eq!(entries[0].cells[1], "", "the silent column must stay empty");
        assert_eq!(entries[0].cells[2], "Same as the Executive.");
    }

    #[test]
    fn a_page_that_draws_no_rules_is_refused_rather_than_guessed() {
        let g = text_at(
            200.0,
            0.0,
            "A page of running prose with no table on it at all",
        );
        let e = to_ruled_table(&g, &RuleOptions::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("not recorded"), "{e}");
    }

    #[test]
    fn an_empty_page_names_the_likely_cause() {
        let e = to_ruled_table(&[], &RuleOptions::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("scanned image"), "{e}");
    }

    #[test]
    fn clustering_keeps_marks_that_are_close_but_distinct() {
        assert_eq!(cluster(vec![10.0, 10.2, 10.1], 1.0).len(), 1);
        assert_eq!(cluster(vec![10.0, 12.5], 1.0).len(), 2);
    }
}
