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
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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
    pub character: Character,
}

#[derive(Debug, Clone, PartialEq)]
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
    // Differencing it against the whole appropriation is wrong by orders of magnitude and
    // the result looks entirely reasonable, which is what makes it worth refusing.
    if let Some(sliced) = expenditures
        .iter()
        .find(|e| e.inst.links.iter().any(|l| l.relationship == "paid-to"))
    {
        return Outcome::Refused {
            reason: format!(
                "{} is scoped to one recipient; comparing a recipient's share against the whole \
                 line item's authority is wrong by orders of magnitude. Aggregate recipients \
                 first, or compare like to like at recipient level.",
                slug_of(&sliced.rel_path)
            ),
        };
    }

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
    }))
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
) -> Result<Outcome> {
    let Some(d) = deflator else {
        return Ok(Outcome::Refused {
            reason: "cross-period comparison requires a deflator; nominal dollars across periods \
                     are not comparable"
                .into(),
        });
    };
    let (a, b) = (
        gap_for(corpus, line_item_slug, earlier),
        gap_for(corpus, line_item_slug, later),
    );
    match (a, b) {
        (Outcome::Computed(x), Outcome::Computed(y)) => {
            let rx = real_dollars::deflate(x.variance_cents, earlier, d)?;
            let ry = real_dollars::deflate(y.variance_cents, later, d)?;
            Ok(Outcome::Computed(Box::new(GapResult {
                variance_cents: ry.real_cents - rx.real_cents,
                ..*y
            })))
        }
        (other, Outcome::Computed(_)) | (Outcome::Computed(_), other) => Ok(other),
        (other, _) => Ok(other),
    }
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
    fn a_recipient_slice_is_refused() {
        // Wrong by orders of magnitude, and the result would look entirely reasonable.
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
        match gap_trend(&c, "foundation-funding", "FY2010-11", "FY2024-25", None).unwrap() {
            Outcome::Refused { reason } => assert!(reason.contains("deflator"), "{reason}"),
            other => panic!("expected Refused, got {other:?}"),
        }
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
