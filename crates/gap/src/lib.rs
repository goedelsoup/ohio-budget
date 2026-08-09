//! Computes appropriated authority against actual disbursement.
//!
//! The repository's central calculation, and the reason `appropriation` and `expenditure` are
//! separate classes rather than one class with a `measure` discriminator.
//!
//! # Most of its work is refusing
//!
//! A gap is a subtraction, which is the easy part. The value is in declining the subtractions
//! that produce a number meaning nothing:
//!
//! - an `[open]` amount is **unavailable**, never zero;
//! - a district-level disbursement against a whole-line appropriation is **refused**, because
//!   it is wrong by orders of magnitude and the result looks entirely plausible;
//! - expenditures on mixed bases are **refused**, because in-year and closed-book figures for
//!   the same line and period routinely disagree;
//! - a cross-period comparison without a deflator is **refused**, per
//!   [`real_dollars`](../../real-dollars/).
//!
//! # The number does not interpret itself
//!
//! A gap on an entitlement-driven line is evidence about forecasting; on a formula line it is
//! evidence about the formula's inputs; on a discretionary line it may be evidence about
//! execution. The calculator returns the figure together with what the corpus knows about the
//! line item's character, and refuses to collapse that into a verdict.

use anyhow::Result;
use corpus_validate::{normalize_join, Corpus, LoadedInstance};
use real_dollars::Deflator;
use serde::Serialize;

fn slug_of(rel_path: &str) -> &str {
    rel_path
        .rsplit('/')
        .next()
        .unwrap_or(rel_path)
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

/// Parses a corpus `amount` property. `[open]` is absence, not zero.
fn amount_cents(raw: &str) -> AmountState {
    if raw.contains("[open]") {
        return AmountState::Open;
    }
    match lsc::parse_money_to_cents(raw) {
        Ok(c) => AmountState::Known(c),
        Err(e) => AmountState::Unparsable(e.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum AmountState {
    Known(i64),
    Open,
    Unparsable(String),
}

/// What the corpus knows about how a gap on this line should be read.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Character {
    /// Policy areas of the programs funding this line item.
    pub policy_areas: Vec<String>,
    /// True when at least one funding program distributes by formula.
    pub formula_driven: bool,
    /// True when at least one funding program declares federal participation.
    pub federally_matched: bool,
}

impl Character {
    pub fn how_to_read(&self) -> &'static str {
        match (self.formula_driven, self.federally_matched) {
            (_, true) => {
                "cost follows enrollment and match rates the state does not set; a divergence \
                 is evidence about forecasting, not spending discipline"
            }
            (true, false) => {
                "distribution follows a formula; a divergence is evidence about the formula's \
                 inputs rather than about execution"
            }
            (false, false) => {
                "no funding program declares a formula or federal match; a divergence is more \
                 likely to reflect execution, but confirm the line item's character first"
            }
        }
    }
}

/// Serializes a [`Character`] together with the reading it implies.
///
/// `how_to_read` is emitted rather than left for the consumer to re-derive: a downstream
/// reimplementation of the mapping is a second definition of it, and the two would drift.
fn character_with_reading<S: serde::Serializer>(c: &Character, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeStruct;
    let mut st = s.serialize_struct("Character", 4)?;
    st.serialize_field("policy_areas", &c.policy_areas)?;
    st.serialize_field("formula_driven", &c.formula_driven)?;
    st.serialize_field("federally_matched", &c.federally_matched)?;
    st.serialize_field("how_to_read", c.how_to_read())?;
    st.end()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GapResult {
    pub line_item: String,
    pub period: String,
    pub appropriated_cents: i64,
    pub spent_cents: i64,
    /// Positive when authority exceeded spending.
    pub variance_cents: i64,
    pub variance_pct: f64,
    pub reversion_cents: Option<i64>,
    pub basis: String,
    /// True when the figures rest on in-year reporting and the books will still move.
    pub provisional: bool,
    #[serde(serialize_with = "character_with_reading")]
    pub character: Character,
    /// Recipient-scoped expenditures excluded from this computation.
    ///
    /// A district-level disbursement is a slice of the line item and cannot be differenced
    /// against the whole appropriation. Where a whole-line figure also exists, the slice is set
    /// aside rather than allowed to block the comparison — but it is named here, because a
    /// reader who sees the corpus holds a Columbus figure and a total is entitled to know which
    /// one this result rests on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipient_slices_excluded: Vec<String>,
}

/// The three answers the calculator can give.
///
/// Serialized internally tagged, so a consumer reads `status` and gets either a figure or the
/// reason there is none. A refusal is content: it says the corpus holds both sides and the
/// subtraction was still declined, which is a different statement from having no data.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Outcome {
    Computed(Box<GapResult>),
    /// The corpus does not yet hold the figures.
    Unavailable {
        reason: String,
    },
    /// The comparison would produce a number that means nothing.
    Refused {
        reason: String,
    },
}

impl Outcome {
    pub fn is_computed(&self) -> bool {
        matches!(self, Outcome::Computed(_))
    }
}

fn class<'a>(c: &'a Corpus, k: &'a str) -> impl Iterator<Item = &'a LoadedInstance> + 'a {
    c.instances.iter().filter(move |i| i.inst.class == k)
}

fn character_of(corpus: &Corpus, line_item: &LoadedInstance) -> Character {
    let funders: Vec<&LoadedInstance> = class(corpus, "program")
        .filter(|p| links_to(p, "funded-by", line_item))
        .collect();
    Character {
        policy_areas: funders
            .iter()
            .filter_map(|p| prop(p, "policy_area").map(str::to_string))
            .collect(),
        formula_driven: funders.iter().any(|p| {
            prop(p, "formula_basis")
                .is_some_and(|f| !f.contains("Not formula") && !f.contains("[open]"))
        }),
        federally_matched: funders.iter().any(|p| {
            prop(p, "federal_participation")
                .is_some_and(|f| !f.trim_start().starts_with("None") && !f.contains("[open]"))
        }),
    }
}

/// Computes the gap for one line item in one period.
pub fn gap_for(corpus: &Corpus, line_item_slug: &str, period: &str) -> Outcome {
    let Some(li) = class(corpus, "line-item").find(|i| slug_of(&i.rel_path) == line_item_slug)
    else {
        return Outcome::Unavailable {
            reason: format!("no line item named {line_item_slug}"),
        };
    };

    let appropriations: Vec<&LoadedInstance> = class(corpus, "appropriation")
        .filter(|a| {
            links_to(a, "grants-authority-for", li)
                && prop(a, "period_label").map(str::trim) == Some(period)
                && prop(a, "stage").map(str::trim) == Some("as-enacted")
        })
        .collect();
    let expenditures: Vec<&LoadedInstance> = class(corpus, "expenditure")
        .filter(|e| {
            links_to(e, "disburses-against", li)
                && prop(e, "period_label").map(str::trim) == Some(period)
        })
        .collect();

    if appropriations.is_empty() || expenditures.is_empty() {
        return Outcome::Unavailable {
            reason: format!(
                "{line_item_slug} {period}: {} enacted appropriation(s) and {} expenditure(s); \
                 both sides are required",
                appropriations.len(),
                expenditures.len()
            ),
        };
    }

    // A district-level disbursement is a slice of the line item, not the line item.
    // Differencing it against the whole appropriation is wrong by orders of magnitude and the
    // result looks entirely reasonable, which is what makes it worth refusing.
    //
    // But a slice is only a reason to refuse when it is *all* the corpus has for the period.
    // Where a whole-line figure exists too, the slice is set aside and named on the result.
    // Treating it as a poison pill cost this calculator a computed answer it already had: the
    // Columbus disbursement and the whole-line FY2024 actual both landed in FY2024, and the
    // pair went from computed to refused without any figure changing.
    let (sliced, whole): (Vec<&&LoadedInstance>, Vec<&&LoadedInstance>) = expenditures
        .iter()
        .partition(|e| e.inst.links.iter().any(|l| l.relationship == "paid-to"));
    let recipient_slices_excluded: Vec<String> = sliced
        .iter()
        .map(|e| slug_of(&e.rel_path).to_string())
        .collect();

    if whole.is_empty() {
        return Outcome::Refused {
            reason: format!(
                "{} is scoped to one recipient; comparing a recipient's share against the whole \
                 line item's authority is wrong by orders of magnitude. Aggregate recipients \
                 first, or compare like to like at recipient level.",
                recipient_slices_excluded.join(", ")
            ),
        };
    }
    let expenditures: Vec<&LoadedInstance> = whole.into_iter().copied().collect();

    let mut bases: Vec<&str> = expenditures
        .iter()
        .filter_map(|e| prop(e, "basis").map(str::trim))
        .collect();
    bases.sort_unstable();
    bases.dedup();
    if bases.len() > 1 {
        return Outcome::Refused {
            reason: format!(
                "expenditures mix bases {bases:?}; in-year and closed-book figures for the same \
                 line and period routinely disagree and must not be summed"
            ),
        };
    }
    let basis = bases.first().copied().unwrap_or("[open]").to_string();

    let mut appropriated = 0i64;
    for a in &appropriations {
        match amount_cents(prop(a, "amount").unwrap_or("[open]")) {
            AmountState::Known(c) => appropriated += c,
            AmountState::Open => {
                return Outcome::Unavailable {
                    reason: format!("{} has no amount yet", slug_of(&a.rel_path)),
                }
            }
            AmountState::Unparsable(e) => {
                return Outcome::Refused {
                    reason: format!("{}: {e}", slug_of(&a.rel_path)),
                }
            }
        }
    }

    let mut spent = 0i64;
    let mut reversion: Option<i64> = None;
    for e in &expenditures {
        match amount_cents(prop(e, "amount").unwrap_or("[open]")) {
            AmountState::Known(c) => spent += c,
            AmountState::Open => {
                return Outcome::Unavailable {
                    reason: format!("{} has no amount yet", slug_of(&e.rel_path)),
                }
            }
            AmountState::Unparsable(err) => {
                return Outcome::Refused {
                    reason: format!("{}: {err}", slug_of(&e.rel_path)),
                }
            }
        }
        if let AmountState::Known(r) = amount_cents(prop(e, "reversion").unwrap_or("[open]")) {
            reversion = Some(reversion.unwrap_or(0) + r);
        }
    }

    let variance = appropriated - spent;
    Outcome::Computed(Box::new(GapResult {
        line_item: line_item_slug.to_string(),
        period: period.to_string(),
        appropriated_cents: appropriated,
        spent_cents: spent,
        variance_cents: variance,
        variance_pct: if appropriated != 0 {
            variance as f64 / appropriated as f64 * 100.0
        } else {
            0.0
        },
        reversion_cents: reversion,
        provisional: basis == "disbursed",
        basis,
        character: character_of(corpus, li),
        recipient_slices_excluded,
    }))
}

// ─── comparing periods ───────────────────────────────────────────────────────

/// One period's variance, in its own dollars and in the base period's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrendPoint {
    pub period: String,
    pub nominal_cents: i64,
    pub real_cents: i64,
}

/// One line item's variance compared across two periods, in constant dollars.
///
/// # Every field says which dollars it is in
///
/// An earlier version of this returned a [`GapResult`] with the real-terms difference written
/// into `variance_cents` and the later period's nominal figures left in the other fields. That
/// is three units in one record with nothing to tell them apart, which is the failure
/// [`real_dollars`] exists to prevent, committed inside the type meant to prevent it.
///
/// So the two endpoints carry both figures, and the change is reported twice: `real_change_cents`
/// is the answer, and `nominal_change_cents` is what subtracting without a deflator would have
/// said. Emitting the wrong one alongside the right one is deliberate — the correction is the
/// finding, and a silent correction teaches the reader nothing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Trend {
    pub line_item: String,
    /// Named on the record, because a constant-dollar figure without its index is not
    /// interpretable and this one will be quoted.
    pub series_name: String,
    pub base_period: String,
    pub earlier: TrendPoint,
    pub later: TrendPoint,
    /// The comparable difference: later minus earlier, both in base-period dollars.
    pub real_change_cents: i64,
    /// What a nominal subtraction would have said. Kept beside the real figure so the size of
    /// the correction is visible rather than absorbed.
    pub nominal_change_cents: i64,
}

/// A trend, or the reason there is none.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum TrendOutcome {
    Computed(Box<Trend>),
    Unavailable { reason: String },
    Refused { reason: String },
}

impl TrendOutcome {
    pub fn is_computed(&self) -> bool {
        matches!(self, TrendOutcome::Computed(_))
    }
}

/// Compares one line item's gap across two periods.
///
/// Requires a deflator. A sixteen-year corpus makes nominal cross-period comparison the
/// single easiest way to produce a confidently wrong result.
pub fn gap_trend(
    corpus: &Corpus,
    line_item_slug: &str,
    earlier: &str,
    later: &str,
    deflator: Option<&Deflator>,
) -> TrendOutcome {
    let Some(d) = deflator else {
        return TrendOutcome::Refused {
            reason: "cross-period comparison requires a deflator; nominal dollars across periods \
                     are not comparable"
                .into(),
        };
    };

    let point = |o: Outcome, period: &str| -> Result<TrendPoint, TrendOutcome> {
        match o {
            Outcome::Computed(r) => match real_dollars::deflate(r.variance_cents, period, d) {
                Ok(real) => Ok(TrendPoint {
                    period: period.to_string(),
                    nominal_cents: real.nominal_cents,
                    real_cents: real.real_cents,
                }),
                // An uncovered period is a refusal, not a missing figure: the corpus holds the
                // variance, and the index declines to restate it rather than extrapolating.
                Err(e) => Err(TrendOutcome::Refused {
                    reason: format!("{period}: {e}"),
                }),
            },
            Outcome::Unavailable { reason } => Err(TrendOutcome::Unavailable { reason }),
            Outcome::Refused { reason } => Err(TrendOutcome::Refused { reason }),
        }
    };

    let a = match point(gap_for(corpus, line_item_slug, earlier), earlier) {
        Ok(p) => p,
        Err(o) => return o,
    };
    let b = match point(gap_for(corpus, line_item_slug, later), later) {
        Ok(p) => p,
        Err(o) => return o,
    };

    TrendOutcome::Computed(Box::new(Trend {
        line_item: line_item_slug.to_string(),
        series_name: d.series_name.clone(),
        base_period: d.base_period.clone(),
        real_change_cents: b.real_cents - a.real_cents,
        nominal_change_cents: b.nominal_cents - a.nominal_cents,
        earlier: a,
        later: b,
    }))
}

/// One comparison per adjacent pair of periods the corpus can compute a gap for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrendCoverage {
    pub line_item: String,
    pub earlier: String,
    pub later: String,
    pub outcome: TrendOutcome,
}

/// Compares every adjacent pair of periods, for every line item with two or more.
///
/// Adjacent rather than first-to-last: a single sixteen-year difference hides everything that
/// happened in between, and the endpoints are recoverable from the steps while the steps are
/// not recoverable from the endpoints.
pub fn trends(corpus: &Corpus, deflator: Option<&Deflator>) -> Vec<TrendCoverage> {
    use std::collections::BTreeMap;

    let mut by_item: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (line_item, period) in coverage(corpus) {
        by_item.entry(line_item).or_default().push(period);
    }

    let mut out = Vec::new();
    for (line_item, mut periods) in by_item {
        // Periods are compared in the order they happened. `FY2024-25` sorts by its first year,
        // which is also the year it starts.
        periods.sort_by_key(|p| {
            (
                p.trim_start_matches("FY")
                    .split(['-', ' '])
                    .next()
                    .and_then(|y| y.parse::<i32>().ok())
                    .unwrap_or(i32::MAX),
                p.clone(),
            )
        });
        for w in periods.windows(2) {
            let (earlier, later) = (&w[0], &w[1]);
            out.push(TrendCoverage {
                line_item: line_item.clone(),
                earlier: earlier.clone(),
                later: later.clone(),
                outcome: gap_trend(corpus, &line_item, earlier, later, deflator),
            });
        }
    }
    out
}

/// Every (line item, period) pair the corpus holds an expenditure for.
///
/// The only pairs where a gap could exist at all — an appropriation with no expenditure
/// beside it is not a gap of unknown size, it is a question the corpus has not reached.
pub fn coverage(corpus: &Corpus) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for e in class(corpus, "expenditure") {
        let Some(period) = prop(e, "period_label") else {
            continue;
        };
        let dir = e.abs_path.parent().unwrap_or(&e.abs_path);
        for l in e
            .inst
            .links
            .iter()
            .filter(|l| l.relationship == "disburses-against")
        {
            let t = normalize_join(dir, &l.target);
            if let Some(name) = t.file_stem().and_then(|s| s.to_str()) {
                pairs.push((name.to_string(), period.trim().to_string()));
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}

/// One outcome per pair in [`coverage`], in the same order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Coverage {
    pub line_item: String,
    pub period: String,
    pub outcome: Outcome,
}

/// Runs the calculator over every pair the corpus could support.
pub fn all(repo_root: &std::path::Path) -> Result<Vec<Coverage>> {
    let corpus = corpus_validate::load(repo_root)?;
    Ok(coverage(&corpus)
        .into_iter()
        .map(|(line_item, period)| {
            let outcome = gap_for(&corpus, &line_item, &period);
            Coverage {
                line_item,
                period,
                outcome,
            }
        })
        .collect())
}

/// Every adjacent-period comparison the corpus could support, restated by `deflator`.
///
/// Passing `None` yields one refusal per pair rather than an empty list, so a feed built
/// without an index still says what it declined and why.
pub fn all_trends(
    repo_root: &std::path::Path,
    deflator: Option<&Deflator>,
) -> Result<Vec<TrendCoverage>> {
    Ok(trends(&corpus_validate::load(repo_root)?, deflator))
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

    fn corpus_with(appr_amount: &str, exp: &[(&str, &str, &str, bool)]) -> Corpus {
        // exp: (slug, amount, basis, paid_to)
        let mut instances = vec![
            inst(
                "c/line-item/foundation-funding.yml",
                "class: line-item\nlabel: FF\ndescription: d\n",
            ),
            inst(
                "c/program/fsfp.yml",
                r#"
class: program
label: FSFP
description: d
properties:
  policy_area: primary-secondary-education
  formula_basis: "computed base cost per pupil"
  federal_participation: "None directly."
links:
  - target: ../line-item/foundation-funding.yml
    relationship: funded-by
"#,
            ),
            inst(
                "c/appropriation/ff-fy2024-25.yml",
                &format!(
                    r#"
class: appropriation
label: A
description: d
properties:
  amount: "{appr_amount}"
  period_label: FY2024-25
  stage: as-enacted
links:
  - target: ../line-item/foundation-funding.yml
    relationship: grants-authority-for
"#
                ),
            ),
        ];
        for (slug, amount, basis, paid) in exp {
            let paid_link = if *paid {
                "  - target: ../jurisdiction/ccsd.yml\n    relationship: paid-to\n"
            } else {
                ""
            };
            instances.push(inst(
                &format!("c/expenditure/{slug}.yml"),
                &format!(
                    r#"
class: expenditure
label: E
description: d
properties:
  amount: "{amount}"
  basis: {basis}
  period_label: FY2024-25
  reversion: "[open]"
links:
  - target: ../line-item/foundation-funding.yml
    relationship: disburses-against
{paid_link}"#
                ),
            ));
        }
        Corpus {
            instances,
            ..Default::default()
        }
    }

    #[test]
    fn an_open_amount_is_unavailable_not_zero() {
        let c = corpus_with(
            "[open] pending the lsc connector",
            &[("e1", "$100.00", "actual-closed", false)],
        );
        match gap_for(&c, "foundation-funding", "FY2024-25") {
            Outcome::Unavailable { reason } => {
                assert!(reason.contains("no amount yet"), "{reason}")
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn a_complete_pair_computes() {
        let c = corpus_with("$1,000.00", &[("e1", "$900.00", "actual-closed", false)]);
        let Outcome::Computed(r) = gap_for(&c, "foundation-funding", "FY2024-25") else {
            panic!("expected a result");
        };
        assert_eq!(r.appropriated_cents, 100_000);
        assert_eq!(r.spent_cents, 90_000);
        assert_eq!(r.variance_cents, 10_000);
        assert!((r.variance_pct - 10.0).abs() < 1e-9);
        assert!(!r.provisional);
    }

    #[test]
    fn a_slice_alongside_a_whole_line_figure_is_set_aside_not_refused() {
        // The regression this guards. When the corpus gained a whole-line FY2024 actual, the
        // Columbus disbursement already sitting in FY2024 turned a computed answer into a
        // refusal — no figure changed, only the company the slice was keeping.
        let c = corpus_with(
            "$1,000,000.00",
            &[
                ("whole", "$999,000.00", "actual-closed", false),
                ("columbus", "$900.00", "actual-closed", true),
            ],
        );
        match gap_for(&c, "foundation-funding", "FY2024-25") {
            Outcome::Computed(r) => {
                assert_eq!(r.spent_cents, 99_900_000, "the slice must not be summed in");
                assert_eq!(r.recipient_slices_excluded, vec!["columbus".to_string()]);
            }
            other => panic!("expected Computed, got {other:?}"),
        }
    }

    #[test]
    fn a_recipient_slice_is_refused() {
        // Refused only when a slice is all the corpus has: then it is the only thing a reader
        // could reach for, and reaching for it is the error.
        let c = corpus_with("$1,000,000.00", &[("e1", "$900.00", "actual-closed", true)]);
        match gap_for(&c, "foundation-funding", "FY2024-25") {
            Outcome::Refused { reason } => {
                assert!(reason.contains("orders of magnitude"), "{reason}")
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn mixed_bases_are_refused() {
        let c = corpus_with(
            "$1,000.00",
            &[
                ("e1", "$400.00", "actual-closed", false),
                ("e2", "$500.00", "disbursed", false),
            ],
        );
        match gap_for(&c, "foundation-funding", "FY2024-25") {
            Outcome::Refused { reason } => assert!(reason.contains("mix bases"), "{reason}"),
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn an_in_year_basis_marks_the_result_provisional() {
        let c = corpus_with("$1,000.00", &[("e1", "$900.00", "disbursed", false)]);
        let Outcome::Computed(r) = gap_for(&c, "foundation-funding", "FY2024-25") else {
            panic!()
        };
        assert!(r.provisional, "an open period's figures will still move");
    }

    #[test]
    fn the_result_carries_how_to_read_it() {
        let c = corpus_with("$1,000.00", &[("e1", "$900.00", "actual-closed", false)]);
        let Outcome::Computed(r) = gap_for(&c, "foundation-funding", "FY2024-25") else {
            panic!()
        };
        assert!(r.character.formula_driven);
        assert!(!r.character.federally_matched);
        assert!(r.character.how_to_read().contains("formula"));
    }

    #[test]
    fn one_missing_side_is_unavailable_with_both_counts() {
        let c = corpus_with("$1,000.00", &[]);
        match gap_for(&c, "foundation-funding", "FY2024-25") {
            Outcome::Unavailable { reason } => assert!(reason.contains("both sides"), "{reason}"),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn a_cross_period_comparison_without_a_deflator_is_refused() {
        let c = corpus_with("$1,000.00", &[("e1", "$900.00", "actual-closed", false)]);
        match gap_trend(&c, "foundation-funding", "FY2010-11", "FY2024-25", None) {
            TrendOutcome::Refused { reason } => assert!(reason.contains("deflator"), "{reason}"),
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    /// Two periods of the same line item, so a trend has something to join.
    fn two_period_corpus() -> Corpus {
        let mut c = corpus_with("$1,000.00", &[("e1", "$900.00", "actual-closed", false)]);
        for (slug, period, appr, spent) in [
            ("a2", "FY2020", "$1,000.00", "$900.00"),
            ("a3", "FY2025", "$1,000.00", "$900.00"),
        ] {
            c.instances.push(inst(
                &format!("c/appropriation/{slug}.yml"),
                &format!(
                    "class: appropriation\nlabel: A\ndescription: d\nproperties:\n  \
                     amount: \"{appr}\"\n  period_label: {period}\n  stage: as-enacted\nlinks:\n  \
                     - target: ../line-item/foundation-funding.yml\n    \
                     relationship: grants-authority-for\n"
                ),
            ));
            c.instances.push(inst(
                &format!("c/expenditure/{slug}.yml"),
                &format!(
                    "class: expenditure\nlabel: E\ndescription: d\nproperties:\n  \
                     amount: \"{spent}\"\n  basis: actual-closed\n  period_label: {period}\n  \
                     reversion: \"[open]\"\nlinks:\n  \
                     - target: ../line-item/foundation-funding.yml\n    \
                     relationship: disburses-against\n"
                ),
            ));
        }
        c
    }

    fn test_deflator() -> Deflator {
        // FY2025 prices are 25% above FY2020's, so an identical nominal variance is a smaller
        // real one — the sign of the change flips relative to a nominal subtraction.
        Deflator::new(
            "FY2025",
            "test index",
            &[("FY2020", 100.0), ("FY2025", 125.0)],
        )
        .unwrap()
    }

    #[test]
    fn a_trend_reports_both_dollars_for_each_endpoint() {
        // The regression this guards: an earlier version wrote the real-terms difference into
        // a GapResult and left the later period's nominal figures in the neighbouring fields,
        // so one record carried three units with nothing to distinguish them.
        let d = test_deflator();
        let TrendOutcome::Computed(t) = gap_trend(
            &two_period_corpus(),
            "foundation-funding",
            "FY2020",
            "FY2025",
            Some(&d),
        ) else {
            panic!("expected a trend");
        };
        assert_eq!(t.earlier.nominal_cents, 10_000);
        assert_eq!(t.earlier.real_cents, 12_500, "FY2020 dollars buy more");
        assert_eq!(t.later.nominal_cents, 10_000);
        assert_eq!(t.later.real_cents, 10_000, "the base period is unchanged");
        assert_eq!(t.base_period, "FY2025");
        assert_eq!(t.series_name, "test index");
    }

    #[test]
    fn the_nominal_answer_travels_beside_the_real_one() {
        // Identical nominal variances: subtracting them says nothing changed. In constant
        // dollars the variance fell by a quarter. Both are emitted, because the gap between
        // them is the finding.
        let d = test_deflator();
        let TrendOutcome::Computed(t) = gap_trend(
            &two_period_corpus(),
            "foundation-funding",
            "FY2020",
            "FY2025",
            Some(&d),
        ) else {
            panic!()
        };
        assert_eq!(t.nominal_change_cents, 0);
        assert_eq!(t.real_change_cents, -2_500);
    }

    #[test]
    fn a_period_the_index_does_not_reach_is_refused_not_extrapolated() {
        let d = test_deflator();
        // FY2024-25 is a biennial label; the index is keyed by single fiscal year and has no
        // entry for it. Averaging two years to cover it would be a modeling decision.
        match gap_trend(
            &two_period_corpus(),
            "foundation-funding",
            "FY2020",
            "FY2024-25",
            Some(&d),
        ) {
            TrendOutcome::Refused { reason } => {
                assert!(reason.contains("extrapolating"), "{reason}")
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn trends_pair_adjacent_periods_in_chronological_order() {
        let d = test_deflator();
        let out = trends(&two_period_corpus(), Some(&d));
        let pairs: Vec<(&str, &str)> = out
            .iter()
            .map(|t| (t.earlier.as_str(), t.later.as_str()))
            .collect();
        // FY2020, FY2024-25 and FY2025 are all held; adjacency gives two steps, not three
        // pairwise combinations, and FY2020 comes first despite sorting after "FY2024-25"
        // lexically only by luck.
        assert_eq!(
            pairs,
            vec![("FY2020", "FY2024-25"), ("FY2024-25", "FY2025")]
        );
    }

    #[test]
    fn trends_without_a_deflator_are_refusals_rather_than_an_empty_list() {
        // A feed built with no index must still say what it declined. An empty list would read
        // as "no periods to compare", which is a different and false statement.
        let out = trends(&two_period_corpus(), None);
        assert!(!out.is_empty());
        assert!(out
            .iter()
            .all(|t| matches!(t.outcome, TrendOutcome::Refused { .. })));
    }

    #[test]
    fn an_unparsable_amount_is_refused_rather_than_skipped() {
        let c = corpus_with(
            "about eight billion",
            &[("e1", "$900.00", "actual-closed", false)],
        );
        assert!(matches!(
            gap_for(&c, "foundation-funding", "FY2024-25"),
            Outcome::Refused { .. }
        ));
    }

    #[test]
    fn an_unknown_line_item_is_unavailable() {
        let c = corpus_with("$1.00", &[("e1", "$1.00", "actual-closed", false)]);
        assert!(matches!(
            gap_for(&c, "no-such-line", "FY2024-25"),
            Outcome::Unavailable { .. }
        ));
    }
}
