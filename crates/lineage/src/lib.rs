//! Proposes line item succession candidates across renumberings.
//!
//! The one calculator whose output is a **hypothesis rather than a value**, and the one whose
//! approval in [`proposals.yml`](../../.yidam/decisions/proposals.yml) carried a standing
//! condition: its output must never be written to `succeeds` or `superseded-by` edges without
//! a contributor confirming the pair.
//!
//! That condition is not caution for its own sake. The failure mode is specific and quiet: a
//! wrong lineage assertion fuses two unrelated funding histories into one series. Nothing
//! downstream detects it, every chart drawn from it looks reasonable, and the error is
//! indistinguishable from a real funding change. A missing lineage edge leaves an obvious
//! hole; a wrong one manufactures false continuity.
//!
//! So this crate ranks candidates, states the evidence for each, and stops.

use corpus_validate::{normalize_join, Corpus, LoadedInstance};

fn slug_of(rel: &str) -> &str {
    rel.rsplit('/')
        .next()
        .unwrap_or(rel)
        .trim_end_matches(".yml")
}

fn prop<'a>(i: &'a LoadedInstance, k: &str) -> Option<&'a str> {
    corpus_validate::property_text(&i.inst, k)
}

fn linked<'a>(from: &LoadedInstance, rel: &str, corpus: &'a Corpus) -> Vec<&'a LoadedInstance> {
    let dir = from.abs_path.parent().unwrap_or(&from.abs_path);
    from.inst
        .links
        .iter()
        .filter(|l| l.relationship == rel)
        .filter_map(|l| {
            let t = normalize_join(dir, &l.target);
            corpus.instances.iter().find(|i| i.abs_path == t)
        })
        .collect()
}

/// Normalizes a title for comparison: lowercase alphanumeric words, ordered.
fn tokens(s: &str) -> Vec<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(str::to_string)
        .collect()
}

/// Jaccard overlap of two token sets, in 0..=1.
fn title_similarity(a: &str, b: &str) -> f64 {
    let (ta, tb) = (tokens(a), tokens(b));
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    let shared = ta.iter().filter(|t| tb.contains(t)).count();
    let union = ta.len() + tb.len() - shared;
    if union == 0 {
        0.0
    } else {
        shared as f64 / union as f64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The corpus already asserts this pair. Reported so a run can distinguish confirming an
    /// existing edge from proposing a new one.
    AlreadyAsserted,
    /// Not yet in the corpus.
    Proposed,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub predecessor: String,
    pub successor: String,
    /// 0..=1. Deliberately not a probability — it ranks, it does not measure.
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub status: Status,
}

impl Candidate {
    /// Every candidate needs confirming. Named as a method so callers cannot forget.
    pub fn requires_human_confirmation(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub candidates: Vec<Candidate>,
    /// Line items marked superseded or retired with no successor proposed at all. These are
    /// the series that simply end, and they are worth surfacing separately: a break with no
    /// candidate is a different problem from a break with a doubtful one.
    pub unresolved: Vec<String>,
}

/// Ranks succession candidates across the corpus.
///
/// A null result is a correct answer. Where the evidence is thin the candidate is either
/// scored low or not produced, because an unresolved break is better than a confident guess.
pub fn propose(corpus: &Corpus) -> Report {
    let items: Vec<&LoadedInstance> = corpus
        .instances
        .iter()
        .filter(|i| i.inst.class == "line-item")
        .collect();

    let mut report = Report::default();

    for succ in &items {
        for pred in &items {
            if succ.abs_path == pred.abs_path {
                continue;
            }

            let mut evidence = Vec::new();
            let mut score = 0.0;

            // Already asserted? Report it, so confirming and proposing stay distinct.
            let asserted = linked(succ, "succeeds", corpus)
                .iter()
                .any(|t| t.abs_path == pred.abs_path);

            // A predecessor must actually have ended.
            let pred_status = prop(pred, "status").unwrap_or("");
            if !matches!(pred_status, "superseded" | "retired" | "restructured") {
                continue;
            }
            evidence.push(format!("predecessor status is '{pred_status}'"));
            score += 0.2;

            let sim = title_similarity(
                prop(succ, "name").unwrap_or(&succ.inst.label),
                prop(pred, "name").unwrap_or(&pred.inst.label),
            );
            if sim > 0.0 {
                evidence.push(format!("title overlap {:.0}%", sim * 100.0));
                score += sim * 0.4;
            }

            // An agency reorganization explains a renumbering, and is the strongest
            // structural signal available — but only for the code change, not for the
            // substance. The money may have been restructured at the same moment.
            let succ_agencies = linked(succ, "belongs-to", corpus);
            let pred_agencies = linked(pred, "belongs-to", corpus);
            let reorg = succ_agencies.iter().any(|sa| {
                linked(sa, "succeeds", corpus)
                    .iter()
                    .any(|prev| pred_agencies.iter().any(|pa| pa.abs_path == prev.abs_path))
            });
            if reorg {
                evidence.push(
                    "the holding agency succeeds the predecessor's agency, which explains a \
                     renumbering but not necessarily continuity of the money"
                        .into(),
                );
                score += 0.3;
            }

            // Periods must not overlap: one ends where the other begins.
            if let (Some(pl), Some(sf)) = (prop(pred, "last_period"), prop(succ, "first_period")) {
                if !pl.contains("[open]") && !sf.contains("[open]") {
                    evidence.push(format!("predecessor ends {pl}, successor begins {sf}"));
                    score += 0.1;
                }
            }

            if score < 0.35 && !asserted {
                continue;
            }

            report.candidates.push(Candidate {
                predecessor: slug_of(&pred.rel_path).to_string(),
                successor: slug_of(&succ.rel_path).to_string(),
                confidence: score.min(1.0),
                evidence,
                status: if asserted {
                    Status::AlreadyAsserted
                } else {
                    Status::Proposed
                },
            });
        }
    }

    report.candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap()
            .then(a.successor.cmp(&b.successor))
    });

    for item in &items {
        let status = prop(item, "status").unwrap_or("");
        if matches!(status, "superseded" | "retired") {
            let slug = slug_of(&item.rel_path);
            if !report.candidates.iter().any(|c| c.predecessor == slug) {
                report.unresolved.push(slug.to_string());
            }
        }
    }
    report.unresolved.sort();
    report
}

/// Renders a report for review. There is no apply path, by design.
pub fn render(r: &Report) -> String {
    let mut s = format!(
        "{} candidate(s), {} unresolved break(s)\n\nEvery candidate below requires human \
         confirmation before becoming an edge.\n",
        r.candidates.len(),
        r.unresolved.len()
    );
    for c in &r.candidates {
        s.push_str(&format!(
            "\n  [{:.2}] {} -> {} ({:?})\n",
            c.confidence, c.predecessor, c.successor, c.status
        ));
        for e in &c.evidence {
            s.push_str(&format!("      - {e}\n"));
        }
    }
    for u in &r.unresolved {
        s.push_str(&format!(
            "\n  UNRESOLVED {u} — ends with no successor proposed\n"
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpus_schema::CorpusInstance;
    use std::path::PathBuf;

    fn inst(rel: &str, yaml: &str) -> LoadedInstance {
        LoadedInstance {
            rel_path: rel.to_string(),
            abs_path: PathBuf::from(rel),
            inst: serde_yaml::from_str::<CorpusInstance>(yaml).unwrap(),
        }
    }

    fn medicaid_corpus(assert_edge: bool) -> Corpus {
        let succ_link = if assert_edge {
            "  - target: ./old-medicaid.yml\n    relationship: succeeds\n"
        } else {
            ""
        };
        Corpus {
            instances: vec![
                inst(
                    "c/agency/odjfs.yml",
                    "class: agency\nlabel: JFS\ndescription: d\n",
                ),
                inst(
                    "c/agency/odm.yml",
                    r#"
class: agency
label: ODM
description: d
links:
  - target: ./odjfs.yml
    relationship: succeeds
"#,
                ),
                inst(
                    "c/line-item/old-medicaid.yml",
                    r#"
class: line-item
label: Medicaid Services
description: d
properties:
  name: Medicaid Services
  status: superseded
  last_period: FY2013
links:
  - target: ../agency/odjfs.yml
    relationship: belongs-to
"#,
                ),
                inst(
                    "c/line-item/new-medicaid.yml",
                    &format!(
                        r#"
class: line-item
label: Medicaid Health Care Services
description: d
properties:
  name: Medicaid Health Care Services
  status: active
  first_period: FY2014
links:
  - target: ../agency/odm.yml
    relationship: belongs-to
{succ_link}"#
                    ),
                ),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn the_medicaid_separation_is_proposed_with_its_evidence() {
        let r = propose(&medicaid_corpus(false));
        let c = r
            .candidates
            .iter()
            .find(|c| c.successor == "new-medicaid")
            .expect("the corpus's worked lineage case must be proposed");
        assert_eq!(c.predecessor, "old-medicaid");
        assert_eq!(c.status, Status::Proposed);
        assert!(c.confidence > 0.5, "confidence {}", c.confidence);
        assert!(c
            .evidence
            .iter()
            .any(|e| e.contains("succeeds the predecessor's agency")));
        assert!(c.evidence.iter().any(|e| e.contains("title overlap")));
    }

    #[test]
    fn an_agency_reorganization_explains_the_code_not_the_money() {
        let r = propose(&medicaid_corpus(false));
        let c = &r.candidates[0];
        let e = c.evidence.join(" ");
        assert!(
            e.contains("not necessarily continuity of the money"),
            "the evidence must not overstate what a reorganization proves: {e}"
        );
    }

    #[test]
    fn an_already_asserted_pair_is_distinguished_from_a_proposal() {
        let r = propose(&medicaid_corpus(true));
        let c = r
            .candidates
            .iter()
            .find(|c| c.successor == "new-medicaid")
            .unwrap();
        assert_eq!(c.status, Status::AlreadyAsserted);
    }

    #[test]
    fn every_candidate_requires_confirmation() {
        let r = propose(&medicaid_corpus(false));
        assert!(!r.candidates.is_empty());
        assert!(r.candidates.iter().all(|c| c.requires_human_confirmation()));
        assert!(render(&r).contains("requires human confirmation"));
    }

    #[test]
    fn an_active_line_item_is_never_a_predecessor() {
        // A line item that has not ended cannot have been succeeded.
        let mut c = medicaid_corpus(false);
        for i in &mut c.instances {
            if i.rel_path.contains("old-medicaid") {
                i.inst = serde_yaml::from_str(
                    "class: line-item\nlabel: Medicaid Services\ndescription: d\nproperties:\n  status: active\n",
                )
                .unwrap();
            }
        }
        let r = propose(&c);
        assert!(r.candidates.is_empty(), "{:?}", r.candidates);
    }

    #[test]
    fn unrelated_line_items_produce_no_candidate() {
        let c = Corpus {
            instances: vec![
                inst(
                    "c/line-item/highway.yml",
                    "class: line-item\nlabel: Highway Construction\ndescription: d\nproperties:\n  name: Highway Construction\n  status: superseded\n",
                ),
                inst(
                    "c/line-item/library.yml",
                    "class: line-item\nlabel: Public Library Distribution\ndescription: d\nproperties:\n  name: Public Library Distribution\n  status: active\n",
                ),
            ],
            ..Default::default()
        };
        let r = propose(&c);
        assert!(
            r.candidates.is_empty(),
            "thin evidence must yield no candidate"
        );
        // The break is still surfaced, because a series that just ends is worth knowing about.
        assert_eq!(r.unresolved, vec!["highway"]);
    }

    #[test]
    fn a_break_with_no_candidate_is_reported_separately() {
        let c = Corpus {
            instances: vec![inst(
                "c/line-item/orphaned.yml",
                "class: line-item\nlabel: Something Retired\ndescription: d\nproperties:\n  name: Something Retired\n  status: retired\n",
            )],
            ..Default::default()
        };
        let r = propose(&c);
        assert_eq!(r.unresolved, vec!["orphaned"]);
        assert!(render(&r).contains("UNRESOLVED"));
    }
}
