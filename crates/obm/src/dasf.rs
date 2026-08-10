//! Detailed Appropriation Summary by Fund — OBM's year-end budgetary report, per OAKS.
//!
//! One row per (agency, fund, appropriation line item), with four money columns:
//!
//! ```text
//! AGENCY NAME              FUND  ALI     NAME              ORIGINAL     FINAL    ACTUAL  VARIANCE
//! DEPARTMENT OF TAXATION   7069  110969  LOCAL GOVERNMENT  424,900,000.00  451,559,619.78  451,474,951.44  84,668.34
//! ```
//!
//! Everything here is pure over [`RawTable`], so it is tested without a document and stays
//! available under `--no-default-features`. Turning a PDF into a `RawTable` lives in [`super`]
//! behind the `pdf` feature — and currently does not work on this publisher's files. See
//! [`verify_decode`].

use anyhow::{bail, Result};
use corpus_schema::{Cents, ObmBudgetaryRow, Provenance};
use lsc::RawTable;

/// What one page yielded, with the rows that could not be read kept rather than dropped.
#[derive(Debug, Default)]
pub struct PageReport {
    pub rows: Vec<ObmBudgetaryRow>,
    /// Rows that carried four money cells but failed to reconcile or parse, with the reason.
    pub problems: Vec<String>,
    /// Rows carrying no money at all — section headings and continuation text.
    pub skipped: usize,
}

/// Digits and punctuation a financial table cannot be missing.
///
/// `pdf-extract` decodes this publisher's files into text that has lost every `.` and every
/// digit from 4 to 9, while recovering 0 to 3 and the thousands separators intact. The result
/// still looks like a number: `4,000.00` arrives as `,00000`. Nothing downstream can detect
/// that, which is why the check belongs at the door rather than in the parser.
const REQUIRED_GLYPHS: [char; 7] = ['.', '4', '5', '6', '7', '8', '9'];

/// Refuses a page whose text cannot be what the document contains.
///
/// A page of a budgetary report states money. Money has decimal points, and across dozens of
/// figures it has every digit. A page missing an entire digit did not decode, and reading it
/// would produce figures that are wrong by orders of magnitude while remaining well-formed.
///
/// This is the [`lsc`] rule applied one step earlier: a misaligned row is an error rather than
/// a truncation, and a mis-decoded page is an error rather than a number.
pub fn verify_decode(page_text: &str) -> Result<()> {
    let missing: Vec<char> = REQUIRED_GLYPHS
        .into_iter()
        .filter(|c| !page_text.contains(*c))
        .collect();
    if !missing.is_empty() {
        bail!(
            "page decoded without {missing:?}, which a table of money cannot be missing; \
             the text extractor failed on this document's fonts rather than the page being \
             empty — figures read from it would be silently wrong, not obviously wrong"
        );
    }
    Ok(())
}

/// Parses `1,234.56`, `-1,234.56`, and `(1,234.56)` into cents.
///
/// Exact by construction: the fractional part is taken as written rather than through a float,
/// because a binary float cannot represent a cent and the error compounds across a sum.
pub fn parse_cents(s: &str) -> Result<Cents> {
    let t = s.trim();
    let (t, negative) = match t.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
        Some(inner) => (inner, true),
        None => match t.strip_prefix('-') {
            Some(inner) => (inner, true),
            None => (t, false),
        },
    };
    let t = t.trim().replace([',', '$'], "");
    let (whole, frac) = match t.split_once('.') {
        Some((w, f)) => (w, f),
        None => bail!("{s:?} has no decimal point; a figure in this report always states cents"),
    };
    if frac.len() != 2 || !frac.chars().all(|c| c.is_ascii_digit()) {
        bail!("{s:?} does not end in exactly two decimal digits");
    }
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        bail!("{s:?} has no whole part");
    }
    let frac: Cents = frac.parse()?;
    let cents: Cents = whole
        .parse::<Cents>()?
        .checked_mul(100)
        .and_then(|c| c.checked_add(frac))
        .ok_or_else(|| anyhow::anyhow!("{s:?} overflows"))?;
    Ok(if negative { -cents } else { cents })
}

fn looks_like_money(s: &str) -> bool {
    parse_cents(s).is_ok()
}

/// An appropriation line item code as this report writes it: six characters, digits with an
/// occasional letter (`200550`, `2006A2`).
fn looks_like_ali(s: &str) -> bool {
    let t = s.trim();
    t.len() == 6
        && t.chars().all(|c| c.is_ascii_alphanumeric())
        && t.chars().any(|c| c.is_ascii_digit())
}

/// Reads one row of cells, or explains why it is not a data row.
///
/// Returns `Ok(None)` for section headings and continuation lines, which carry no money and are
/// not defects. Returns `Err` only where a row looks like data and then fails to reconcile —
/// which is the case worth surfacing, because it means the columns were cut in the wrong place.
pub fn row_from_cells(
    cells: &[String],
    fiscal_year: &str,
    provenance: &Provenance,
) -> Result<Option<ObmBudgetaryRow>> {
    // The four money columns are the trailing cells. Anchoring on the end rather than on a
    // fixed column index survives the agency and line item names, which contain spaces and
    // may or may not be split into separate cells by the river scan.
    let money_start = match cells.len().checked_sub(4) {
        Some(i) if cells[i..].iter().all(|c| looks_like_money(c)) => i,
        _ => return Ok(None),
    };
    let original = parse_cents(&cells[money_start])?;
    let final_ = parse_cents(&cells[money_start + 1])?;
    let actual = parse_cents(&cells[money_start + 2])?;
    let published_variance = parse_cents(&cells[money_start + 3])?;

    // The document states the variance, and it must equal what the other three imply. This is
    // the check that catches a river cutting a column in the wrong place: a misaligned row
    // still parses as four numbers, and only the arithmetic gives it away.
    let implied = final_ - actual;
    if published_variance != implied {
        bail!(
            "row {:?} states a variance of {published_variance} cents but its final minus \
             actual is {implied}; the columns were cut in the wrong place",
            cells.join(" | ")
        );
    }

    let head = cells[..money_start].join(" ");
    let mut tokens = head.split_whitespace().peekable();
    let mut agency = Vec::new();
    let mut fund = None;
    let mut ali = None;
    let mut name = Vec::new();
    while let Some(tok) = tokens.next() {
        if ali.is_some() {
            name.push(tok);
        } else if looks_like_ali(tok) {
            ali = Some(tok.to_string());
        } else if tokens.peek().is_some_and(|n| looks_like_ali(n)) {
            // The cell immediately before the line item code is the fund.
            fund = Some(tok.to_string());
        } else {
            agency.push(tok);
        }
    }
    let (Some(fund), Some(line_item_code)) = (fund, ali) else {
        return Ok(None);
    };
    Ok(Some(ObmBudgetaryRow {
        agency_name: agency.join(" "),
        fund_code: fund,
        line_item_code,
        line_item_name: name.join(" "),
        fiscal_year: fiscal_year.to_string(),
        original_cents: original,
        final_cents: final_,
        actual_cents: actual,
        provenance: provenance.clone(),
    }))
}

/// Reads every row of a reconstructed page.
pub fn rows_from_table(table: &RawTable, fiscal_year: &str, provenance: &Provenance) -> PageReport {
    let mut report = PageReport::default();
    for cells in &table.rows {
        match row_from_cells(cells, fiscal_year, provenance) {
            Ok(Some(row)) => report.rows.push(row),
            Ok(None) => report.skipped += 1,
            Err(e) => report.problems.push(format!("{e:#}")),
        }
    }
    report
}

/// Reconstructs every page's table from PDF bytes.
#[cfg(feature = "pdf")]
pub fn tables(bytes: &[u8]) -> Result<Vec<(u32, Result<RawTable>)>> {
    lsc::pdf::tables_from_pdf(bytes, &lsc::geometry::TableOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance() -> Provenance {
        Provenance {
            catalog_slug: "synthetic-fixture".to_string(),
            document_ref: "SYNTHETIC-dasf".to_string(),
            locator: None,
            retrieved: "1970-01-01".to_string(),
        }
    }

    fn cells(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn cents_are_exact_through_the_fractional_part() {
        assert_eq!(parse_cents("424,900,000.00").unwrap(), 42_490_000_000);
        assert_eq!(parse_cents("451,474,951.44").unwrap(), 45_147_495_144);
        assert_eq!(parse_cents("0.00").unwrap(), 0);
        assert_eq!(parse_cents("(1,234.56)").unwrap(), -123_456);
        assert_eq!(parse_cents("-1,234.56").unwrap(), -123_456);
    }

    #[test]
    fn a_figure_without_cents_is_refused() {
        // Every figure in this report states cents. One that does not is a sign the decimal
        // point was lost in decoding, which is the failure this connector exists to refuse.
        assert!(parse_cents("424,900,000").is_err());
        assert!(parse_cents(",00000").is_err());
    }

    #[test]
    fn a_real_row_reads_and_reconciles() {
        // Local government, FY2021 — the row that agrees with the corpus to the cent.
        let row = row_from_cells(
            &cells(&[
                "DEPARTMENT OF TAXATION",
                "7069 110969 LOCAL GOVERNMENT",
                "424,900,000.00",
                "451,559,619.78",
                "451,474,951.44",
                "84,668.34",
            ]),
            "FY2021",
            &provenance(),
        )
        .unwrap()
        .expect("a data row");
        assert_eq!(row.agency_name, "DEPARTMENT OF TAXATION");
        assert_eq!(row.fund_code, "7069");
        assert_eq!(row.line_item_code, "110969");
        assert_eq!(row.line_item_name, "LOCAL GOVERNMENT");
        assert_eq!(row.original_cents, 42_490_000_000);
        assert_eq!(row.final_cents, 45_155_961_978);
        assert_eq!(row.actual_cents, 45_147_495_144);
        assert_eq!(row.unspent_cents(), 8_466_834);
    }

    #[test]
    fn an_alphanumeric_line_item_code_is_a_line_item_code() {
        // `2006A2` is a real ALI. A rule that required six digits would drop it silently.
        let row = row_from_cells(
            &cells(&[
                "DEPARTMENT OF EDUCATION AND WORKFORCE 5AD1 2006A2 CTE EQUIPMENT",
                "50,000,000.00",
                "97,750,755.60",
                "97,750,754.60",
                "1.00",
            ]),
            "FY2025",
            &provenance(),
        )
        .unwrap()
        .expect("a data row");
        assert_eq!(row.line_item_code, "2006A2");
        assert_eq!(row.fund_code, "5AD1");
        assert_eq!(row.line_item_name, "CTE EQUIPMENT");
    }

    #[test]
    fn a_misaligned_row_is_an_error_not_a_figure() {
        // The whole reason the report's own variance column is read. Shift the columns by one
        // and every cell still parses as money; only the arithmetic disagrees.
        let err = row_from_cells(
            &cells(&[
                "DEPARTMENT OF TAXATION 7069 110969 LOCAL GOVERNMENT",
                "451,559,619.78",
                "451,474,951.44",
                "84,668.34",
                "0.00",
            ]),
            "FY2021",
            &provenance(),
        )
        .unwrap_err();
        assert!(
            format!("{err:#}").contains("cut in the wrong place"),
            "{err:#}"
        );
    }

    #[test]
    fn a_section_heading_is_skipped_rather_than_faulted() {
        for heading in [
            "FUND TYPE - GENERAL",
            "GAAP CHARACTER OF EXPENDITURE - CURRENT OPERATING",
        ] {
            assert!(row_from_cells(&cells(&[heading]), "FY2021", &provenance())
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn a_page_missing_a_digit_entirely_is_refused() {
        // Measured against the real document: `pdf-extract` returns page 3 of the FY2021
        // report with every `.` and every digit from 4 to 9 absent, and 0 to 3 intact.
        let mangled = "DEPARTMENT OF TAATION 0 110 LOCAL GOERNMENT ,00000 ,110 ,111 ,3";
        let err = verify_decode(mangled).unwrap_err();
        assert!(format!("{err:#}").contains("silently wrong"), "{err:#}");
    }

    #[test]
    fn a_page_that_decoded_is_accepted() {
        assert!(verify_decode(
            "DEPARTMENT OF TAXATION 7069 110969 LOCAL GOVERNMENT \
             424,900,000.00 451,559,619.78 451,474,951.44 84,668.34"
        )
        .is_ok());
    }
}
