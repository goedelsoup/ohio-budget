//! Reads the appropriation spreadsheet from its published xlsx form.
//!
//! LSC publishes the appropriation spreadsheet as both PDF and xlsx. The xlsx is the better
//! source and this module exists because of a concrete failure: the spreadsheet PDF embeds a
//! subset font with no usable ToUnicode mapping, so 93% of its glyphs decode to empty strings.
//! The geometry recovers the table perfectly and every cell is blank.
//!
//! That is not a defect in [`crate::geometry`] — the per-agency comparison documents, which
//! this connector is named for, decode fine. It is a property of this one document, and the
//! right response is to read the structured sibling rather than to fight the font.
//!
//! Output is a [`RawTable`], so [`crate::map_columns`] and [`crate::normalize`] apply
//! unchanged. Three input formats, one downstream path.

use anyhow::{anyhow, bail, Context, Result};
use calamine::{open_workbook_auto_from_rs, Data, Reader};

use crate::RawTable;

/// Renders a cell as the text the shared pipeline expects.
///
/// Numbers are formatted to exactly two decimal places. Budget magnitudes with cents need
/// about twelve significant digits and an f64 carries fifteen, so this is lossless for the
/// values in question — but rounding happens *here*, at the boundary, rather than silently
/// inside the money parser, which refuses more than two decimal places by design.
fn cell_text(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => format!("{f:.2}"),
        Data::Int(i) => format!("{i}.00"),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DurationIso(s) | Data::DateTimeIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR:{e:?}"),
    }
}

/// Scores a row on how well it works as a header.
///
/// Taking the first non-empty row is wrong on real workbooks: the HB 33 spreadsheet with
/// actual expenditures opens with a title row ("Main Operating Budget as of September 30,
/// 2024") and puts the real headers on the row below. Guessing wrong there shifts every
/// figure by a row and produces a table that parses cleanly and is entirely false.
///
/// So the header row is the one whose cells classify best, which is a property of the
/// content rather than of the position.
fn header_score(row: &[String]) -> usize {
    use crate::columns::{classify_header, ColumnKind};
    row.iter()
        .map(|h| match classify_header(h) {
            ColumnKind::Identity(_) => 3,
            ColumnKind::Appropriation { .. }
            | ColumnKind::Actual { .. }
            | ColumnKind::AdjustedAppropriation { .. } => 2,
            ColumnKind::Estimate { .. } | ColumnKind::UnknownStage { .. } => 1,
            ColumnKind::Other => 0,
        })
        .sum()
}

/// Finds the header row among the first `search_depth` rows.
pub fn find_header_row(rows: &[Vec<String>], search_depth: usize) -> Option<usize> {
    rows.iter()
        .take(search_depth)
        .enumerate()
        .max_by_key(|(i, r)| (header_score(r), std::cmp::Reverse(*i)))
        .filter(|(_, r)| header_score(r) > 0)
        .map(|(i, _)| i)
}

/// Opens a workbook by what it contains rather than by what it is called.
///
/// `open_workbook_auto` dispatches on the file extension, which is right until a publisher is
/// careless. LSC serves the 129th General Assembly's budget workbook at a `.xlsx` URL and the
/// file is a legacy OLE2 `.xls`: the extension picks the zip reader, which reports
/// `Could not find EOCD` — a message about zip central directories that says nothing about the
/// actual problem, on a file that opens fine in a spreadsheet program.
///
/// Reading the bytes and letting calamine sniff them costs one buffer per workbook, which at
/// these sizes is under a megabyte, and removes a whole class of confusing failure.
fn open(path: &std::path::Path) -> Result<calamine::Sheets<std::io::Cursor<Vec<u8>>>> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    open_workbook_auto_from_rs(std::io::Cursor::new(bytes))
        .with_context(|| format!("opening {}", path.display()))
}

/// Reads one worksheet into a table, detecting which row carries the headers.
pub fn sheet_to_table(path: &std::path::Path, sheet: &str) -> Result<RawTable> {
    let mut wb = open(path)?;
    let range = wb
        .worksheet_range(sheet)
        .map_err(|e| anyhow!("worksheet {sheet}: {e}"))?;

    let mut rows = range
        .rows()
        .map(|r| r.iter().map(cell_text).collect::<Vec<_>>());
    let headers = rows
        .by_ref()
        .find(|r: &Vec<String>| r.iter().any(|c| !c.is_empty()))
        .ok_or_else(|| anyhow!("sheet {sheet} has no non-empty row"))?;

    // Trailing all-empty rows are padding, not data, and would otherwise be reported as
    // blank figures for a line item that does not exist.
    let data: Vec<Vec<String>> = rows
        .filter(|r| r.iter().any(|c| !c.is_empty()))
        .map(|mut r| {
            r.resize(headers.len(), String::new());
            r
        })
        .collect();

    if data.is_empty() {
        bail!("sheet {sheet} has headers but no data rows");
    }
    Ok(RawTable {
        headers,
        rows: data,
    })
}

/// Sheet names published in the appropriation spreadsheet.
pub fn sheet_names(path: &std::path::Path) -> Result<Vec<String>> {
    let wb = open(path)?;
    Ok(wb.sheet_names().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_title_row_is_not_mistaken_for_headers() {
        // The HB 33 actuals workbook opens with a title and puts headers on the next row.
        // Getting this wrong shifts every figure by a row and still parses cleanly.
        let rows = vec![
            row(&["Main Operating Budget as of September 30, 2024", "", "", ""]),
            row(&["Agency", "Fund Group", "ALI", "FY 2024"]),
            row(&["EDU", "GRF", "200550", "7975003596.89"]),
        ];
        assert_eq!(find_header_row(&rows, 10), Some(1));
    }

    #[test]
    fn a_header_row_at_the_top_is_still_found() {
        let rows = vec![
            row(&["Agency", "Fund Group", "ALI", "ALI Name", "FY 2024"]),
            row(&["EDU", "GRF", "200550", "Foundation Funding", "1.00"]),
        ];
        assert_eq!(find_header_row(&rows, 10), Some(0));
    }

    #[test]
    fn a_sheet_with_no_recognisable_header_returns_none() {
        let rows = vec![row(&["notes", "", ""]), row(&["free text", "", ""])];
        assert_eq!(find_header_row(&rows, 10), None);
    }

    #[test]
    fn the_earlier_row_wins_a_tie() {
        // Repeated header rows appear where a table continues across a page break.
        let rows = vec![
            row(&["Agency", "ALI", "FY 2024"]),
            row(&["EDU", "200550", "1.00"]),
            row(&["Agency", "ALI", "FY 2024"]),
        ];
        assert_eq!(find_header_row(&rows, 10), Some(0));
    }

    #[test]
    fn a_workbook_is_opened_by_content_not_by_extension() {
        // LSC serves the 129th General Assembly's workbook at a `.xlsx` URL and the file is a
        // legacy OLE2 `.xls`. Dispatching on the extension picks the zip reader, which fails
        // with `Could not find EOCD` — a message about zip central directories, on a file that
        // opens fine in any spreadsheet program. Committed under its true extension; this reads
        // it regardless.
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.yidam/sources/lsc/hb153-budget-in-detail-as-enrolled-129th.xls");
        let names = sheet_names(&p).expect("legacy .xls must open");
        assert!(
            names.iter().any(|n| n == "All Funds without Summary"),
            "got {names:?}"
        );
    }
}
