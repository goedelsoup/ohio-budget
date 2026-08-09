//! Turns a classified table into typed extraction records.
//!
//! Uses [`crate::columns::ColumnPlan`] rather than [`crate::map_columns`], so each money
//! column carries its own stage. Everything is accounted for: a cell that is blank, that
//! fails to parse, or that sits under a column whose stage could not be identified is
//! reported rather than dropped, because a run that silently returns fewer records than the
//! document contained looks exactly like a run that found fewer figures.

use corpus_schema::{LscComparisonRow, Provenance};

use crate::columns::{ColumnPlan, Identity};
use crate::{parse_money_to_cents, RawTable};

/// What a table cannot tell you about itself. Stage is deliberately absent — it comes from
/// the column, not from the caller.
#[derive(Debug, Clone)]
pub struct ExtractionContext {
    pub bill_number: String,
    pub general_assembly: String,
    pub provenance: Provenance,
}

/// A completed year's outturn, which is an expenditure rather than an appropriation.
#[derive(Debug, Clone, PartialEq)]
pub struct ActualRow {
    pub agency_code: String,
    pub line_item_code: String,
    pub line_item_name: String,
    pub fiscal_year: String,
    pub amount_cents: i64,
}

#[derive(Debug, Clone)]
pub struct Failure {
    pub line_item_code: String,
    pub fiscal_year: String,
    pub raw: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractionReport {
    pub appropriations: Vec<LscComparisonRow>,
    pub actuals: Vec<ActualRow>,
    /// Cells reporting no figure. Counted rather than listed — there are many, and absent is
    /// a normal state for a line item that received nothing in a given year.
    pub blanks: usize,
    pub failures: Vec<Failure>,
    /// Money columns whose stage could not be identified, with their header label. Every
    /// figure under these was skipped.
    pub unclassified_columns: Vec<(String, String)>,
    /// Estimate columns, skipped by design. An estimate is neither an appropriation nor an
    /// actual, and silently treating one as either is the error this field exists to prevent.
    pub estimate_columns_skipped: Vec<String>,
    /// Line item codes that appear under more than one name, with those names.
    ///
    /// A workbook may carry a memorandum breakdown of a line item beneath the schedule proper.
    /// HB 110's does: ALI 651525 appears as `Medicaid Health Care Services` and then again as
    /// `- State`, `- Federal`, and `- Total`, the middle two summing to the first.
    ///
    /// Nothing distinguishes them but the name. Anything that selects a row by code alone —
    /// a promotion tool, a join, an analyst — can silently take a component for the whole, and
    /// the resulting figure looks entirely reasonable. It produced a $3.5 billion phantom
    /// underspend in this repository, committed as `[verified]`, and was caught only because a
    /// second workbook overlapped the same fiscal year.
    pub ambiguous_line_item_codes: Vec<(String, Vec<String>)>,
}

impl ExtractionReport {
    /// Every figure the document contained, whether or not it produced a record. A caller
    /// comparing this against the document's own row count can tell whether extraction was
    /// complete.
    pub fn cells_accounted(&self) -> usize {
        self.appropriations.len() + self.actuals.len() + self.blanks + self.failures.len()
    }
}

fn cell(row: &[String], i: Option<usize>) -> &str {
    i.and_then(|i| row.get(i)).map(String::as_str).unwrap_or("")
}

/// Extracts every money cell, one record per (line item, column).
pub fn extract(table: &RawTable, plan: &ColumnPlan, ctx: &ExtractionContext) -> ExtractionReport {
    let mut r = ExtractionReport {
        unclassified_columns: plan
            .unclassified()
            .into_iter()
            .map(|(_, fy, label)| (fy.to_string(), label.to_string()))
            .collect(),
        estimate_columns_skipped: plan
            .kinds
            .iter()
            .filter_map(|k| match k {
                crate::columns::ColumnKind::Estimate {
                    fiscal_year,
                    issuer,
                } => Some(format!("{issuer} {fiscal_year}")),
                _ => None,
            })
            .collect(),
        ..Default::default()
    };

    // A code carrying more than one name means the sheet distinguishes rows by something the
    // code does not capture. Collected before extraction so the report can say so even when
    // every row parses cleanly, which is exactly when it goes unnoticed.
    {
        let i_code = plan.index_of(Identity::LineItemCode);
        let i_name = plan.index_of(Identity::LineItemName);
        let mut names: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
            Default::default();
        for row in &table.rows {
            let code = cell(row, i_code).trim().to_string();
            let name = cell(row, i_name).trim().to_string();
            if !code.is_empty() && !name.is_empty() {
                names.entry(code).or_default().insert(name);
            }
        }
        r.ambiguous_line_item_codes = names
            .into_iter()
            .filter(|(_, n)| n.len() > 1)
            .map(|(c, n)| (c, n.into_iter().collect()))
            .collect();
    }

    let i_agency = plan.index_of(Identity::Agency);
    let i_code = plan.index_of(Identity::LineItemCode);
    let i_name = plan.index_of(Identity::LineItemName);
    let i_group = plan.index_of(Identity::FundGroup);
    let i_fund = plan.index_of(Identity::Fund);

    let appr_cols = plan.appropriation_columns();
    let actual_cols = plan.actual_columns();

    for row in &table.rows {
        let code = cell(row, i_code).to_string();
        if !crate::is_line_item_code(&code) {
            continue;
        }
        let name = cell(row, i_name).to_string();
        let agency = cell(row, i_agency).to_string();
        let group = cell(row, i_group).to_string();
        let fund = cell(row, i_fund).to_string();

        let mut take = |idx: usize, fy: &str| -> Option<i64> {
            let raw = row.get(idx).map(String::as_str).unwrap_or("").trim();
            if raw.is_empty() || raw == "-" {
                r.blanks += 1;
                return None;
            }
            match parse_money_to_cents(raw) {
                Ok(c) => Some(c),
                Err(e) => {
                    r.failures.push(Failure {
                        line_item_code: code.clone(),
                        fiscal_year: fy.to_string(),
                        raw: raw.to_string(),
                        reason: e.to_string(),
                    });
                    None
                }
            }
        };

        let mut appr = Vec::new();
        for (idx, stage, fy) in &appr_cols {
            if let Some(amount_cents) = take(*idx, fy) {
                appr.push(LscComparisonRow {
                    bill_number: ctx.bill_number.clone(),
                    general_assembly: ctx.general_assembly.clone(),
                    stage: *stage,
                    agency_code: agency.clone(),
                    line_item_code: code.clone(),
                    line_item_name: name.clone(),
                    fund_group: group.clone(),
                    fund_code: fund.clone(),
                    fiscal_year: fy.to_string(),
                    amount_cents,
                    provenance: ctx.provenance.clone(),
                });
            }
        }
        let mut acts = Vec::new();
        for (idx, fy) in &actual_cols {
            if let Some(amount_cents) = take(*idx, fy) {
                acts.push(ActualRow {
                    agency_code: agency.clone(),
                    line_item_code: code.clone(),
                    line_item_name: name.clone(),
                    fiscal_year: fy.to_string(),
                    amount_cents,
                });
            }
        }
        r.appropriations.extend(appr);
        r.actuals.extend(acts);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpus_schema::BillStage;

    fn ctx() -> ExtractionContext {
        ExtractionContext {
            bill_number: "HB 96".into(),
            general_assembly: "136th".into(),
            provenance: Provenance {
                catalog_slug: "lsc-hb96-appropriation-spreadsheet".into(),
                document_ref: "EN".into(),
                locator: None,
                retrieved: "2026-08-08".into(),
            },
        }
    }

    fn table() -> RawTable {
        RawTable {
            headers: [
                "Agency",
                "Fund Group",
                "Fund",
                "ALI",
                "ALI Name",
                "FY 2024",
                "OBM Estimate\r\nFY 2025",
                "Introduced\r\nFY 2026",
                "House Passed\r\nFY 2026",
                "As Enacted\r\nFY 2026",
                "Governor Recommended\r\nFY 2026",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            rows: vec![
                [
                    "EDU",
                    "GRF",
                    "GRF",
                    "200550",
                    "Foundation Funding",
                    "100.00",
                    "110.00",
                    "120.00",
                    "130.00",
                    "140.00",
                    "150.00",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                [
                    "MCD", "GRF", "GRF", "651525", "Medicaid", "200.00", "", "", "n/a", "240.00",
                    "",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            ],
        }
    }

    #[test]
    fn each_appropriation_column_keeps_its_own_stage() {
        let t = table();
        let plan = ColumnPlan::of(&t.headers);
        let r = extract(&t, &plan, &ctx());
        let ff: Vec<&LscComparisonRow> = r
            .appropriations
            .iter()
            .filter(|a| a.line_item_code == "200550")
            .collect();
        assert_eq!(ff.len(), 3);
        let mut stages: Vec<BillStage> = ff.iter().map(|a| a.stage).collect();
        stages.sort();
        assert_eq!(
            stages,
            vec![
                BillStage::AsIntroduced,
                BillStage::AsPassedHouse,
                BillStage::AsEnacted
            ]
        );
        assert_eq!(
            ff.iter()
                .find(|a| a.stage == BillStage::AsEnacted)
                .unwrap()
                .amount_cents,
            14000
        );
    }

    #[test]
    fn a_bare_year_becomes_an_actual_not_an_appropriation() {
        let t = table();
        let r = extract(&t, &ColumnPlan::of(&t.headers), &ctx());
        assert_eq!(r.actuals.len(), 2);
        assert!(r.actuals.iter().all(|a| a.fiscal_year == "FY2024"));
        assert!(r.appropriations.iter().all(|a| a.fiscal_year != "FY2024"));
    }

    #[test]
    fn an_estimate_column_is_skipped_and_named() {
        let t = table();
        let r = extract(&t, &ColumnPlan::of(&t.headers), &ctx());
        assert_eq!(r.estimate_columns_skipped.len(), 1);
        assert!(r.estimate_columns_skipped[0].contains("FY2025"));
        // The estimate value 110.00 must appear nowhere.
        assert!(r.appropriations.iter().all(|a| a.amount_cents != 11000));
        assert!(r.actuals.iter().all(|a| a.amount_cents != 11000));
    }

    #[test]
    fn an_unclassified_stage_column_is_reported_and_its_figures_skipped() {
        let t = table();
        let r = extract(&t, &ColumnPlan::of(&t.headers), &ctx());
        assert_eq!(r.unclassified_columns.len(), 1);
        assert_eq!(r.unclassified_columns[0].1, "governor recommended");
        assert!(r.appropriations.iter().all(|a| a.amount_cents != 15000));
    }

    #[test]
    fn blanks_and_failures_are_counted_not_dropped() {
        let t = table();
        let r = extract(&t, &ColumnPlan::of(&t.headers), &ctx());
        // The Medicaid row has one blank among the *classified* columns — its other two
        // empty cells sit under the estimate and unclassified columns, which are skipped
        // wholesale and so are neither blanks nor failures. Counting them would overstate
        // what the document actually reported.
        assert_eq!(r.blanks, 1, "blanks {}", r.blanks);
        assert_eq!(r.failures.len(), 1);
        assert_eq!(r.failures[0].raw, "n/a");
        assert_eq!(r.failures[0].line_item_code, "651525");
    }

    #[test]
    fn every_classified_cell_is_accounted_for() {
        // Two rows times five classified money columns (one actual, three appropriation,
        // one unclassified which is skipped wholesale) — the accounting must add up so a
        // caller can tell a short extraction from a short document.
        let t = table();
        let plan = ColumnPlan::of(&t.headers);
        let r = extract(&t, &plan, &ctx());
        let classified = plan.appropriation_columns().len() + plan.actual_columns().len();
        assert_eq!(r.cells_accounted(), classified * t.rows.len());
    }

    #[test]
    fn a_row_without_a_line_item_code_is_skipped() {
        let mut t = table();
        t.rows.push(
            ["", "", "", "", "Subtotal", "999.00", "", "", "", "", ""]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        );
        let r = extract(&t, &ColumnPlan::of(&t.headers), &ctx());
        assert!(r.actuals.iter().all(|a| a.amount_cents != 99900));
    }
}

#[cfg(test)]
mod ambiguity {
    use super::*;
    use crate::columns::ColumnPlan;
    use crate::parse_delimited;

    fn ctx() -> ExtractionContext {
        ExtractionContext {
            bill_number: "HB 110".into(),
            general_assembly: "134th".into(),
            provenance: corpus_schema::Provenance {
                catalog_slug: "t".into(),
                document_ref: "t".into(),
                locator: None,
                retrieved: "2026-08-09".into(),
            },
        }
    }

    #[test]
    fn one_code_under_several_names_is_reported() {
        // The real shape: a memorandum breakdown beneath the schedule, where the components
        // sum to the line above and nothing but the name distinguishes them. Selecting by code
        // alone takes a component for the whole, and the figure looks entirely reasonable.
        let text = "Agency\tFund Group\tFund\tALI\tALITitle\tFY 2020\n\
                    MCD\tGRF\tGRF\t651525\tMedicaid Health Care Services\t14111993687.92\n\
                    MCD\tGRF\tGRF\t651525\tMedicaid/Health Care Services - State\t3525731926.06\n\
                    MCD\tGRF\tGRF\t651525\tMedicaid/Health Care Services - Federal\t10586261761.86\n\
                    EDU\tGRF\tGRF\t200550\tFoundation Funding\t6687924225.44\n";
        let t = parse_delimited(text, '\t').unwrap();
        let plan = ColumnPlan::of(&t.headers);
        let r = extract(&t, &plan, &ctx());
        assert_eq!(
            r.ambiguous_line_item_codes.len(),
            1,
            "{:?}",
            r.ambiguous_line_item_codes
        );
        let (code, names) = &r.ambiguous_line_item_codes[0];
        assert_eq!(code, "651525");
        assert_eq!(names.len(), 3);
        // Every row still extracts; the point is that the report says the code is not a key.
        assert_eq!(r.actuals.len(), 4);
    }

    #[test]
    fn a_clean_sheet_reports_no_ambiguity() {
        let text = "Agency\tFund Group\tFund\tALI\tALITitle\tFY 2020\n\
                    EDU\tGRF\tGRF\t200550\tFoundation Funding\t1.00\n\
                    DRC\tGRF\tGRF\t501321\tInstitutional Operations\t2.00\n";
        let t = parse_delimited(text, '\t').unwrap();
        let plan = ColumnPlan::of(&t.headers);
        assert!(extract(&t, &plan, &ctx())
            .ambiguous_line_item_codes
            .is_empty());
    }

    #[test]
    fn the_unspaced_name_header_is_recognised() {
        // `ALITitle` appears in every workbook with actuals. Unmatched, it left the name empty
        // on every row — which is what hid the ambiguity above.
        use crate::columns::{classify_header, ColumnKind, Identity};
        assert_eq!(
            classify_header("ALITitle"),
            ColumnKind::Identity(Identity::LineItemName)
        );
        assert_eq!(
            classify_header("CAS"),
            ColumnKind::Identity(Identity::Agency)
        );
    }
}
