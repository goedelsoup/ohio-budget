//! Proposes corpus updates from validated connector extraction records.
//!
//! This closes the loop the repository was missing: fixtures and connectors produce typed
//! records, and something has to decide which corpus node each record belongs to and what it
//! would change. That decision is made here, and it is **only proposed** — nothing in this
//! crate writes to the corpus.
//!
//! Three refusals are built in, each guarding a distinct way an extraction run could quietly
//! corrupt the corpus:
//!
//! 1. **Synthetic provenance is refused outright.** Fixture values are fabricated. One
//!    reaching a corpus node would be a made-up figure indistinguishable downstream from a
//!    real one.
//! 2. **A match across a line item's historical code is flagged, never silent.** It means the
//!    promotion crosses a renumbering, and asserting continuity is a human judgment.
//! 3. **A figure whose catalog entry has uncommitted content cannot be marked verified.**
//!    Registering a source is not the same as committing it, so such a proposal fills the
//!    amount and leaves the claim at `[inference]`.

use corpus_schema::{BillStage, CatalogEntry, LscComparisonRow};
use corpus_validate::{normalize_join, Corpus, LoadedInstance, FIXTURE_MARKER};

/// Corpus spelling of a bill stage.
///
/// Delegates to the schema crate so the vocabulary lives in exactly one place — when four
/// stages were added after reading the real source, this was one of two call sites the
/// compiler flagged rather than one of two that silently disagreed.
pub fn stage_str(s: BillStage) -> &'static str {
    s.as_str()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// The row's code is the line item's current code.
    CurrentCode,
    /// The row's code appears in the line item's historical codes. The promotion crosses a
    /// renumbering and asserts continuity that a person must confirm.
    HistoricalCode,
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub node_path: String,
    pub line_item_path: String,
    pub current_amount: String,
    pub proposed_cents: i64,
    pub proposed_display: String,
    pub catalog_slug: String,
    /// False when the cited catalog entry's content is not committed. Such a figure may be
    /// filled but its claim must stay `[inference]`.
    pub may_mark_verified: bool,
    pub matched_via: MatchKind,
}

#[derive(Debug, Clone)]
pub struct Refusal {
    pub line_item_code: String,
    pub fiscal_year: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct UnmatchedRow {
    pub line_item_code: String,
    pub line_item_name: String,
    pub fiscal_year: String,
    pub stage: &'static str,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct PromotionReport {
    pub proposals: Vec<Proposal>,
    pub refused: Vec<Refusal>,
    pub unmatched_rows: Vec<UnmatchedRow>,
    /// Appropriation nodes still carrying an `[open]` amount after this run.
    pub still_open: Vec<String>,
}

impl PromotionReport {
    pub fn needs_human_review(&self) -> Vec<&Proposal> {
        self.proposals
            .iter()
            .filter(|p| p.matched_via == MatchKind::HistoricalCode)
            .collect()
    }
}

fn prop<'a>(inst: &'a LoadedInstance, key: &str) -> Option<&'a str> {
    corpus_validate::property_text(&inst.inst, key)
}

fn instances_of<'a>(
    corpus: &'a Corpus,
    class: &'a str,
) -> impl Iterator<Item = &'a LoadedInstance> + 'a {
    corpus
        .instances
        .iter()
        .filter(move |i| i.inst.class == class)
}

/// Finds the line item a code refers to, and how.
fn match_line_item<'a>(corpus: &'a Corpus, code: &str) -> Option<(&'a LoadedInstance, MatchKind)> {
    let code = code.trim();
    if code.is_empty() {
        return None;
    }
    let mut historical: Option<&LoadedInstance> = None;
    for li in instances_of(corpus, "line-item") {
        if let Some(current) = prop(li, "current_code") {
            if current.trim() == code {
                return Some((li, MatchKind::CurrentCode));
            }
        }
        if let Some(hist) = prop(li, "historical_codes") {
            // Whole-token match so that "200" does not match inside "200550".
            if hist
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|t| t == code)
            {
                historical = Some(li);
            }
        }
    }
    historical.map(|li| (li, MatchKind::HistoricalCode))
}

fn catalog_entry<'a>(corpus: &'a Corpus, slug: &str) -> Option<&'a CatalogEntry> {
    corpus
        .catalog
        .iter()
        .find(|c| c.file_slug == slug)
        .and_then(|c| c.parsed.as_ref().ok())
}

/// True when an appropriation node grants authority for the given line item.
fn grants_for(appr: &LoadedInstance, line_item: &LoadedInstance) -> bool {
    let dir = appr.abs_path.parent().unwrap_or(&appr.abs_path);
    appr.inst.links.iter().any(|l| {
        l.relationship == "grants-authority-for"
            && normalize_join(dir, &l.target) == line_item.abs_path
    })
}

/// Matches extraction records against the corpus and proposes what each would change.
pub fn propose_from_lsc(corpus: &Corpus, rows: &[LscComparisonRow]) -> PromotionReport {
    let mut report = PromotionReport::default();
    let mut filled: Vec<String> = Vec::new();

    for row in rows {
        // 1. Synthetic provenance never reaches the corpus.
        if row.provenance.catalog_slug == FIXTURE_MARKER {
            report.refused.push(Refusal {
                line_item_code: row.line_item_code.clone(),
                fiscal_year: row.fiscal_year.clone(),
                reason: format!(
                    "provenance is '{FIXTURE_MARKER}'; fixture values are fabricated and must \
                     never become corpus facts"
                ),
            });
            continue;
        }

        let Some((line_item, matched_via)) = match_line_item(corpus, &row.line_item_code) else {
            report.unmatched_rows.push(UnmatchedRow {
                line_item_code: row.line_item_code.clone(),
                line_item_name: row.line_item_name.clone(),
                fiscal_year: row.fiscal_year.clone(),
                stage: stage_str(row.stage),
                reason: "no line item carries this code, currently or historically".into(),
            });
            continue;
        };

        let want_stage = stage_str(row.stage);
        let target = instances_of(corpus, "appropriation").find(|a| {
            prop(a, "period_label").map(str::trim) == Some(row.fiscal_year.as_str())
                && prop(a, "stage").map(str::trim) == Some(want_stage)
                && grants_for(a, line_item)
        });

        let Some(appr) = target else {
            report.unmatched_rows.push(UnmatchedRow {
                line_item_code: row.line_item_code.clone(),
                line_item_name: row.line_item_name.clone(),
                fiscal_year: row.fiscal_year.clone(),
                stage: want_stage,
                reason: format!(
                    "line item {} exists but no appropriation node covers {} at stage {}",
                    line_item.rel_path, row.fiscal_year, want_stage
                ),
            });
            continue;
        };

        let may_mark_verified = catalog_entry(corpus, &row.provenance.catalog_slug)
            .map(|e| e.content_committed)
            .unwrap_or(false);

        filled.push(appr.rel_path.clone());
        report.proposals.push(Proposal {
            node_path: appr.rel_path.clone(),
            line_item_path: line_item.rel_path.clone(),
            current_amount: prop(appr, "amount").unwrap_or("").to_string(),
            proposed_cents: row.amount_cents,
            proposed_display: lsc::format_cents(row.amount_cents),
            catalog_slug: row.provenance.catalog_slug.clone(),
            may_mark_verified,
            matched_via,
        });
    }

    report.still_open = instances_of(corpus, "appropriation")
        .filter(|a| prop(a, "amount").is_some_and(|v| v.contains("[open]")))
        .filter(|a| !filled.contains(&a.rel_path))
        .map(|a| a.rel_path.clone())
        .collect();
    report.still_open.sort();

    report
}

/// Renders a report for a human to read before anything is written.
pub fn render(report: &PromotionReport) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{} proposal(s), {} refused, {} unmatched row(s), {} appropriation(s) still open\n",
        report.proposals.len(),
        report.refused.len(),
        report.unmatched_rows.len(),
        report.still_open.len()
    ));
    for p in &report.proposals {
        s.push_str(&format!(
            "\n  {} \n    amount: {:?} -> {} ({} cents)\n    source: {} [{}]\n",
            p.node_path,
            p.current_amount,
            p.proposed_display,
            p.proposed_cents,
            p.catalog_slug,
            if p.may_mark_verified {
                "content committed — may be marked [verified]"
            } else {
                "content NOT committed — claim stays [inference]"
            },
        ));
        if p.matched_via == MatchKind::HistoricalCode {
            s.push_str(
                "    REVIEW: matched a historical code. This promotion crosses a renumbering \
                 and asserts continuity.\n",
            );
        }
    }
    for r in &report.refused {
        s.push_str(&format!(
            "\n  REFUSED {} {} — {}\n",
            r.line_item_code, r.fiscal_year, r.reason
        ));
    }
    for u in &report.unmatched_rows {
        s.push_str(&format!(
            "\n  UNMATCHED {} {} {} {} — {}\n",
            u.line_item_code, u.line_item_name, u.fiscal_year, u.stage, u.reason
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpus_schema::{CorpusInstance, Provenance, SourceType};
    use corpus_validate::LoadedCatalog;
    use std::path::PathBuf;

    fn inst(rel: &str, yaml: &str) -> LoadedInstance {
        LoadedInstance {
            rel_path: rel.to_string(),
            abs_path: PathBuf::from(rel),
            inst: serde_yaml::from_str::<CorpusInstance>(yaml).unwrap(),
        }
    }

    fn cat(slug: &str, committed: bool) -> LoadedCatalog {
        LoadedCatalog {
            rel_path: format!("catalog/{slug}.md"),
            abs_path: PathBuf::from(format!("catalog/{slug}.md")),
            file_slug: slug.to_string(),
            parsed: Ok(CatalogEntry {
                slug: slug.to_string(),
                name: slug.to_string(),
                source_type: SourceType::LegislativeDocument,
                location: "x".into(),
                publisher: "x".into(),
                content_committed: committed,
                access_constraints: None,
                feeds: vec![],
            }),
        }
    }

    fn corpus() -> Corpus {
        Corpus {
            catalog: vec![
                cat("lsc-hb96-comparison", false),
                cat("lsc-committed", true),
            ],
            instances: vec![
                inst(
                    "corpus/line-item/foundation-funding.yml",
                    r#"
class: line-item
label: Foundation Funding
description: d
properties:
  current_code: "200550"
  historical_codes: "200502 in force through FY2013"
"#,
                ),
                inst(
                    "corpus/appropriation/ff-fy2026-as-enacted.yml",
                    r#"
class: appropriation
label: FF FY2026 enacted
description: d
properties:
  amount: "[open] pending the lsc connector"
  period_label: FY2026
  stage: as-enacted
links:
  - target: ../line-item/foundation-funding.yml
    relationship: grants-authority-for
"#,
                ),
                inst(
                    "corpus/appropriation/ff-fy2027-as-enacted.yml",
                    r#"
class: appropriation
label: FF FY2027 enacted
description: d
properties:
  amount: "[open] pending the lsc connector"
  period_label: FY2027
  stage: as-enacted
links:
  - target: ../line-item/foundation-funding.yml
    relationship: grants-authority-for
"#,
                ),
            ],
            ..Default::default()
        }
    }

    fn row(code: &str, fy: &str, stage: BillStage, cents: i64, slug: &str) -> LscComparisonRow {
        LscComparisonRow {
            bill_number: "HB 96".into(),
            general_assembly: "136th".into(),
            stage,
            agency_code: "200".into(),
            line_item_code: code.into(),
            line_item_name: "Foundation Funding".into(),
            fund_group: "GRF".into(),
            fund_code: "5000".into(),
            fiscal_year: fy.into(),
            amount_cents: cents,
            provenance: Provenance {
                catalog_slug: slug.into(),
                document_ref: "d".into(),
                locator: None,
                retrieved: "2026-08-08".into(),
            },
        }
    }

    #[test]
    fn synthetic_provenance_is_refused_before_anything_else() {
        let rows = vec![row(
            "200550",
            "FY2026",
            BillStage::AsEnacted,
            800_000_000_000,
            FIXTURE_MARKER,
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert!(
            r.proposals.is_empty(),
            "no fixture value may become a proposal"
        );
        assert_eq!(r.refused.len(), 1);
        assert!(r.refused[0].reason.contains("fabricated"));
    }

    #[test]
    fn a_matching_row_proposes_against_the_right_node() {
        let rows = vec![row(
            "200550",
            "FY2026",
            BillStage::AsEnacted,
            800_000_000_000,
            "lsc-hb96-comparison",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert_eq!(r.proposals.len(), 1);
        let p = &r.proposals[0];
        assert_eq!(p.node_path, "corpus/appropriation/ff-fy2026-as-enacted.yml");
        assert_eq!(p.proposed_display, "$8000000000.00");
        assert_eq!(p.matched_via, MatchKind::CurrentCode);
        // FY2027 was not filled, so it stays on the open list.
        assert_eq!(
            r.still_open,
            vec!["corpus/appropriation/ff-fy2027-as-enacted.yml"]
        );
    }

    #[test]
    fn an_uncommitted_source_fills_the_amount_but_forbids_verified() {
        let rows = vec![row(
            "200550",
            "FY2026",
            BillStage::AsEnacted,
            1,
            "lsc-hb96-comparison",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert!(
            !r.proposals[0].may_mark_verified,
            "registering a source is not committing it"
        );

        let rows = vec![row(
            "200550",
            "FY2026",
            BillStage::AsEnacted,
            1,
            "lsc-committed",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert!(r.proposals[0].may_mark_verified);
    }

    #[test]
    fn an_unknown_catalog_slug_forbids_verified() {
        let rows = vec![row(
            "200550",
            "FY2026",
            BillStage::AsEnacted,
            1,
            "never-registered",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert!(!r.proposals[0].may_mark_verified);
    }

    #[test]
    fn a_historical_code_match_is_flagged_for_review() {
        // The code was retired at a renumbering. Promoting through it asserts continuity.
        let rows = vec![row(
            "200502",
            "FY2026",
            BillStage::AsEnacted,
            1,
            "lsc-hb96-comparison",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert_eq!(r.proposals.len(), 1);
        assert_eq!(r.proposals[0].matched_via, MatchKind::HistoricalCode);
        assert_eq!(r.needs_human_review().len(), 1);
        assert!(render(&r).contains("crosses a renumbering"));
    }

    #[test]
    fn a_code_is_matched_whole_not_as_a_substring() {
        // "200" must not match inside "200550" or inside the historical code list.
        let rows = vec![row(
            "200",
            "FY2026",
            BillStage::AsEnacted,
            1,
            "lsc-hb96-comparison",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert!(r.proposals.is_empty());
        assert_eq!(r.unmatched_rows.len(), 1);
    }

    #[test]
    fn a_stage_mismatch_does_not_silently_fill_the_wrong_node() {
        let rows = vec![row(
            "200550",
            "FY2026",
            BillStage::AsIntroduced,
            1,
            "lsc-hb96-comparison",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert!(r.proposals.is_empty());
        assert!(r.unmatched_rows[0]
            .reason
            .contains("no appropriation node covers"));
    }

    #[test]
    fn a_fiscal_year_with_no_node_is_reported_not_dropped() {
        let rows = vec![row(
            "200550",
            "FY2030",
            BillStage::AsEnacted,
            1,
            "lsc-hb96-comparison",
        )];
        let r = propose_from_lsc(&corpus(), &rows);
        assert_eq!(r.unmatched_rows.len(), 1);
        assert_eq!(r.unmatched_rows[0].fiscal_year, "FY2030");
    }
}
