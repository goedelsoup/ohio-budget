//! Reconstructs table structure from positioned glyphs.
//!
//! A PDF does not contain a table. It contains characters at coordinates, and the columns a
//! reader perceives are whitespace. Recovering cells is therefore a geometry problem, and it
//! is the only genuinely hard part of extracting figures from these documents.
//!
//! Nothing here touches a PDF. Everything takes [`Glyph`] values and is tested with synthetic
//! input, which is why the algorithm can be exercised against the cases that actually break
//! extractors without needing a document that exhibits them.
//!
//! # Why columns are found by whitespace rather than by alignment
//!
//! The obvious approach is to cluster cells by where they start. It fails on exactly the
//! columns that matter here: **currency in budget tables is right-aligned**, so `$1,000.00`
//! and `$812,345,678.00` in the same column begin at very different x positions and end at
//! the same one. Clustering on start splits one column into several; clustering on end breaks
//! the left-aligned text columns instead.
//!
//! So columns are found as vertical **rivers** — bands of x where no glyph appears on any
//! row. That is alignment-agnostic: it finds the gap between columns rather than the edge of
//! any particular one, and handles left, right, and centre alignment without being told which
//! is which.

use anyhow::{bail, Result};

use crate::RawTable;

/// One rendered character with its position on the page.
///
/// `x` is the left edge, `y` the baseline. PDF space puts y increasing upward, so rows run
/// from high y to low.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub x: f64,
    pub y: f64,
    /// Horizontal advance — how far the cursor moves after drawing this glyph.
    pub advance: f64,
    pub font_size: f64,
    pub text: String,
}

impl Glyph {
    pub fn new(x: f64, y: f64, advance: f64, font_size: f64, text: &str) -> Self {
        Self {
            x,
            y,
            advance,
            font_size,
            text: text.to_string(),
        }
    }

    pub fn right(&self) -> f64 {
        self.x + self.advance
    }
}

#[derive(Debug, Clone)]
pub struct TableOptions {
    /// Baselines within this distance are the same row. Expressed as a multiple of font size.
    pub row_tolerance_ratio: f64,
    /// A river must be at least this wide, as a multiple of font size, to separate columns.
    /// Too small and a wide inter-word space splits a column; too large and adjacent columns
    /// merge.
    pub min_river_ratio: f64,
    /// Quantisation of the occupancy scan, as a multiple of font size.
    pub bin_ratio: f64,
    /// A gap wider than this multiple of font size inserts a space inside a cell.
    pub space_ratio: f64,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            row_tolerance_ratio: 0.5,
            min_river_ratio: 1.2,
            bin_ratio: 0.1,
            space_ratio: 0.25,
        }
    }
}

/// A run of glyphs sharing a baseline, ordered left to right.
#[derive(Debug, Clone)]
pub struct Row {
    pub y: f64,
    pub glyphs: Vec<Glyph>,
}

pub(crate) fn median_font_size(glyphs: &[Glyph]) -> f64 {
    if glyphs.is_empty() {
        return 1.0;
    }
    let mut sizes: Vec<f64> = glyphs.iter().map(|g| g.font_size).collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let m = sizes[sizes.len() / 2];
    if m > 0.0 {
        m
    } else {
        1.0
    }
}

/// Groups glyphs into rows by baseline.
pub fn group_rows(glyphs: &[Glyph], tolerance: f64) -> Vec<Row> {
    if glyphs.is_empty() {
        return Vec::new();
    }
    let mut sorted: Vec<Glyph> = glyphs.to_vec();
    // Descending y: PDF space runs bottom-up, reading order runs top-down.
    sorted.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap()
            .then(a.x.partial_cmp(&b.x).unwrap())
    });

    let mut rows: Vec<Row> = Vec::new();
    for g in sorted {
        match rows.last_mut() {
            Some(row) if (row.y - g.y).abs() <= tolerance => row.glyphs.push(g),
            _ => rows.push(Row {
                y: g.y,
                glyphs: vec![g],
            }),
        }
    }
    for row in &mut rows {
        row.glyphs.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    }
    rows
}

/// x ranges occupied by no glyph on any row, wide enough to separate columns.
///
/// This is the alignment-agnostic step: it looks for the absence of text rather than for the
/// edge of any column.
pub fn find_rivers(rows: &[Row], bin: f64, min_width: f64) -> Vec<(f64, f64)> {
    let all: Vec<&Glyph> = rows.iter().flat_map(|r| r.glyphs.iter()).collect();
    if all.is_empty() {
        return Vec::new();
    }
    let min_x = all.iter().map(|g| g.x).fold(f64::INFINITY, f64::min);
    let max_x = all
        .iter()
        .map(|g| g.right())
        .fold(f64::NEG_INFINITY, f64::max);
    if max_x <= min_x {
        return Vec::new();
    }

    let n = (((max_x - min_x) / bin).ceil() as usize).max(1);
    let mut occupied = vec![false; n];
    for g in &all {
        let start = (((g.x - min_x) / bin).floor() as usize).min(n - 1);
        let end = (((g.right() - min_x) / bin).ceil() as usize).min(n);
        for slot in occupied.iter_mut().take(end).skip(start) {
            *slot = true;
        }
    }

    let mut rivers = Vec::new();
    let mut run_start: Option<usize> = None;
    // One past the end, so a run reaching the final bin is closed rather than left open.
    for (i, free) in occupied
        .iter()
        .map(|o| !*o)
        .chain(std::iter::once(false))
        .enumerate()
    {
        match (free, run_start) {
            (true, None) => run_start = Some(i),
            (false, Some(s)) => {
                let width = (i - s) as f64 * bin;
                // Runs touching the page edges are margins, not separators.
                if width >= min_width && s > 0 && i < n {
                    rivers.push((min_x + s as f64 * bin, min_x + i as f64 * bin));
                }
                run_start = None;
            }
            _ => {}
        }
    }
    rivers
}

/// Column x ranges, derived from the rivers between them.
pub fn columns_from_rivers(rows: &[Row], rivers: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let all: Vec<&Glyph> = rows.iter().flat_map(|r| r.glyphs.iter()).collect();
    if all.is_empty() {
        return Vec::new();
    }
    let min_x = all.iter().map(|g| g.x).fold(f64::INFINITY, f64::min);
    let max_x = all
        .iter()
        .map(|g| g.right())
        .fold(f64::NEG_INFINITY, f64::max);

    let mut cols = Vec::new();
    let mut start = min_x;
    for (rs, re) in rivers {
        cols.push((start, *rs));
        start = *re;
    }
    cols.push((start, max_x));
    cols
}

pub(crate) fn cell_text(glyphs: &[&Glyph], space_gap: f64) -> String {
    let mut s = String::new();
    let mut prev_right: Option<f64> = None;
    for g in glyphs {
        if let Some(pr) = prev_right {
            if g.x - pr > space_gap && !s.ends_with(' ') {
                s.push(' ');
            }
        }
        s.push_str(&g.text);
        prev_right = Some(g.right());
    }
    s.trim().to_string()
}

/// Turns positioned glyphs into a table, taking the first row as headers.
///
/// The result feeds [`crate::map_columns`] and [`crate::normalize`] unchanged, so a PDF and a
/// hand-written delimited file follow exactly the same path from here on.
pub fn to_table(glyphs: &[Glyph], opts: &TableOptions) -> Result<RawTable> {
    if glyphs.is_empty() {
        bail!("no glyphs: the page yielded no text, which usually means it is a scanned image");
    }
    let fs = median_font_size(glyphs);
    let rows = group_rows(glyphs, fs * opts.row_tolerance_ratio);
    if rows.len() < 2 {
        bail!(
            "only {} row(s) of text found; a comparison table needs a header and at least one \
             data row",
            rows.len()
        );
    }

    let rivers = find_rivers(&rows, fs * opts.bin_ratio, fs * opts.min_river_ratio);
    let cols = columns_from_rivers(&rows, &rivers);
    let space_gap = fs * opts.space_ratio;

    let mut out: Vec<Vec<String>> = Vec::new();
    for row in &rows {
        let mut cells = vec![Vec::new(); cols.len()];
        for g in &row.glyphs {
            let centre = (g.x + g.right()) / 2.0;
            // Nearest column by centre, so a glyph straddling a boundary lands in one cell
            // rather than being dropped.
            let idx = cols
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let da = if centre < a.0 {
                        a.0 - centre
                    } else if centre > a.1 {
                        centre - a.1
                    } else {
                        0.0
                    };
                    let db = if centre < b.0 {
                        b.0 - centre
                    } else if centre > b.1 {
                        centre - b.1
                    } else {
                        0.0
                    };
                    da.partial_cmp(&db).unwrap()
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            cells[idx].push(g);
        }
        out.push(
            cells
                .iter()
                .map(|c| cell_text(c, space_gap))
                .collect::<Vec<_>>(),
        );
    }

    let headers = out.remove(0);
    Ok(RawTable { headers, rows: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lays out a row of cells at fixed x offsets, one glyph per character.
    fn row_at(y: f64, cells: &[(f64, &str)], fs: f64) -> Vec<Glyph> {
        let adv = fs * 0.5;
        let mut out = Vec::new();
        for (x0, text) in cells {
            for (i, ch) in text.chars().enumerate() {
                out.push(Glyph::new(x0 + i as f64 * adv, y, adv, fs, &ch.to_string()));
            }
        }
        out
    }

    /// Right-aligns text so it ends at `x_end` — how currency appears in these documents.
    fn right_aligned(x_end: f64, text: &str, fs: f64) -> (f64, &str) {
        let adv = fs * 0.5;
        (x_end - text.chars().count() as f64 * adv, text)
    }

    #[test]
    fn rows_group_by_baseline_and_read_top_down() {
        let fs = 10.0;
        let mut g = row_at(700.0, &[(0.0, "b")], fs);
        g.extend(row_at(720.0, &[(0.0, "a")], fs));
        let rows = group_rows(&g, fs * 0.5);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].glyphs[0].text, "a", "higher y is the earlier row");
    }

    #[test]
    fn baselines_within_tolerance_are_one_row() {
        let fs = 10.0;
        let mut g = row_at(700.0, &[(0.0, "a")], fs);
        // Sub-pixel baseline drift is common and must not split a row.
        g.extend(row_at(700.3, &[(50.0, "b")], fs));
        assert_eq!(group_rows(&g, fs * 0.5).len(), 1);
    }

    #[test]
    fn right_aligned_currency_stays_one_column() {
        // The case that defeats start-position clustering: two amounts of very different
        // width in the same column, ending at the same x.
        let fs = 10.0;
        let mut g = row_at(700.0, &[(0.0, "Line Item"), (200.0, "FY2026")], fs);
        g.extend(row_at(
            680.0,
            &[
                (0.0, "Foundation"),
                right_aligned(260.0, "8,000,000.00", fs),
            ],
            fs,
        ));
        g.extend(row_at(
            660.0,
            &[(0.0, "Small"), right_aligned(260.0, "12.00", fs)],
            fs,
        ));

        let table = to_table(&g, &TableOptions::default()).unwrap();
        assert_eq!(table.headers.len(), 2, "headers: {:?}", table.headers);
        assert_eq!(table.rows.len(), 2);
        for row in &table.rows {
            assert_eq!(
                row.len(),
                2,
                "row split into the wrong number of cells: {row:?}"
            );
        }
        assert_eq!(table.rows[0][1], "8,000,000.00");
        assert_eq!(table.rows[1][1], "12.00");
        // Both amounts landed in the same column despite starting 100 units apart.
    }

    #[test]
    fn wide_intra_cell_spacing_does_not_split_a_column() {
        let fs = 10.0;
        // "Line Item Name" has spaces in it; the river threshold must not treat them as gaps.
        let mut g = row_at(700.0, &[(0.0, "Line Item Name"), (300.0, "FY2026")], fs);
        g.extend(row_at(
            680.0,
            &[(0.0, "Foundation Funding"), (300.0, "100.00")],
            fs,
        ));
        let table = to_table(&g, &TableOptions::default()).unwrap();
        assert_eq!(table.headers, vec!["Line Item Name", "FY2026"]);
        assert_eq!(table.rows[0], vec!["Foundation Funding", "100.00"]);
    }

    #[test]
    fn a_blank_cell_is_preserved_as_empty() {
        let fs = 10.0;
        let mut g = row_at(
            700.0,
            &[(0.0, "Item"), (200.0, "FY2026"), (300.0, "FY2027")],
            fs,
        );
        // Second data row reports no FY2027 figure. Absent is not zero.
        g.extend(row_at(
            680.0,
            &[(0.0, "A"), (200.0, "1.00"), (300.0, "2.00")],
            fs,
        ));
        g.extend(row_at(660.0, &[(0.0, "B"), (200.0, "3.00")], fs));
        let table = to_table(&g, &TableOptions::default()).unwrap();
        assert_eq!(table.rows[1].len(), 3);
        assert_eq!(
            table.rows[1][2], "",
            "the missing figure must survive as a blank cell"
        );
    }

    #[test]
    fn output_feeds_the_existing_column_mapper() {
        // The whole point of returning RawTable: a PDF and a delimited file take the same
        // path from here on.
        let fs = 10.0;
        let mut g = row_at(
            700.0,
            &[
                (0.0, "Agency Code"),
                (150.0, "ALI"),
                (250.0, "Line Item Name"),
                (450.0, "FY 2026"),
            ],
            fs,
        );
        g.extend(row_at(
            680.0,
            &[
                (0.0, "200"),
                (150.0, "200550"),
                (250.0, "Foundation Funding"),
                right_aligned(520.0, "8,000,000.00", fs),
            ],
            fs,
        ));
        let table = to_table(&g, &TableOptions::default()).unwrap();
        let map = crate::map_columns(&table.headers).unwrap();
        assert_eq!(map.fiscal_years.len(), 1);
        assert_eq!(map.fiscal_years[0].fiscal_year, "FY2026");
        assert_eq!(table.rows[0][map.line_item_code], "200550");
        assert_eq!(
            crate::parse_money_to_cents(&table.rows[0][map.fiscal_years[0].index]).unwrap(),
            800_000_000
        );
    }

    #[test]
    fn an_empty_page_is_an_error_naming_the_likely_cause() {
        let e = to_table(&[], &TableOptions::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("scanned image"), "{e}");
    }

    #[test]
    fn a_single_line_of_text_is_not_a_table() {
        let g = row_at(700.0, &[(0.0, "just a heading")], 10.0);
        assert!(to_table(&g, &TableOptions::default()).is_err());
    }
}
