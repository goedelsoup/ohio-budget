//! How each actor in the budget process responds to the one before it.
//!
//! # Why this is not `decompose`
//!
//! [`crate::decompose`] follows one line item through nine stages and asks what moved it.
//! This asks the opposite question at the opposite scale: across every line item in a bill,
//! does an actor's direction depend on what the previous actor did?
//!
//! It exists because every earlier attempt to characterise a chamber was posed per biennium,
//! where the corpus has four observations and always will —
//! [the unit of observation](../../../.yidam/decisions/the-unit-of-observation.yml) records what
//! that cost. Per line item there are roughly 1,400 per bill, and the same questions answer.
//!
//! # It reads workbooks, not the corpus
//!
//! Deliberately. The corpus models 22 line items and the answer needs all 1,400, so seeding a
//! node per line item per stage would be ~50,000 nodes carrying no prose and answering one
//! question. The committed workbooks already hold the figures; this reads them where they are.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{bail, Result};
use corpus_schema::BillStage;
use lsc::columns::{ColumnPlan, Identity};
use serde::Serialize;

/// A bill, its presiding officers, and the workbook carrying its stages.
#[derive(Debug, Clone, Copy)]
pub struct Biennium {
    pub bill: &'static str,
    pub general_assembly: &'static str,
    pub speaker: &'static str,
    pub senate_president: &'static str,
    pub file: &'static str,
    /// The two fiscal years this bill appropriates.
    ///
    /// Only the first is measured. See [`OBSERVED_YEAR`].
    pub years: [&'static str; 2],
    /// The previous bill's workbook, for the executive's baseline.
    pub previous_file: &'static str,
    pub previous_year: &'static str,
}

pub const BIENNIA: [Biennium; 4] = [
    Biennium {
        bill: "HB 166",
        general_assembly: "133rd",
        speaker: "Householder",
        senate_president: "Obhof",
        file: "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
        years: ["FY2020", "FY2021"],
        previous_file: "hb49-budget-in-detail-as-enrolled-132nd.xlsx",
        previous_year: "FY2019",
    },
    Biennium {
        bill: "HB 110",
        general_assembly: "134th",
        speaker: "Cupp",
        senate_president: "Huffman",
        file: "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
        years: ["FY2022", "FY2023"],
        previous_file: "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
        previous_year: "FY2021",
    },
    Biennium {
        bill: "HB 33",
        general_assembly: "135th",
        speaker: "Stephens",
        senate_president: "Huffman",
        file: "hb33-appropriation-spreadsheet-as-enacted-135th.xlsx",
        years: ["FY2024", "FY2025"],
        previous_file: "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
        previous_year: "FY2023",
    },
    Biennium {
        bill: "HB 96",
        general_assembly: "136th",
        speaker: "Huffman",
        senate_president: "McColley",
        file: "hb96-appropriation-spreadsheet-as-enacted-136th.xlsx",
        years: ["FY2026", "FY2027"],
        previous_file: "hb33-appropriation-spreadsheet-as-enacted-135th.xlsx",
        previous_year: "FY2025",
    },
];

/// Which of a biennium's two fiscal years is treated as the observation.
///
/// **The first, and only the first.** A substitute sets both years at once, so a line item's
/// treatment in a biennium is one decision and not two. Measured across every hand-off, a line
/// item moved in year one moves the *same* direction in year two 83.6% of the time, the opposite
/// direction 2.4%, and not at all 13.9%. [verified]
///
/// Pooling both years therefore roughly doubles n while adding about a sixth as much
/// information, which inflates every significance figure computed from it. This module was
/// briefly written to pool them and the counts silently went from 325 to 647 — the same data,
/// twice as confident.
///
/// The cost is real: 13.9% of year-one moves are not repeated, and a second-year-only decision
/// is invisible here. [open] A unit of (line item, biennium) taking the larger of the two moves
/// would keep both and is not implemented.
pub const OBSERVED_YEAR: usize = 0;

/// Agencies the Fair School Funding Plan can reach.
///
/// The partition that separates a policy explanation from a leadership one. The plan is a school
/// funding formula: a plan effect must be confined to this side, a leadership effect must not be.
pub const PLAN_REACHES: [&str; 2] = ["EDU", "KID"];

/// Where a figure comes from — a stage in this bill, or the previous bill's enacted column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Stage(BillStage),
    PreviousEnacted,
}

/// One line item's identity across workbooks.
///
/// Agency, ALI and fund group rather than the ALI alone. A code appears more than once in these
/// sheets — as a `- State`/`- Federal`/`- Total` memorandum breakdown, and occasionally under two
/// fund groups — so joining two workbooks on the code by itself pairs a line item in one with a
/// component of itself in the other.
pub type Key = (String, String, String);

/// Counts and dollars for one group of line items.
///
/// Both, always. On this corpus they have disagreed every time they have been asked separately —
/// see the conference result, where 6.2% of lines carry 51.2% of the contested money.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Tally {
    pub raised: usize,
    pub cut: usize,
    pub net_cents: i128,
    pub gross_cents: i128,
}

impl Tally {
    pub fn add(&mut self, delta: i64) {
        match delta.cmp(&0) {
            std::cmp::Ordering::Greater => self.raised += 1,
            std::cmp::Ordering::Less => self.cut += 1,
            std::cmp::Ordering::Equal => return,
        }
        self.net_cents += delta as i128;
        self.gross_cents += delta.unsigned_abs() as i128;
    }
    pub fn moved(&self) -> usize {
        self.raised + self.cut
    }
    /// Share of moved line items that went up. `None` when nothing moved: a ratio out of zero is
    /// undefined, and reporting it as 0% would read as "everything was cut".
    pub fn raised_share(&self) -> Option<f64> {
        (self.moved() > 0).then(|| self.raised as f64 / self.moved() as f64 * 100.0)
    }
}

fn keyed(dir: &Path, file: &str, stage: BillStage, fy: &str) -> Result<BTreeMap<Key, i64>> {
    let table = lsc::xlsx::sheet_to_table(&dir.join(file), "EN")?;
    let plan = ColumnPlan::of(&table.headers);
    let column = plan
        .appropriation_columns()
        .into_iter()
        .find(|(_, s, y)| *s == stage && *y == fy)
        .map(|(i, _, _)| i);
    let (Some(c), Some(a), Some(l), Some(f)) = (
        column,
        plan.identity_column(Identity::Agency),
        plan.identity_column(Identity::LineItemCode),
        plan.identity_column(Identity::FundGroup),
    ) else {
        bail!("{file}: no {stage:?} column for {fy}, or missing identity columns");
    };

    let mut out: BTreeMap<Key, i64> = BTreeMap::new();
    let mut duplicated: BTreeSet<Key> = BTreeSet::new();
    for r in &table.rows {
        // A row with no ALI code is a **grand total**, not a line item. Every workbook carries
        // between one and six of them — `Grand Total`, `Total All Funds`, `Total GRF`,
        // `GRF State Total` — and each one's movement is the sum of every line item's.
        //
        // Their effect on counts is negligible and on dollars is not: a single total row shifted
        // the share of contested money conference splits by sixteen percentage points. It looked
        // like a line item, it had a fund group and an agency, and nothing about the figure was
        // implausible.
        if !lsc::is_line_item_code(&r[l]) {
            continue;
        }
        let Ok(cents) = lsc::parse_money_to_cents(&r[c]) else {
            continue;
        };
        let k = (
            r[a].trim().to_string(),
            r[l].trim().to_string(),
            r[f].trim().to_string(),
        );
        if out.insert(k.clone(), cents).is_some() {
            duplicated.insert(k);
        }
    }
    // A key landing twice cannot be joined unambiguously, so it is dropped from both sides rather
    // than resolved by taking whichever row happened to come last.
    for k in &duplicated {
        out.remove(k);
    }
    Ok(out)
}

/// Exposed for the independence check in the tests below.
pub fn figures_pub(
    dir: &Path,
    b: &Biennium,
    fy: &str,
    source: Source,
) -> Result<BTreeMap<Key, i64>> {
    figures(dir, b, fy, source)
}

fn figures(dir: &Path, b: &Biennium, fy: &str, source: Source) -> Result<BTreeMap<Key, i64>> {
    match source {
        Source::Stage(s) => keyed(dir, b.file, s, fy),
        Source::PreviousEnacted => {
            keyed(dir, b.previous_file, BillStage::AsEnacted, b.previous_year)
        }
    }
}

// ─── one actor answering the one before it ───────────────────────────────────

/// A hand-off in the process: what the previous actor did, and what this one did next.
#[derive(Debug, Clone, Copy)]
pub struct HandOff {
    pub actor: &'static str,
    pub answering: &'static str,
    /// The previous actor's move, as (their baseline, their figure).
    pub prior: (Source, Source),
    /// This actor's move, as (their input, their figure).
    pub theirs: (Source, Source),
}

pub fn hand_offs() -> [HandOff; 3] {
    use BillStage::*;
    [
        HandOff {
            actor: "House",
            answering: "the executive",
            prior: (Source::PreviousEnacted, Source::Stage(AsIntroduced)),
            theirs: (Source::Stage(AsIntroduced), Source::Stage(HouseSubstitute)),
        },
        HandOff {
            actor: "Senate",
            answering: "the House",
            prior: (Source::Stage(AsIntroduced), Source::Stage(AsPassedHouse)),
            theirs: (
                Source::Stage(AsPassedHouse),
                Source::Stage(SenateSubstitute),
            ),
        },
        HandOff {
            actor: "conference",
            answering: "the Senate",
            prior: (Source::Stage(AsPassedHouse), Source::Stage(AsPassedSenate)),
            theirs: (
                Source::Stage(AsPassedSenate),
                Source::Stage(ConferenceReport),
            ),
        },
    ]
}

/// What one actor did in one bill, split by what its predecessor had done to the same line item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conditioning {
    pub actor: String,
    pub answering: String,
    pub bill: String,
    pub general_assembly: String,
    /// Who held the relevant chair, where one actor is a chamber. Absent for conference, which
    /// has no single presiding officer and where naming one would attribute a joint committee's
    /// behaviour to a person.
    pub officer: Option<String>,
    /// This actor's moves on line items its predecessor had raised.
    pub when_prior_raised: Tally,
    /// And on the ones its predecessor had cut.
    pub when_prior_cut: Tally,
    /// Line items whose figure moved toward the level the predecessor started from, against those
    /// that moved away.
    ///
    /// The discriminator. "Raises what its predecessor cut" is also what regression toward a line
    /// item's usual level looks like, which would be an artefact and not a behaviour. An actor
    /// pulling figures back should land *closer* to the baseline than its predecessor did.
    pub moved_toward_baseline: usize,
    pub moved_away_from_baseline: usize,
}

impl Conditioning {
    /// Percentage points by which this actor was likelier to raise a line its predecessor cut.
    ///
    /// `None` where either side had no movement to measure.
    pub fn reversal_gap(&self) -> Option<f64> {
        Some(self.when_prior_cut.raised_share()? - self.when_prior_raised.raised_share()?)
    }
    pub fn toward_share(&self) -> Option<f64> {
        let n = self.moved_toward_baseline + self.moved_away_from_baseline;
        (n > 0).then(|| self.moved_toward_baseline as f64 / n as f64 * 100.0)
    }
}

fn officer_for(actor: &str, b: &Biennium) -> Option<String> {
    match actor {
        "House" => Some(b.speaker.to_string()),
        "Senate" => Some(b.senate_president.to_string()),
        _ => None,
    }
}

/// Runs one hand-off across one bill, pooling its two fiscal years.
pub fn conditioning(dir: &Path, b: &Biennium, h: &HandOff) -> Result<Conditioning> {
    let mut out = Conditioning {
        actor: h.actor.to_string(),
        answering: h.answering.to_string(),
        bill: b.bill.to_string(),
        general_assembly: b.general_assembly.to_string(),
        officer: officer_for(h.actor, b),
        when_prior_raised: Tally::default(),
        when_prior_cut: Tally::default(),
        moved_toward_baseline: 0,
        moved_away_from_baseline: 0,
    };

    {
        let fy = b.years[OBSERVED_YEAR];
        let base = figures(dir, b, fy, h.prior.0)?;
        let prior = figures(dir, b, fy, h.prior.1)?;
        let input = figures(dir, b, fy, h.theirs.0)?;
        let result = figures(dir, b, fy, h.theirs.1)?;

        for (k, p) in &prior {
            let (Some(z), Some(i), Some(o)) = (base.get(k), input.get(k), result.get(k)) else {
                continue;
            };
            if o == i {
                continue;
            }
            match (p - z).cmp(&0) {
                std::cmp::Ordering::Greater => out.when_prior_raised.add(o - i),
                std::cmp::Ordering::Less => out.when_prior_cut.add(o - i),
                std::cmp::Ordering::Equal => {}
            }
            match (o - z).abs().cmp(&(i - z).abs()) {
                std::cmp::Ordering::Less => out.moved_toward_baseline += 1,
                std::cmp::Ordering::Greater => out.moved_away_from_baseline += 1,
                std::cmp::Ordering::Equal => {}
            }
        }
    }
    Ok(out)
}

// ─── where conference lands ──────────────────────────────────────────────────

/// Where the conference report sits, on line items the chambers disagreed about.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct ConferencePosition {
    pub bill: String,
    pub general_assembly: String,
    /// Line items where the House-passed and Senate-passed figures differ.
    pub contested: usize,
    pub at_house: usize,
    pub at_senate: usize,
    pub between: usize,
    pub outside_both: usize,
    /// The same four, weighted by the size of the disagreement — the quantity conference actually
    /// has to resolve. Counts and dollars answer differently here and the difference is the point.
    pub contested_cents: i128,
    pub cents_at_house: i128,
    pub cents_at_senate: i128,
    pub cents_between: i128,
    pub cents_outside: i128,
    /// Median position on a scale where 0.0 is the Senate's figure and 1.0 is the House's.
    pub median_position: Option<f64>,
}

pub fn conference_position(dir: &Path, b: &Biennium) -> Result<ConferencePosition> {
    let mut out = ConferencePosition {
        bill: b.bill.to_string(),
        general_assembly: b.general_assembly.to_string(),
        ..Default::default()
    };
    let mut positions: Vec<f64> = Vec::new();

    {
        let fy = b.years[OBSERVED_YEAR];
        let house = keyed(dir, b.file, BillStage::AsPassedHouse, fy)?;
        let senate = keyed(dir, b.file, BillStage::AsPassedSenate, fy)?;
        let conf = keyed(dir, b.file, BillStage::ConferenceReport, fy)?;
        for (k, h) in &house {
            let (Some(s), Some(c)) = (senate.get(k), conf.get(k)) else {
                continue;
            };
            if h == s {
                continue;
            }
            let spread = (h - s).unsigned_abs() as i128;
            out.contested += 1;
            out.contested_cents += spread;
            if c == h {
                out.at_house += 1;
                out.cents_at_house += spread;
            } else if c == s {
                out.at_senate += 1;
                out.cents_at_senate += spread;
            } else if *c > (*h).min(*s) && *c < (*h).max(*s) {
                out.between += 1;
                out.cents_between += spread;
            } else {
                out.outside_both += 1;
                out.cents_outside += spread;
            }
            positions.push((c - s) as f64 / (h - s) as f64);
        }
    }
    positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    out.median_position = positions.get(positions.len() / 2).copied();
    Ok(out)
}

/// Everything this module computes, for one repository.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessFindings {
    pub conditioning: Vec<Conditioning>,
    pub conference: Vec<ConferencePosition>,
    /// Which agencies were treated as reachable by the school funding plan, so a reader can see
    /// the partition rather than infer it.
    pub plan_reaches: Vec<String>,
}

/// Runs every measurement over the committed workbooks.
///
/// Returns `Ok(None)` where the sources are absent rather than failing: the feed is built in
/// contexts that may not carry a gigabyte of spreadsheets, and a missing input is a smaller
/// problem than an export that cannot run.
pub fn analyse(repo_root: &Path) -> Result<Option<ProcessFindings>> {
    let dir = repo_root.join(".yidam/sources/lsc");
    if !BIENNIA.iter().all(|b| dir.join(b.file).is_file()) {
        return Ok(None);
    }
    let mut conditionings = Vec::new();
    for h in hand_offs() {
        for b in &BIENNIA {
            conditionings.push(conditioning(&dir, b, &h)?);
        }
    }
    Ok(Some(ProcessFindings {
        conditioning: conditionings,
        conference: BIENNIA
            .iter()
            .map(|b| conference_position(&dir, b))
            .collect::<Result<_>>()?,
        plan_reaches: PLAN_REACHES.iter().map(|s| s.to_string()).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn a_tally_reports_no_share_rather_than_zero_when_nothing_moved() {
        // 0% would read as "everything was cut", which is the opposite of "nothing happened".
        assert!(Tally::default().raised_share().is_none());
        let mut t = Tally::default();
        t.add(0);
        assert_eq!(t.moved(), 0, "an unchanged figure is not a movement");
        assert!(t.raised_share().is_none());
    }

    #[test]
    fn a_tally_counts_direction_and_magnitude_separately() {
        let mut t = Tally::default();
        t.add(100);
        t.add(-300);
        assert_eq!((t.raised, t.cut), (1, 1));
        assert_eq!(t.net_cents, -200);
        assert_eq!(t.gross_cents, 400, "gross is activity, net is direction");
        assert_eq!(t.raised_share(), Some(50.0));
    }

    /// The figures quoted in `.yidam/decisions/the-unit-of-observation.yml` and on
    /// `conference-committee`.
    ///
    /// This test passed on figures that were wrong. It pins what the computation produces, which
    /// catches a record drifting from the code and does not catch the code being wrong — the
    /// grand-total rows were inside both. What found that was measuring a different question
    /// (whether vetoes move appropriation figures) and noticing a `Grand Total` in the output.
    ///
    /// Transcribed numbers in prose are the failure this whole exercise exists to close: a record
    /// asserting `27.4%` has nothing checking it, and the corpus has already been caught three
    /// times by a claim that quietly stopped being true. This is the check. If the workbooks
    /// change, the partition changes, or `OBSERVED_YEAR` moves, this fails and names the record
    /// that needs editing.
    #[test]
    fn the_figures_the_decision_records_quote() {
        let Some(f) = analyse(&root()).expect("analysis must run over committed workbooks") else {
            panic!("committed LSC workbooks are missing");
        };

        let pooled = |actor: &str| {
            let rows: Vec<&Conditioning> =
                f.conditioning.iter().filter(|c| c.actor == actor).collect();
            let (mut ra, mut rn, mut ca, mut cn) = (0usize, 0usize, 0usize, 0usize);
            for c in rows {
                ra += c.when_prior_raised.raised;
                rn += c.when_prior_raised.moved();
                ca += c.when_prior_cut.raised;
                cn += c.when_prior_cut.moved();
            }
            (ra, rn, ca, cn)
        };

        // "the House raises 27.4% of what the executive proposed to raise and 81.1% of what it
        // proposed to cut" — the-unit-of-observation.yml
        assert_eq!(pooled("House"), (89, 324, 90, 111));
        // "Senate, answering the House | 25.0% | 51.8%"
        assert_eq!(pooled("Senate"), (56, 223, 59, 113));
        // "conference, answering the Senate | 54.4% | 93.9%"
        assert_eq!(pooled("conference"), (31, 57, 107, 113));

        // "581 | 77.2% | 33.0%" and the rest of the conference table —
        // conference-committee.yml
        let c = &f.conference;
        let sum = |g: fn(&ConferencePosition) -> usize| c.iter().map(g).sum::<usize>();
        assert_eq!(sum(|x| x.contested), 751);
        assert_eq!(sum(|x| x.at_senate), 581);
        assert_eq!(sum(|x| x.at_house), 73);
        assert_eq!(sum(|x| x.between), 46);
        assert_eq!(sum(|x| x.outside_both), 51);

        let cents = |g: fn(&ConferencePosition) -> i128| c.iter().map(g).sum::<i128>();
        let total = cents(|x| x.contested_cents) as f64;
        let share = |v: i128| (v as f64 / total * 1000.0).round() / 10.0;
        assert_eq!(share(cents(|x| x.cents_between)), 38.1);
        assert_eq!(share(cents(|x| x.cents_at_senate)), 45.6);
    }

    #[test]
    fn every_actor_is_likelier_to_raise_what_its_predecessor_cut() {
        // The finding itself, as an assertion rather than as prose: twelve of twelve cells.
        let Some(f) = analyse(&root()).unwrap() else {
            panic!("sources missing")
        };
        assert_eq!(f.conditioning.len(), 12);
        for c in &f.conditioning {
            let gap = c
                .reversal_gap()
                .unwrap_or_else(|| panic!("{} {} has a side with no movement", c.actor, c.bill));
            assert!(
                gap > 0.0,
                "{} in {} raised {:?}% of what its predecessor raised and {:?}% of what it cut",
                c.actor,
                c.bill,
                c.when_prior_raised.raised_share(),
                c.when_prior_cut.raised_share()
            );
        }
    }

    #[test]
    fn grand_total_rows_are_never_read_as_line_items() {
        // Every workbook carries between one and six rows with no ALI code whose movement is the
        // sum of every line item's. Including them changed the share of contested money
        // conference splits from 38.1% to 51.2% and left the line counts almost untouched, so a
        // check of the counts would not have found it — and did not.
        use lsc::columns::{ColumnPlan, Identity};
        let dir = root().join(".yidam/sources/lsc");
        for b in &BIENNIA {
            let t = lsc::xlsx::sheet_to_table(&dir.join(b.file), "EN").unwrap();
            let plan = ColumnPlan::of(&t.headers);
            let l = plan.identity_column(Identity::LineItemCode).unwrap();
            let blank = t.rows.iter().filter(|r| r[l].trim().is_empty()).count();
            assert!(
                blank > 0,
                "{} has no total rows to exclude — has the format changed?",
                b.bill
            );

            let keyed_rows = keyed(&dir, b.file, BillStage::AsEnacted, b.years[0]).unwrap();
            assert!(
                !keyed_rows.keys().any(|(_, code, _)| code.is_empty()),
                "{}: a row with no ALI code reached the analysis",
                b.bill
            );
        }
    }

    #[test]
    fn conference_is_the_only_actor_whose_officer_is_absent() {
        // A joint committee has no single presiding officer, and printing one beside its
        // behaviour attributed a chamber's record to the wrong person once already.
        let Some(f) = analyse(&root()).unwrap() else {
            panic!("sources missing")
        };
        for c in &f.conditioning {
            assert_eq!(
                c.officer.is_none(),
                c.actor == "conference",
                "{} {}",
                c.actor,
                c.bill
            );
        }
    }
}
