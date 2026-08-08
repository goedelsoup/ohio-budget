//! Classifies a comparison table's columns.
//!
//! Written after reading a real LSC appropriation spreadsheet, which broke the assumption
//! [`crate::map_columns`] was built on. That function assumes **one stage per document** and
//! takes the stage from [`crate::TableContext`], because a per-agency comparison document
//! covers one stage. The appropriation spreadsheet does not: it carries every stage of the
//! bill as separate columns, with stage and fiscal year encoded together in each header —
//! `House Passed FY 2026`, `As Enacted FY 2026`.
//!
//! Run through `map_columns`, that workbook reports `FY2026` five times and silently loses
//! which stage each figure belongs to. Every amount would be attributed to whatever single
//! stage the caller happened to name.
//!
//! So columns are classified individually here. Three kinds of money column exist and they
//! are **not** interchangeable:
//!
//! - an **appropriation** at a named stage — authority, and what the corpus stores;
//! - an **actual** for a completed year — money spent, which belongs to `expenditure`;
//! - an **estimate** — neither, and the most dangerous of the three, because it looks exactly
//!   like an appropriation and is somebody's projection.

use corpus_schema::BillStage;

use crate::fiscal_year_of_header;

/// Which identity a non-money column carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identity {
    Agency,
    FundGroup,
    Fund,
    LineItemCode,
    LineItemName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnKind {
    Identity(Identity),
    /// A completed year's outturn. Belongs to `expenditure`, not `appropriation`.
    Actual {
        fiscal_year: String,
    },
    /// Somebody's projection. Never an appropriation, and never an actual.
    Estimate {
        fiscal_year: String,
        issuer: String,
    },
    /// Spending authority at a named stage of the bill.
    Appropriation {
        stage: BillStage,
        fiscal_year: String,
    },
    /// A money column whose stage could not be identified. Reported, never guessed.
    UnknownStage {
        fiscal_year: String,
        label: String,
    },
    /// Not a column this extractor understands.
    Other,
}

fn norm(h: &str) -> String {
    h.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Removes the fiscal-year tokens, leaving whatever labels the column.
fn label_without_year(h: &str) -> String {
    let n = norm(h);
    let mut out: Vec<&str> = Vec::new();
    let toks: Vec<&str> = n.split_whitespace().collect();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if let Some(rest) = t.strip_prefix("fy") {
            if rest.is_empty() {
                // `fy 2026` — skip both tokens.
                if toks
                    .get(i + 1)
                    .is_some_and(|n| n.len() == 4 && n.chars().all(|c| c.is_ascii_digit()))
                {
                    i += 2;
                    continue;
                }
            } else if rest.len() == 4 && rest.chars().all(|c| c.is_ascii_digit()) {
                i += 1;
                continue;
            }
        }
        out.push(t);
        i += 1;
    }
    out.join(" ")
}

/// Classifies one header.
pub fn classify_header(h: &str) -> ColumnKind {
    let n = norm(h);
    match n.as_str() {
        "agency" | "agency code" | "agy" => return ColumnKind::Identity(Identity::Agency),
        "fund group" | "group" => return ColumnKind::Identity(Identity::FundGroup),
        "fund" | "fund code" => return ColumnKind::Identity(Identity::Fund),
        "ali" | "ali code" | "line item code" | "code" => {
            return ColumnKind::Identity(Identity::LineItemCode)
        }
        "ali name" | "line item name" | "line item" | "title" | "name" => {
            return ColumnKind::Identity(Identity::LineItemName)
        }
        _ => {}
    }

    let Some(fiscal_year) = fiscal_year_of_header(h) else {
        return ColumnKind::Other;
    };
    let label = label_without_year(h);

    if label.is_empty() {
        // A bare fiscal year in an as-enacted document is the completed year's outturn.
        return ColumnKind::Actual { fiscal_year };
    }
    if label.contains("estimate") || label.contains("projected") {
        return ColumnKind::Estimate {
            fiscal_year,
            issuer: label,
        };
    }
    match BillStage::parse_label(&label) {
        Some(stage) => ColumnKind::Appropriation { stage, fiscal_year },
        None => ColumnKind::UnknownStage { fiscal_year, label },
    }
}

/// A classified table.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnPlan {
    pub kinds: Vec<ColumnKind>,
}

impl ColumnPlan {
    pub fn of(headers: &[String]) -> Self {
        Self {
            kinds: headers.iter().map(|h| classify_header(h)).collect(),
        }
    }

    pub fn index_of(&self, id: Identity) -> Option<usize> {
        self.kinds
            .iter()
            .position(|k| matches!(k, ColumnKind::Identity(x) if *x == id))
    }

    /// Every (index, stage, fiscal year) appropriation column.
    pub fn appropriation_columns(&self) -> Vec<(usize, BillStage, &str)> {
        self.kinds
            .iter()
            .enumerate()
            .filter_map(|(i, k)| match k {
                ColumnKind::Appropriation { stage, fiscal_year } => {
                    Some((i, *stage, fiscal_year.as_str()))
                }
                _ => None,
            })
            .collect()
    }

    pub fn actual_columns(&self) -> Vec<(usize, &str)> {
        self.kinds
            .iter()
            .enumerate()
            .filter_map(|(i, k)| match k {
                ColumnKind::Actual { fiscal_year } => Some((i, fiscal_year.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Money columns that could not be classified. A caller that ignores these is dropping
    /// figures, so they are surfaced rather than filtered.
    pub fn unclassified(&self) -> Vec<(usize, &str, &str)> {
        self.kinds
            .iter()
            .enumerate()
            .filter_map(|(i, k)| match k {
                ColumnKind::UnknownStage { fiscal_year, label } => {
                    Some((i, fiscal_year.as_str(), label.as_str()))
                }
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_and_year_are_separated_from_one_header() {
        // The exact header shape that defeated map_columns.
        assert_eq!(
            classify_header("House Passed\r\nFY 2026"),
            ColumnKind::Appropriation {
                stage: BillStage::AsPassedHouse,
                fiscal_year: "FY2026".into()
            }
        );
        assert_eq!(
            classify_header("As Enacted\r\nFY 2027"),
            ColumnKind::Appropriation {
                stage: BillStage::AsEnacted,
                fiscal_year: "FY2027".into()
            }
        );
    }

    #[test]
    fn the_intermediate_stages_are_distinguished() {
        // Substitute, reported, and passed are different figures within one chamber.
        for (h, want) in [
            ("House Substitute\r\nFY 2026", BillStage::HouseSubstitute),
            ("House Reported\r\nFY 2026", BillStage::HouseReported),
            ("Senate Substitute\r\nFY 2026", BillStage::SenateSubstitute),
            ("Senate Reported\r\nFY 2026", BillStage::SenateReported),
            ("Conference Report\r\nFY 2026", BillStage::ConferenceReport),
        ] {
            assert_eq!(
                classify_header(h),
                ColumnKind::Appropriation {
                    stage: want,
                    fiscal_year: "FY2026".into()
                },
                "{h}"
            );
        }
    }

    #[test]
    fn a_bare_fiscal_year_is_an_actual_not_an_appropriation() {
        assert_eq!(
            classify_header("FY 2024"),
            ColumnKind::Actual {
                fiscal_year: "FY2024".into()
            }
        );
    }

    #[test]
    fn an_estimate_is_neither_an_appropriation_nor_an_actual() {
        // The most dangerous column: it looks like an appropriation and is a projection.
        match classify_header("OBM Estimate\r\nFY 2025") {
            ColumnKind::Estimate {
                fiscal_year,
                issuer,
            } => {
                assert_eq!(fiscal_year, "FY2025");
                assert!(issuer.contains("obm"), "{issuer}");
            }
            other => panic!("an estimate must not classify as {other:?}"),
        }
    }

    #[test]
    fn an_unrecognised_stage_is_reported_rather_than_guessed() {
        match classify_header("Governor Recommended\r\nFY 2026") {
            ColumnKind::UnknownStage { fiscal_year, label } => {
                assert_eq!(fiscal_year, "FY2026");
                assert_eq!(label, "governor recommended");
            }
            other => panic!("expected UnknownStage, got {other:?}"),
        }
    }

    #[test]
    fn identity_columns_are_recognised() {
        let headers: Vec<String> = ["Agency", "Fund Group", "Fund", "ALI", "ALI Name"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plan = ColumnPlan::of(&headers);
        assert_eq!(plan.index_of(Identity::Agency), Some(0));
        assert_eq!(plan.index_of(Identity::LineItemCode), Some(3));
        assert_eq!(plan.index_of(Identity::LineItemName), Some(4));
    }

    /// The real workbook's header row, verbatim.
    fn real_headers() -> Vec<String> {
        [
            "Agency",
            "Fund Group",
            "Fund",
            "ALI",
            "ALI Name",
            "FY 2024",
            "OBM Estimate\r\nFY 2025",
            "Introduced\r\nFY 2026",
            "Introduced\r\nFY 2027",
            "House Substitute\r\nFY 2026",
            "House Substitute\r\nFY 2027",
            "House Reported\r\nFY 2026",
            "House Reported\r\nFY 2027",
            "House Passed\r\nFY 2026",
            "House Passed\r\nFY 2027",
            "Senate Substitute\r\nFY 2026",
            "Senate Substitute\r\nFY 2027",
            "Senate Reported\r\nFY 2026",
            "Senate Reported\r\nFY 2027",
            "Senate Passed\r\nFY 2026",
            "Senate Passed\r\nFY 2027",
            "Conference Report\r\nFY 2026",
            "Conference Report\r\nFY 2027",
            "As Enacted\r\nFY 2026",
            "As Enacted\r\nFY 2027",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn the_real_workbook_header_row_classifies_completely() {
        let plan = ColumnPlan::of(&real_headers());
        assert_eq!(
            plan.appropriation_columns().len(),
            18,
            "nine stages times two years"
        );
        assert_eq!(plan.actual_columns().len(), 1, "FY2024 only");
        assert!(plan.unclassified().is_empty(), "{:?}", plan.unclassified());
        assert_eq!(
            plan.kinds
                .iter()
                .filter(|k| matches!(k, ColumnKind::Estimate { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn every_appropriation_column_keeps_its_own_stage() {
        // The regression this module exists for: map_columns collapsed these to five
        // identical FY2026 columns and lost the stage entirely.
        let plan = ColumnPlan::of(&real_headers());
        let fy26: Vec<BillStage> = plan
            .appropriation_columns()
            .into_iter()
            .filter(|(_, _, fy)| *fy == "FY2026")
            .map(|(_, s, _)| s)
            .collect();
        assert_eq!(fy26.len(), 9);
        let mut uniq = fy26.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), 9, "stages must stay distinct, got {fy26:?}");
    }
}
