//! Connector for Legislative Service Commission budget publications.
//!
//! The load-bearing connector. LSC publishes two things this corpus needs, and they are not
//! interchangeable:
//!
//! - the **appropriation spreadsheet**, which carries one figure per line item per stage per
//!   fiscal year. Read by [`xlsx`] and [`columns`] into [`LscComparisonRow`] records. This is
//!   where every amount in the corpus comes from.
//! - the **comparison document**, which carries one paragraph of prose per provision per
//!   chamber. Read by [`ruled`] and [`comparison`] into [`corpus_schema::LscProvisionRow`]
//!   records. This is where every `stated_justification` comes from.
//!
//! The spreadsheet says what a figure became; the comparison document says why. Keeping them
//! apart matters more than it sounds, because the comparison document also quotes dollar
//! amounts and those amounts are **distribution estimates for a recipient class**, not
//! appropriation authority. For HB 96's FY2026 school funding the two move in opposite
//! directions at two of three transitions. See [`comparison`] and the
//! `appropriation-is-not-distribution` decision record.
//!
//! # Table geometry
//!
//! A PDF holds characters at coordinates, not tables, so recovering cells is a geometry
//! problem — and the two publications need different algorithms. [`geometry`] finds columns as
//! whitespace **rivers**, which is right for the spreadsheet. [`ruled`] reads the column
//! separators the comparison document **draws for itself**, which is the only thing that works
//! when two columns are set 0.44 points apart. [`pdf`] (behind the `pdf` feature) supplies the
//! positioned glyphs both consume.
//!
//! [`geometry`] produces the same [`RawTable`] that [`parse_delimited`] does, so a PDF and a
//! hand-written file are indistinguishable to everything downstream.
//!
//! # Money
//!
//! [`parse_money_to_cents`] is the most safety-critical function in this repository. Budget
//! figures are summed across thousands of line items; a parser that silently rounds, or that
//! accepts a value it cannot represent exactly, produces totals that are wrong by amounts no
//! downstream check can detect. It refuses rather than approximates.

pub mod columns;
pub mod comparison;
pub mod extract;
pub mod geometry;
#[cfg(feature = "pdf")]
pub mod pdf;
pub mod ruled;
#[cfg(feature = "xlsx")]
pub mod xlsx;

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use corpus_schema::{BillStage, Cents, LscComparisonRow, Provenance};

// ─── money ───────────────────────────────────────────────────────────────────
/// Is this row's ALI code a line item's, or something else in the same column?
///
/// **Not every row in an LSC workbook is a line item**, and the ones that are not carry figures
/// large enough to swamp anything computed over them. Two shapes, and this catches the first:
///
/// 1. **Grand totals.** Every workbook carries between one and six rows with no ALI code —
///    `Grand Total`, `Total All Funds`, `Total GRF`, `GRF State Total`. Each one's movement is
///    the sum of every line item's. Reading them as line items shifted the share of contested
///    money conference splits by thirteen percentage points, changed the line counts by two out
///    of 753, and so survived a check of the counts. They have an agency, a fund group, and a
///    plausible figure; only the empty code marks them.
///
/// 2. **Memorandum breakdowns**, which share their parent's code and are suffixed `- State`,
///    `- Federal`, `- Total`. Those are *not* caught here, because they are only components in
///    the company of a sibling: ALI 651623 is genuinely named `Medicaid Services - Federal` and
///    is a line item in its own right. Callers that select by code must disambiguate within the
///    code — see `ExtractionReport::ambiguous_line_item_codes`, which exists because taking the
///    `- Federal` row of ALI 651525 produced a $3.5 billion error.
pub fn is_line_item_code(code: &str) -> bool {
    !code.trim().is_empty()
}

/// Parses a published dollar figure into exact integer cents.
///
/// Accepts the forms these documents actually use: `$1,234,567.89`, `1234567`, `(1,234.56)`
/// for negatives, and a leading minus. Rejects anything it cannot represent exactly — most
/// importantly a value with more than two decimal places, which would otherwise be silently
/// rounded into a figure that looks authoritative and is wrong.
pub fn parse_money_to_cents(raw: &str) -> Result<Cents> {
    let t = raw.trim();
    if t.is_empty() {
        bail!("empty value: an absent figure is not zero and must not be parsed as one");
    }

    let (negative, body) = if let (Some(b), true) = (t.strip_prefix('('), t.ends_with(')')) {
        (true, &b[..b.len() - 1])
    } else if let Some(b) = t.strip_prefix('-') {
        (true, b)
    } else {
        (false, t)
    };

    let cleaned: String = body
        .chars()
        .filter(|c| !matches!(c, '$' | ',' | ' ' | '\u{a0}' | '_'))
        .collect();
    if cleaned.is_empty() {
        bail!("no digits in {raw:?}");
    }

    let (whole, frac) = match cleaned.split_once('.') {
        Some((w, f)) => (w, f),
        None => (cleaned.as_str(), ""),
    };

    if whole.is_empty() && frac.is_empty() {
        bail!("no digits in {raw:?}");
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        bail!("not a number: {raw:?}");
    }
    if frac.len() > 2 {
        bail!(
            "{raw:?} has {} decimal places; cents cannot represent it exactly and rounding \
             here would be undetectable downstream",
            frac.len()
        );
    }

    let cents_frac: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>()? * 10,
        _ => frac.parse::<i64>()?,
    };
    let whole_v: i64 = if whole.is_empty() { 0 } else { whole.parse()? };

    let magnitude = whole_v
        .checked_mul(100)
        .and_then(|v| v.checked_add(cents_frac))
        .ok_or_else(|| anyhow!("{raw:?} overflows i64 cents"))?;

    Ok(if negative { -magnitude } else { magnitude })
}

/// Renders cents back to a dollar string. Used in reports, never in stored records.
pub fn format_cents(c: Cents) -> String {
    let neg = c < 0;
    let a = c.unsigned_abs();
    let s = format!("{}.{:02}", a / 100, a % 100);
    if neg {
        format!("-${s}")
    } else {
        format!("${s}")
    }
}

// ─── delimited tables ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Parses delimited text produced by an upstream PDF-to-text step.
///
/// Blank lines and `#` comments are skipped. Rows with a different column count than the
/// header are an error rather than a truncation — a misaligned row in a budget table means
/// figures land in the wrong fiscal year.
pub fn parse_delimited(text: &str, delim: char) -> Result<RawTable> {
    // Only the line ending is stripped, never trailing whitespace: a trailing delimiter
    // marks a final empty cell, and trimming it away silently drops a column. A blank cell
    // means the document reported no figure, which is not the same as the column not
    // existing — so losing it here would turn "absent" into "absent from the table".
    let mut lines = text
        .lines()
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));

    let header_line = lines.next().ok_or_else(|| anyhow!("no header row"))?;
    let headers: Vec<String> = header_line
        .split(delim)
        .map(|h| h.trim().to_string())
        .collect();

    let mut rows = Vec::new();
    for (i, line) in lines.enumerate() {
        let cells: Vec<String> = line.split(delim).map(|c| c.trim().to_string()).collect();
        if cells.len() != headers.len() {
            bail!(
                "row {} has {} cells against {} headers; a misaligned row puts figures in the \
                 wrong fiscal year",
                i + 1,
                cells.len(),
                headers.len()
            );
        }
        rows.push(cells);
    }
    Ok(RawTable { headers, rows })
}

fn norm_header(h: &str) -> String {
    h.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A fiscal-year column, e.g. header `FY 2026` at index 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiscalYearColumn {
    pub index: usize,
    pub fiscal_year: String,
}

/// Which column holds what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnMap {
    pub agency_code: usize,
    pub line_item_code: usize,
    pub line_item_name: usize,
    pub fund_group: Option<usize>,
    pub fund_code: Option<usize>,
    pub fiscal_years: Vec<FiscalYearColumn>,
}

fn find(headers: &[String], candidates: &[&str]) -> Option<usize> {
    headers
        .iter()
        .position(|h| candidates.contains(&norm_header(h).as_str()))
}

/// Detects a fiscal-year column from its header, e.g. `FY 2026`, `FY2026`, `fy 2026 approp`.
pub fn fiscal_year_of_header(h: &str) -> Option<String> {
    let n = norm_header(h);
    let mut toks = n.split_whitespace().peekable();
    while let Some(t) = toks.next() {
        if let Some(rest) = t.strip_prefix("fy") {
            if rest.len() == 4 && rest.chars().all(|c| c.is_ascii_digit()) {
                return Some(format!("FY{rest}"));
            }
            if rest.is_empty() {
                if let Some(next) = toks.peek() {
                    if next.len() == 4 && next.chars().all(|c| c.is_ascii_digit()) {
                        return Some(format!("FY{next}"));
                    }
                }
            }
        }
    }
    None
}

pub fn map_columns(headers: &[String]) -> Result<ColumnMap> {
    let agency_code = find(headers, &["agency code", "agency", "agy"])
        .ok_or_else(|| anyhow!("no agency code column in {headers:?}"))?;
    let line_item_code = find(headers, &["ali", "line item code", "ali code", "code"])
        .ok_or_else(|| anyhow!("no line item code column in {headers:?}"))?;
    let line_item_name = find(
        headers,
        &["line item name", "line item", "ali name", "title", "name"],
    )
    .ok_or_else(|| anyhow!("no line item name column in {headers:?}"))?;

    let fiscal_years: Vec<FiscalYearColumn> = headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            fiscal_year_of_header(h).map(|fy| FiscalYearColumn {
                index: i,
                fiscal_year: fy,
            })
        })
        .collect();
    if fiscal_years.is_empty() {
        bail!("no fiscal year columns in {headers:?}");
    }

    Ok(ColumnMap {
        agency_code,
        line_item_code,
        line_item_name,
        fund_group: find(headers, &["fund group", "group"]),
        fund_code: find(headers, &["fund", "fund code"]),
        fiscal_years,
    })
}

// ─── normalization ───────────────────────────────────────────────────────────

/// What a table cannot tell you about itself.
#[derive(Debug, Clone)]
pub struct TableContext {
    pub bill_number: String,
    pub general_assembly: String,
    pub stage: BillStage,
    pub provenance: Provenance,
}

/// One outcome per table cell that should have held a figure.
#[derive(Debug, Clone)]
pub enum CellOutcome {
    Row(Box<LscComparisonRow>),
    /// A blank cell. Absent is not zero, so it is reported rather than parsed.
    Blank {
        line_item_code: String,
        fiscal_year: String,
    },
    /// A cell that would not parse. Reported rather than dropped.
    Unparsed {
        line_item_code: String,
        fiscal_year: String,
        raw: String,
        reason: String,
    },
}

/// Fans a table out into one record per (line item, fiscal year).
///
/// A comparison table carries one row per line item and one column per fiscal year, so a
/// single table row is several appropriations. Nothing is dropped: blanks and unparsable
/// cells come back as outcomes so an extraction run can report its own coverage rather than
/// silently returning fewer records than the document contained.
pub fn normalize(table: &RawTable, map: &ColumnMap, ctx: &TableContext) -> Vec<CellOutcome> {
    let mut out = Vec::new();
    for row in &table.rows {
        let code = row[map.line_item_code].clone();
        let name = row[map.line_item_name].clone();
        let agency = row[map.agency_code].clone();
        let fund_group = map
            .fund_group
            .map(|i| row[i].clone())
            .unwrap_or_else(|| "[open]".into());
        let fund_code = map
            .fund_code
            .map(|i| row[i].clone())
            .unwrap_or_else(|| "[open]".into());

        for fy in &map.fiscal_years {
            let raw = row[fy.index].trim();
            if raw.is_empty() || raw == "-" {
                out.push(CellOutcome::Blank {
                    line_item_code: code.clone(),
                    fiscal_year: fy.fiscal_year.clone(),
                });
                continue;
            }
            match parse_money_to_cents(raw) {
                Ok(amount_cents) => out.push(CellOutcome::Row(Box::new(LscComparisonRow {
                    bill_number: ctx.bill_number.clone(),
                    general_assembly: ctx.general_assembly.clone(),
                    stage: ctx.stage,
                    agency_code: agency.clone(),
                    line_item_code: code.clone(),
                    line_item_name: name.clone(),
                    fund_group: fund_group.clone(),
                    fund_code: fund_code.clone(),
                    fiscal_year: fy.fiscal_year.clone(),
                    amount_cents,
                    provenance: ctx.provenance.clone(),
                }))),
                Err(e) => out.push(CellOutcome::Unparsed {
                    line_item_code: code.clone(),
                    fiscal_year: fy.fiscal_year.clone(),
                    raw: raw.to_string(),
                    reason: e.to_string(),
                }),
            }
        }
    }
    out
}

/// Keeps only the successfully parsed records.
pub fn rows_only(outcomes: Vec<CellOutcome>) -> Vec<LscComparisonRow> {
    outcomes
        .into_iter()
        .filter_map(|o| match o {
            CellOutcome::Row(r) => Some(*r),
            _ => None,
        })
        .collect()
}

// ─── sources ─────────────────────────────────────────────────────────────────

pub trait Source {
    fn fetch_all(&self) -> Result<Vec<LscComparisonRow>>;
}

pub struct FixtureSource {
    root: PathBuf,
}

impl FixtureSource {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
    pub fn from_repo(repo_root: impl AsRef<Path>) -> Self {
        Self::new(repo_root.as_ref().join(".yidam/fixtures/lsc"))
    }
}

impl Source for FixtureSource {
    fn fetch_all(&self) -> Result<Vec<LscComparisonRow>> {
        let mut out = Vec::new();
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&self.root)
            .with_context(|| format!("reading {}", self.root.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "yml"))
            .collect();
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path)?;
            let rows: Vec<LscComparisonRow> = serde_yaml::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))?;
            out.extend(rows);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_row_with_no_ali_code_is_not_a_line_item() {
        // The grand-total rows. They carry an agency, a fund group and a figure that is the sum
        // of everything above them; the empty code is the only thing marking them.
        assert!(!is_line_item_code(""));
        assert!(!is_line_item_code("   "));
        assert!(is_line_item_code("200550"));
        // Ohio numbers one line item `110644.00`, so a code is not always digits.
        assert!(is_line_item_code("110644.00"));
        // A memorandum breakdown carries its parent's code and is not caught here, by design:
        // ALI 651623 is genuinely named `Medicaid Services - Federal`.
        assert!(is_line_item_code("651525"));
    }
    use super::*;

    fn prov() -> Provenance {
        Provenance {
            catalog_slug: "lsc-hb96-comparison".into(),
            document_ref: "test".into(),
            locator: None,
            retrieved: "2026-08-08".into(),
        }
    }

    #[test]
    fn money_parses_published_forms_exactly() {
        assert_eq!(parse_money_to_cents("$1,234,567.89").unwrap(), 123456789);
        assert_eq!(parse_money_to_cents("1234567").unwrap(), 123456700);
        assert_eq!(
            parse_money_to_cents("  $8,123,456,789.00 ").unwrap(),
            812345678900
        );
        assert_eq!(parse_money_to_cents("0").unwrap(), 0);
        assert_eq!(parse_money_to_cents(".5").unwrap(), 50);
    }

    #[test]
    fn money_handles_both_negative_conventions() {
        assert_eq!(parse_money_to_cents("(1,234.56)").unwrap(), -123456);
        assert_eq!(parse_money_to_cents("-1,234.56").unwrap(), -123456);
        assert_eq!(parse_money_to_cents("($8,750.00)").unwrap(), -875000);
    }

    #[test]
    fn money_refuses_precision_it_cannot_represent() {
        // The critical case. Rounding here would be undetectable downstream.
        let e = parse_money_to_cents("1,234.567").unwrap_err().to_string();
        assert!(e.contains("decimal places"), "{e}");
        assert!(parse_money_to_cents("1.2345").is_err());
    }

    #[test]
    fn money_refuses_absent_values_rather_than_calling_them_zero() {
        assert!(parse_money_to_cents("").is_err());
        assert!(parse_money_to_cents("   ").is_err());
        assert!(parse_money_to_cents("$").is_err());
    }

    #[test]
    fn money_refuses_garbage() {
        assert!(parse_money_to_cents("n/a").is_err());
        assert!(parse_money_to_cents("1,2x4").is_err());
        assert!(parse_money_to_cents("--5").is_err());
    }

    #[test]
    fn cents_round_trip_through_display() {
        for v in [0i64, 1, 99, 100, -123456789, 812345678900] {
            let s = format_cents(v);
            assert_eq!(parse_money_to_cents(&s).unwrap(), v, "{s}");
        }
    }

    #[test]
    fn trailing_delimiter_preserves_the_final_empty_cell() {
        // Regression: trimming the line before splitting dropped the last column when a row
        // ended with a delimiter, turning a reported blank into a missing column.
        let text = "A\tB\tC\n1\t2\t\n";
        let t = parse_delimited(text, '\t').unwrap();
        assert_eq!(t.rows[0].len(), 3);
        assert_eq!(t.rows[0][2], "");
    }

    #[test]
    fn misaligned_row_is_an_error_not_a_truncation() {
        let text = "Agency\tALI\tName\tFY 2026\n200\t200550\tFoundation Funding\n";
        let e = parse_delimited(text, '\t').unwrap_err().to_string();
        assert!(e.contains("wrong fiscal year"), "{e}");
    }

    #[test]
    fn fiscal_year_headers_are_detected_in_several_spellings() {
        assert_eq!(fiscal_year_of_header("FY 2026").as_deref(), Some("FY2026"));
        assert_eq!(fiscal_year_of_header("FY2027").as_deref(), Some("FY2027"));
        assert_eq!(
            fiscal_year_of_header("FY 2026 Appropriation").as_deref(),
            Some("FY2026")
        );
        assert_eq!(fiscal_year_of_header("Line Item Name"), None);
    }

    fn sample() -> (RawTable, ColumnMap) {
        let text = "\
# comment line is skipped
Agency Code\tALI\tLine Item Name\tFund Group\tFund\tFY 2026\tFY 2027
200\t200550\tFoundation Funding\tGRF\t5000\t$8,000,000,000.00\t$8,310,000,000.00
651\t651525\tMedicaid Health Care Services\tGRF\t5000\t$21,400,000,000.00\t
775\t775xxx\tHighway Construction\tHOF\t7002\tn/a\t$1,000,000.00
";
        let table = parse_delimited(text, '\t').unwrap();
        let map = map_columns(&table.headers).unwrap();
        (table, map)
    }

    #[test]
    fn columns_map_and_fiscal_years_fan_out() {
        let (table, map) = sample();
        assert_eq!(map.agency_code, 0);
        assert_eq!(map.line_item_code, 1);
        assert_eq!(map.fiscal_years.len(), 2);
        assert_eq!(table.rows.len(), 3);
    }

    #[test]
    fn normalize_reports_blanks_and_failures_instead_of_dropping_them() {
        let (table, map) = sample();
        let ctx = TableContext {
            bill_number: "HB 96".into(),
            general_assembly: "136th".into(),
            stage: BillStage::AsEnacted,
            provenance: prov(),
        };
        let outcomes = normalize(&table, &map, &ctx);
        // Three rows times two fiscal year columns: every cell accounted for.
        assert_eq!(outcomes.len(), 6);

        let blanks = outcomes
            .iter()
            .filter(|o| matches!(o, CellOutcome::Blank { .. }))
            .count();
        let unparsed = outcomes
            .iter()
            .filter(|o| matches!(o, CellOutcome::Unparsed { .. }))
            .count();
        assert_eq!(blanks, 1, "the empty FY2027 Medicaid cell");
        assert_eq!(unparsed, 1, "the n/a highway cell");

        let rows = rows_only(outcomes);
        assert_eq!(rows.len(), 4);
        let ff = rows
            .iter()
            .find(|r| r.line_item_code == "200550" && r.fiscal_year == "FY2026")
            .unwrap();
        assert_eq!(ff.amount_cents, 800_000_000_000);
        assert_eq!(ff.stage, BillStage::AsEnacted);
        assert_eq!(ff.bill_number, "HB 96");
    }

    #[test]
    fn fixture_source_reads_committed_rows() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let rows = FixtureSource::from_repo(root).fetch_all().unwrap();
        assert!(rows.len() >= 5);
        assert!(rows
            .iter()
            .all(|r| r.provenance.catalog_slug == "synthetic-fixture"));
    }
}
