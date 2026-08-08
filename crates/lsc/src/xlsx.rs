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
use calamine::{open_workbook_auto, Data, Reader};

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

/// Reads one worksheet into a table, taking the first non-empty row as headers.
pub fn sheet_to_table(path: &std::path::Path, sheet: &str) -> Result<RawTable> {
    let mut wb = open_workbook_auto(path).with_context(|| format!("opening {}", path.display()))?;
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
    let wb = open_workbook_auto(path).with_context(|| format!("opening {}", path.display()))?;
    Ok(wb.sheet_names().to_vec())
}
