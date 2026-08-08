//! Decomposes an appropriation's movement across bill stages and attributes each step.
//!
//! Serves the repository's priority-2 question: how the numbers got that way. The corpus
//! stores a figure per stage rather than a chain of deltas precisely so this is a difference
//! between checkpoints rather than a replay.
//!
//! # What it refuses to do
//!
//! **It does not distribute movement across a gap.** Where stages are missing between two
//! present ones, the delta covers everything that happened in between and is reported as
//! unattributed. Splitting it would invent a decision.
//!
//! **It does not let a veto increase anything.** Ohio's governor may strike items but not add
//! or raise them, so an increase into the enacted stage attributed to a veto is a
//! contradiction. Rather than report it, this flags a missing version — because that is the
//! likelier explanation and the one a reader should check.

use anyhow::Result;
use corpus_schema::BillStage;
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

fn links_to(from: &LoadedInstance, rel: &str, target: &LoadedInstance) -> bool {
    let dir = from.abs_path.parent().unwrap_or(&from.abs_path);
    from.inst
        .links
        .iter()
        .any(|l| l.relationship == rel && normalize_join(dir, &l.target) == target.abs_path)
}

fn class<'a>(c: &'a Corpus, k: &'a str) -> impl Iterator<Item = &'a LoadedInstance> + 'a {
    c.instances.iter().filter(move |i| i.inst.class == k)
}

/// An action that moved a figure, with what its taker said about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribution {
    pub action: String,
    pub action_type: String,
    pub actor: Option<String>,
    /// What the actor said, not why they did it. Empty where the source records no reason —
    /// which is normal at the conference stage and is a property of the process, not a gap.
    pub stated_justification: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub from: BillStage,
    pub to: BillStage,
    pub from_cents: i64,
    pub to_cents: i64,
    pub delta_cents: i64,
    /// Stages absent between `from` and `to`. Non-empty means the delta is aggregate.
    pub skipped: Vec<BillStage>,
    pub attributions: Vec<Attribution>,
    /// Set when the movement contradicts what the acting power can do.
    pub anomaly: Option<String>,
}

impl Step {
    pub fn is_attributed(&self) -> bool {
        self.skipped.is_empty() && !self.attributions.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Decomposition {
    pub line_item: String,
    pub period: String,
    pub steps: Vec<Step>,
    /// Stages the corpus holds no amount for.
    pub missing_stages: Vec<BillStage>,
    pub net_cents: i64,
}

impl Decomposition {
    /// True when every stage in the sequence carries a figure, so no delta is aggregate.
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| s.skipped.is_empty())
    }
}

fn amount_of(inst: &LoadedInstance) -> Option<i64> {
    let raw = prop(inst, "amount")?;
    if raw.contains("[open]") {
        return None;
    }
    lsc::parse_money_to_cents(raw).ok()
}

/// Decomposes movement for one line item in one period.
pub fn decompose(corpus: &Corpus, line_item_slug: &str, period: &str) -> Decomposition {
    let mut out = Decomposition {
        line_item: line_item_slug.to_string(),
        period: period.to_string(),
        ..Default::default()
    };

    let Some(li) = class(corpus, "line-item").find(|i| slug_of(&i.rel_path) == line_item_slug)
    else {
        return out;
    };

    // One amount per stage, in the order a bill passes through them.
    let mut points: Vec<(BillStage, i64, &LoadedInstance)> = Vec::new();
    for stage in BillStage::sequence() {
        let found = class(corpus, "appropriation").find(|a| {
            links_to(a, "grants-authority-for", li)
                && prop(a, "period_label").map(str::trim) == Some(period)
                && prop(a, "stage").map(str::trim) == Some(stage.as_str())
        });
        match found.and_then(|a| amount_of(a).map(|c| (a, c))) {
            Some((a, cents)) => points.push((stage, cents, a)),
            None => out.missing_stages.push(stage),
        }
    }

    if points.len() < 2 {
        return out;
    }
    out.net_cents = points.last().unwrap().1 - points[0].1;

    let seq = BillStage::sequence();
    for w in points.windows(2) {
        let (from, from_cents, _) = w[0];
        let (to, to_cents, to_node) = w[1];
        let fi = seq.iter().position(|s| *s == from).unwrap();
        let ti = seq.iter().position(|s| *s == to).unwrap();
        let skipped: Vec<BillStage> = seq[fi + 1..ti].to_vec();

        // Actions that operate on the later checkpoint, or on the earlier figure.
        //
        // The version is resolved through the appropriation's own `stated-in` edge, not by
        // looking for any bill-version carrying the same stage name. Matching on stage alone
        // attributes every bill's as-enacted actions to every other bill's as-enacted step —
        // which is exactly what the first run against the real corpus did, crediting HB 96's
        // final step with vetoes from HB 110 and HB 166 and a reduction from HB 153.
        let to_version =
            class(corpus, "bill-version").find(|bv| links_to(to_node, "stated-in", bv));
        let mut attributions = Vec::new();
        for act in class(corpus, "budget-action") {
            let applies = to_version.is_some_and(|bv| links_to(act, "applies-to", bv));
            let modifies = links_to(act, "modifies", to_node);
            if !applies && !modifies {
                continue;
            }
            let actor = class(corpus, "actor")
                .find(|a| links_to(act, "taken-by", a))
                .map(|a| slug_of(&a.rel_path).to_string());
            attributions.push(Attribution {
                action: slug_of(&act.rel_path).to_string(),
                action_type: prop(act, "action_type").unwrap_or("").to_string(),
                actor,
                stated_justification: prop(act, "stated_justification")
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            });
        }

        let delta = to_cents - from_cents;
        let veto = attributions
            .iter()
            .any(|a| a.action_type == "line-item-veto");
        let anomaly = if veto && delta > 0 {
            Some(
                "a line-item veto cannot increase an appropriation, so this rise is not \
                 attributable to it — a bill version between these two is probably missing"
                    .to_string(),
            )
        } else {
            None
        };

        out.steps.push(Step {
            from,
            to,
            from_cents,
            to_cents,
            delta_cents: delta,
            skipped,
            attributions,
            anomaly,
        });
    }
    out
}

pub fn render(d: &Decomposition) -> String {
    if d.steps.is_empty() {
        return format!(
            "{} {}: fewer than two stages carry an amount; nothing to decompose\n",
            d.line_item, d.period
        );
    }
    let mut s = format!(
        "{} {} — net {} cents across {} step(s){}\n",
        d.line_item,
        d.period,
        d.net_cents,
        d.steps.len(),
        if d.is_complete() {
            ", chain complete"
        } else {
            ", chain incomplete"
        }
    );
    for st in &d.steps {
        s.push_str(&format!(
            "\n  {:>18} -> {:<18} {:>+16} cents\n",
            st.from.as_str(),
            st.to.as_str(),
            st.delta_cents
        ));
        if !st.skipped.is_empty() {
            let names: Vec<&str> = st.skipped.iter().map(|x| x.as_str()).collect();
            s.push_str(&format!(
                "      AGGREGATE: covers {} — not attributable to any one actor\n",
                names.join(", ")
            ));
        }
        for a in &st.attributions {
            s.push_str(&format!(
                "      {} ({}){}\n",
                a.action,
                a.action_type,
                a.actor
                    .as_ref()
                    .map(|x| format!(" by {x}"))
                    .unwrap_or_default()
            ));
            if !a.stated_justification.is_empty() {
                s.push_str(&format!("        said: {}\n", a.stated_justification));
            }
        }
        if st.attributions.is_empty() && st.skipped.is_empty() {
            s.push_str("      no action recorded for this step\n");
        }
        if let Some(x) = &st.anomaly {
            s.push_str(&format!("      ANOMALY: {x}\n"));
        }
    }
    if !d.missing_stages.is_empty() {
        let names: Vec<&str> = d.missing_stages.iter().map(|x| x.as_str()).collect();
        s.push_str(&format!("\n  no amount for: {}\n", names.join(", ")));
    }
    s
}

/// Every (line item, period) the corpus holds two or more staged amounts for.
pub fn decomposable(corpus: &Corpus) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for a in class(corpus, "appropriation") {
        let Some(period) = prop(a, "period_label") else {
            continue;
        };
        let dir = a.abs_path.parent().unwrap_or(&a.abs_path);
        for l in a
            .inst
            .links
            .iter()
            .filter(|l| l.relationship == "grants-authority-for")
        {
            let t = normalize_join(dir, &l.target);
            if let Some(n) = t.file_stem().and_then(|s| s.to_str()) {
                pairs.push((n.to_string(), period.trim().to_string()));
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
        .into_iter()
        .filter(|(li, p)| !decompose(corpus, li, p).steps.is_empty())
        .collect()
}

// ─── aggregate movement across a whole extraction ────────────────────────────

/// How much money moved at one transition, across every line item in a document.
#[derive(Debug, Clone, PartialEq)]
pub struct StageMovement {
    pub from: BillStage,
    pub to: BillStage,
    /// Line items whose figure changed at this transition.
    pub line_items_moved: usize,
    /// Sum of absolute movement. This is the measure of *activity* — a chamber that adds a
    /// billion to one line and removes a billion from another has done a great deal, and a
    /// net figure would report zero.
    pub gross_cents: i64,
    /// Signed sum. The measure of *direction*.
    pub net_cents: i64,
    pub largest_increase: Option<(String, i64)>,
    pub largest_decrease: Option<(String, i64)>,
}

/// Aggregates stage-to-stage movement across every line item in an extraction.
///
/// Answers at full scale what a single line item can only suggest: which step of the process
/// actually moves money. Gross and net are both reported because they answer different
/// questions and a chamber can be extremely active while netting to nothing.
pub fn aggregate(
    rows: &[corpus_schema::LscComparisonRow],
    fiscal_year: &str,
) -> Vec<StageMovement> {
    use std::collections::BTreeMap;

    // line item -> stage -> amount
    let mut by_item: BTreeMap<&str, BTreeMap<BillStage, i64>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.fiscal_year == fiscal_year) {
        by_item
            .entry(r.line_item_code.as_str())
            .or_default()
            .insert(r.stage, r.amount_cents);
    }

    let seq = BillStage::sequence();
    let mut out = Vec::new();
    for w in seq.windows(2) {
        let (from, to) = (w[0], w[1]);
        let mut m = StageMovement {
            from,
            to,
            line_items_moved: 0,
            gross_cents: 0,
            net_cents: 0,
            largest_increase: None,
            largest_decrease: None,
        };
        for (code, stages) in &by_item {
            let (Some(a), Some(b)) = (stages.get(&from), stages.get(&to)) else {
                continue;
            };
            let d = b - a;
            if d == 0 {
                continue;
            }
            m.line_items_moved += 1;
            m.gross_cents += d.abs();
            m.net_cents += d;
            if d > 0 && m.largest_increase.as_ref().is_none_or(|(_, v)| d > *v) {
                m.largest_increase = Some((code.to_string(), d));
            }
            if d < 0 && m.largest_decrease.as_ref().is_none_or(|(_, v)| d < *v) {
                m.largest_decrease = Some((code.to_string(), d));
            }
        }
        out.push(m);
    }
    out
}

pub fn render_aggregate(m: &[StageMovement], fiscal_year: &str) -> String {
    let total: i64 = m.iter().map(|x| x.gross_cents).sum();
    let mut s = format!(
        "stage movement across all line items, {fiscal_year}\n\
         (gross = total absolute movement; net = direction)\n\n"
    );
    for x in m {
        let share = if total > 0 {
            x.gross_cents as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        s.push_str(&format!(
            "  {:>18} -> {:<18} {:>5} item(s)  gross {:>16}  net {:>+16}  {:>5.1}%\n",
            x.from.as_str(),
            x.to.as_str(),
            x.line_items_moved,
            x.gross_cents,
            x.net_cents,
            share
        ));
    }
    s
}

pub fn run(repo_root: &std::path::Path) -> Result<String> {
    let corpus = corpus_validate::load(repo_root)?;
    let mut out = String::new();
    let pairs = decomposable(&corpus);
    out.push_str(&format!("{} decomposable series\n", pairs.len()));
    for (li, p) in pairs {
        out.push('\n');
        out.push_str(&render(&decompose(&corpus, &li, &p)));
    }
    Ok(out)
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

    fn appr(slug: &str, stage: &str, amount: &str) -> LoadedInstance {
        inst(
            &format!("c/appropriation/{slug}.yml"),
            &format!(
                r#"
class: appropriation
label: A
description: d
properties:
  amount: "{amount}"
  period_label: FY2026
  stage: {stage}
links:
  - target: ../line-item/ff.yml
    relationship: grants-authority-for
"#
            ),
        )
    }

    /// An appropriation that names the bill version it was stated in.
    fn appr_in(slug: &str, stage: &str, amount: &str, version: &str) -> LoadedInstance {
        inst(
            &format!("c/appropriation/{slug}.yml"),
            &format!(
                r#"
class: appropriation
label: A
description: d
properties:
  amount: "{amount}"
  period_label: FY2026
  stage: {stage}
links:
  - target: ../line-item/ff.yml
    relationship: grants-authority-for
  - target: ../bill-version/{version}.yml
    relationship: stated-in
"#
            ),
        )
    }

    fn base() -> Vec<LoadedInstance> {
        vec![inst(
            "c/line-item/ff.yml",
            "class: line-item\nlabel: FF\ndescription: d\n",
        )]
    }

    fn row(code: &str, stage: BillStage, fy: &str, cents: i64) -> corpus_schema::LscComparisonRow {
        corpus_schema::LscComparisonRow {
            bill_number: "HB 96".into(),
            general_assembly: "136th".into(),
            stage,
            agency_code: "EDU".into(),
            line_item_code: code.into(),
            line_item_name: "x".into(),
            fund_group: "GRF".into(),
            fund_code: "GRF".into(),
            fiscal_year: fy.into(),
            amount_cents: cents,
            provenance: corpus_schema::Provenance {
                catalog_slug: "c".into(),
                document_ref: "d".into(),
                locator: None,
                retrieved: "2026-08-08".into(),
            },
        }
    }

    #[test]
    fn aggregate_separates_activity_from_direction() {
        // Two line items moving in opposite directions by the same amount: a great deal of
        // activity, and a net of zero. Reporting only net would say nothing happened.
        let rows = vec![
            row("a", BillStage::AsIntroduced, "FY2026", 100),
            row("a", BillStage::HouseSubstitute, "FY2026", 200),
            row("b", BillStage::AsIntroduced, "FY2026", 100),
            row("b", BillStage::HouseSubstitute, "FY2026", 0),
        ];
        let m = aggregate(&rows, "FY2026");
        let step = m
            .iter()
            .find(|x| x.to == BillStage::HouseSubstitute)
            .unwrap();
        assert_eq!(step.line_items_moved, 2);
        assert_eq!(step.gross_cents, 200);
        assert_eq!(step.net_cents, 0);
        assert_eq!(step.largest_increase.as_ref().unwrap().0, "a");
        assert_eq!(step.largest_decrease.as_ref().unwrap().0, "b");
    }

    #[test]
    fn aggregate_ignores_other_fiscal_years() {
        let rows = vec![
            row("a", BillStage::AsIntroduced, "FY2026", 100),
            row("a", BillStage::HouseSubstitute, "FY2026", 200),
            row("a", BillStage::AsIntroduced, "FY2027", 500),
            row("a", BillStage::HouseSubstitute, "FY2027", 900),
        ];
        let m = aggregate(&rows, "FY2026");
        let step = m
            .iter()
            .find(|x| x.to == BillStage::HouseSubstitute)
            .unwrap();
        assert_eq!(step.gross_cents, 100);
    }

    #[test]
    fn an_unchanged_line_item_is_not_counted_as_movement() {
        let rows = vec![
            row("a", BillStage::AsIntroduced, "FY2026", 100),
            row("a", BillStage::HouseSubstitute, "FY2026", 100),
        ];
        let m = aggregate(&rows, "FY2026");
        let step = m
            .iter()
            .find(|x| x.to == BillStage::HouseSubstitute)
            .unwrap();
        assert_eq!(step.line_items_moved, 0);
        assert_eq!(step.gross_cents, 0);
    }

    #[test]
    fn consecutive_stages_produce_signed_deltas() {
        let mut v = base();
        v.push(appr("a", "as-introduced", "$100.00"));
        v.push(appr("b", "as-passed-house", "$150.00"));
        v.push(appr("c", "as-enacted", "$120.00"));
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        assert_eq!(d.steps.len(), 2);
        assert_eq!(d.steps[0].delta_cents, 5000);
        assert_eq!(d.steps[1].delta_cents, -3000);
        assert_eq!(d.net_cents, 2000);
    }

    #[test]
    fn a_gap_is_reported_as_aggregate_rather_than_split() {
        // Introduced straight to enacted covers seven intervening stages. Distributing that
        // movement across them would invent decisions nobody made.
        let mut v = base();
        v.push(appr("a", "as-introduced", "$100.00"));
        v.push(appr("b", "as-enacted", "$200.00"));
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        assert_eq!(d.steps.len(), 1);
        assert_eq!(d.steps[0].skipped.len(), 7);
        assert!(!d.steps[0].is_attributed());
        assert!(!d.is_complete());
        assert!(render(&d).contains("AGGREGATE"));
    }

    #[test]
    fn a_complete_chain_reports_itself_complete() {
        let mut v = base();
        for (slug, stage) in [
            ("a", "as-introduced"),
            ("b", "house-substitute"),
            ("c", "house-reported"),
            ("d", "as-passed-house"),
            ("e", "senate-substitute"),
            ("f", "senate-reported"),
            ("g", "as-passed-senate"),
            ("h", "conference-report"),
            ("i", "as-enacted"),
        ] {
            v.push(appr(slug, stage, "$100.00"));
        }
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        assert_eq!(d.steps.len(), 8);
        assert!(d.is_complete());
        assert!(d.steps.iter().all(|s| s.skipped.is_empty()));
    }

    #[test]
    fn an_action_on_the_later_version_attributes_the_step() {
        let mut v = base();
        v.push(appr("a", "as-introduced", "$100.00"));
        v.push(appr_in("b", "as-passed-house", "$150.00", "hb-house"));
        v.push(inst(
            "c/bill-version/hb-house.yml",
            "class: bill-version\nlabel: H\ndescription: d\nproperties:\n  stage: as-passed-house\n",
        ));
        v.push(inst(
            "c/actor/house.yml",
            "class: actor\nlabel: House\ndescription: d\n",
        ));
        v.push(inst(
            "c/budget-action/amend.yml",
            r#"
class: budget-action
label: Amendment
description: d
properties:
  action_type: amendment
  stated_justification: |
    Increase to reflect the revised base cost.
links:
  - target: ../bill-version/hb-house.yml
    relationship: applies-to
  - target: ../actor/house.yml
    relationship: taken-by
"#,
        ));
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        // introduced -> house skips two intermediate stages, so it is aggregate, but the
        // action is still named.
        let step = &d.steps[0];
        assert_eq!(step.attributions.len(), 1);
        assert_eq!(step.attributions[0].actor.as_deref(), Some("house"));
        assert!(step.attributions[0]
            .stated_justification
            .contains("revised base cost"));
    }

    #[test]
    fn actions_on_another_bill_are_not_attributed() {
        // Regression: matching on stage name alone credited HB 96's as-enacted step with
        // vetoes belonging to HB 110 and HB 166, because all three have an as-enacted stage.
        let mut v = base();
        v.push(inst(
            "c/appropriation/a.yml",
            r#"
class: appropriation
label: A
description: d
properties:
  amount: "$100.00"
  period_label: FY2026
  stage: conference-report
links:
  - target: ../line-item/ff.yml
    relationship: grants-authority-for
"#,
        ));
        v.push(inst(
            "c/appropriation/b.yml",
            r#"
class: appropriation
label: B
description: d
properties:
  amount: "$90.00"
  period_label: FY2026
  stage: as-enacted
links:
  - target: ../line-item/ff.yml
    relationship: grants-authority-for
  - target: ../bill-version/ours-enacted.yml
    relationship: stated-in
"#,
        ));
        for slug in ["ours-enacted", "theirs-enacted"] {
            v.push(inst(
                &format!("c/bill-version/{slug}.yml"),
                "class: bill-version\nlabel: V\ndescription: d\nproperties:\n  stage: as-enacted\n",
            ));
        }
        for (slug, target) in [
            ("ours-veto", "ours-enacted"),
            ("theirs-veto", "theirs-enacted"),
        ] {
            v.push(inst(
                &format!("c/budget-action/{slug}.yml"),
                &format!(
                    "class: budget-action\nlabel: V\ndescription: d\nproperties:\n  action_type: line-item-veto\nlinks:\n  - target: ../bill-version/{target}.yml\n    relationship: applies-to\n"
                ),
            ));
        }
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        let names: Vec<&str> = d.steps[0]
            .attributions
            .iter()
            .map(|a| a.action.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["ours-veto"],
            "another bill's action was attributed: {names:?}"
        );
    }

    #[test]
    fn a_veto_cannot_explain_an_increase() {
        // Ohio's governor may strike but not raise. An increase attributed to a veto means a
        // version is missing, not that the executive added money.
        let mut v = base();
        v.push(appr("a", "conference-report", "$100.00"));
        v.push(appr_in("b", "as-enacted", "$150.00", "en"));
        v.push(inst(
            "c/bill-version/en.yml",
            "class: bill-version\nlabel: E\ndescription: d\nproperties:\n  stage: as-enacted\n",
        ));
        v.push(inst(
            "c/budget-action/veto.yml",
            r#"
class: budget-action
label: Veto
description: d
properties:
  action_type: line-item-veto
links:
  - target: ../bill-version/en.yml
    relationship: applies-to
"#,
        ));
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        let a = d.steps[0].anomaly.as_deref().unwrap_or_default();
        assert!(a.contains("cannot increase"), "{a}");
    }

    #[test]
    fn a_veto_reducing_a_figure_is_not_anomalous() {
        let mut v = base();
        v.push(appr("a", "conference-report", "$150.00"));
        v.push(appr_in("b", "as-enacted", "$100.00", "en"));
        v.push(inst(
            "c/bill-version/en.yml",
            "class: bill-version\nlabel: E\ndescription: d\nproperties:\n  stage: as-enacted\n",
        ));
        v.push(inst(
            "c/budget-action/veto.yml",
            "class: budget-action\nlabel: V\ndescription: d\nproperties:\n  action_type: line-item-veto\nlinks:\n  - target: ../bill-version/en.yml\n    relationship: applies-to\n",
        ));
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        assert!(d.steps[0].anomaly.is_none());
    }

    #[test]
    fn an_open_amount_leaves_the_stage_missing() {
        let mut v = base();
        v.push(appr("a", "as-introduced", "$100.00"));
        v.push(appr(
            "b",
            "as-passed-house",
            "[open] pending the lsc connector",
        ));
        v.push(appr("c", "as-enacted", "$200.00"));
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        assert!(d.missing_stages.contains(&BillStage::AsPassedHouse));
        assert_eq!(d.steps.len(), 1, "only the two known points form a step");
    }

    #[test]
    fn one_stage_alone_decomposes_to_nothing() {
        let mut v = base();
        v.push(appr("a", "as-enacted", "$100.00"));
        let d = decompose(
            &Corpus {
                instances: v,
                ..Default::default()
            },
            "ff",
            "FY2026",
        );
        assert!(d.steps.is_empty());
        assert!(render(&d).contains("nothing to decompose"));
    }
}
