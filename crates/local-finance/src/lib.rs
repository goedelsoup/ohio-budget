//! Separates state money to local government from local money the state merely handles.
//!
//! # Why an agency total answers nothing
//!
//! Ohio appropriates $9.93B through agency `RDF`, revenue distribution funds, across 31 line
//! items in FY2026. It is an accounting container rather than a programme, and three unrelated
//! kinds of money sit in it:
//!
//! - **Own-source revenue in transit.** A county levies a permissive sales tax; the state
//!   collects it and hands it back. $3.707B in FY2026 — seven times the local government fund —
//!   and not one cent of it is state support.
//! - **Shared state revenue.** The state levies a tax and distributes a statutory share. The
//!   local government fund and the public library fund are this.
//! - **Reimbursement.** The state compensates local government for revenue it eliminated or
//!   relieved: the property tax rollbacks, and the tangible personal property replacement.
//!
//! Summing the agency mixes all three, which is how
//! [the sixth slice error](../../../.yidam/decisions/appropriation-is-not-distribution.yml) was
//! made — an accounting boundary taken for a policy one.
//!
//! # The classification is a reading and says so
//!
//! No source marks these categories. Each assignment is `[inference]` from the line item's name
//! and mechanism, the code lists are public so the argument can be had against the same figures,
//! and [`Category::why`] states the test applied. A reader who classifies the fuel tax as a user
//! fee returned to the roads that raised it will get different totals, and should.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;
use corpus_schema::BillStage;
use lsc::columns::{ColumnPlan, Identity};
use real_dollars::Deflator;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// A local government levied it; the state collects and returns it.
    OwnSourceInTransit,
    /// The state levied it and distributes a statutory share.
    SharedStateRevenue,
    /// The state compensates local government for revenue it removed.
    Reimbursement,
    /// Programme aid the state appropriates to school districts, which are local governments.
    ///
    /// Outside the other three by construction and reported separately, because including it
    /// changes the headline by 27 percentage points and the choice is not obvious. School
    /// foundation aid is a state programme with a formula, not a shared tax or a compensation;
    /// it is also, plainly, state money going to a political subdivision.
    SchoolFoundationAid,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::OwnSourceInTransit => "own-source revenue in transit",
            Category::SharedStateRevenue => "shared state revenue",
            Category::Reimbursement => "reimbursement",
            Category::SchoolFoundationAid => "school foundation aid",
        }
    }
    /// The test applied, so the classification can be argued with rather than only accepted.
    pub fn why(&self) -> &'static str {
        match self {
            Category::OwnSourceInTransit => {
                "the tax is levied by the local government; the state is a collection agent and \
                 the appropriation is a pass-through"
            }
            Category::SharedStateRevenue => {
                "the tax is levied by the state, and a statutory share is distributed; the share \
                 is a policy choice and has been changed by legislation"
            }
            Category::Reimbursement => {
                "the state removed or relieved a local revenue source and pays compensation; the \
                 obligation is fixed by past policy rather than by current receipts"
            }
            Category::SchoolFoundationAid => {
                "the state appropriates it to school districts under a funding formula; it is \
                 state money to a political subdivision but is neither a shared tax nor a \
                 compensation, and it is reported separately for that reason"
            }
        }
    }
    /// True where the money is the state's own.
    pub fn is_state_money(&self) -> bool {
        !matches!(self, Category::OwnSourceInTransit)
    }
    /// True for the two categories the narrow headline counts.
    pub fn in_narrow_boundary(&self) -> bool {
        matches!(self, Category::SharedStateRevenue | Category::Reimbursement)
    }
}

/// The classification, as (ALI, category).
///
/// Listed rather than inferred at runtime. A rule over names would be shorter and would be a
/// second, invisible set of judgments; these are the judgments, and they are reviewable.
pub const CLASSIFIED: [(&str, Category); 30] = [
    // Levied locally, collected by the state, returned.
    ("110963", Category::OwnSourceInTransit), // permissive sales tax
    ("110967", Category::OwnSourceInTransit), // school district income tax
    ("110902", Category::OwnSourceInTransit), // municipal net profit tax
    ("110995", Category::OwnSourceInTransit), // municipal income tax
    ("762902", Category::OwnSourceInTransit), // permissive auto registration tax
    ("110962", Category::OwnSourceInTransit), // resort area excise tax
    // Levied by the state, a statutory share distributed.
    ("110969", Category::SharedStateRevenue), // local government fund
    ("110965", Category::SharedStateRevenue), // public library fund
    ("110633", Category::SharedStateRevenue), // casino revenue, county
    ("110634", Category::SharedStateRevenue), // casino revenue, school districts
    ("110636", Category::SharedStateRevenue), // casino revenue, host city
    ("762901", Category::SharedStateRevenue), // auto registration distribution
    ("110617", Category::SharedStateRevenue), // international fuel tax
    ("762900", Category::SharedStateRevenue), // international registration plan
    ("110982", Category::SharedStateRevenue), // horse racing tax
    ("110996", Category::SharedStateRevenue), // horse racing tax, local payments
    // Compensation for revenue the state removed.
    ("110908", Category::Reimbursement), // property tax reimbursement, local
    ("200903", Category::Reimbursement), // property tax reimbursement, education
    ("110901", Category::Reimbursement), // predecessor of 110908
    ("200901", Category::Reimbursement), // predecessor of 200903
    ("110907", Category::Reimbursement), // TPP phase out, local
    ("200902", Category::Reimbursement), // TPP phase out, education
    ("110403", Category::Reimbursement), // TPP phase out, local, renumbered
    ("200417", Category::Reimbursement), // TPP phase out, school district, renumbered
    ("110981", Category::Reimbursement), // TPP replacement, business
    ("110954", Category::Reimbursement), // TPP replacement, utility
    ("200909", Category::Reimbursement), // school district TPP replacement, business
    ("200900", Category::Reimbursement), // school district TPP replacement, utility
    // Programme aid, reported outside the narrow boundary.
    //
    // ALI 200604 is deliberately absent. It carries `Foundation Funding - All Students` from a
    // dedicated purpose fund and exists only from FY2020, so including it compares a programme
    // of two line items against one of three: the growth reads +12.2% with it and +5.8% without.
    // A category that gains a member mid-window measures its own composition.
    ("200550", Category::SchoolFoundationAid),
    ("200612", Category::SchoolFoundationAid),
];

/// Agencies these line items are appropriated through.
///
/// Three, not one, and that is the point. ALI 110901 sits under `TAX` while its successor 110908
/// sits under `RDF`; an extract taken from `RDF` alone omits four years of the largest
/// reimbursement and produces a spurious jump at the renumbering. That happened.
pub const AGENCIES: [&str; 3] = ["RDF", "EDU", "TAX"];

/// The committed workbooks carrying enacted appropriations, oldest first.
pub const WORKBOOKS: [&str; 14] = [
    "hb153-budget-in-detail-as-enrolled-129th.xls",
    "hb59-budget-in-detail-as-enrolled-130th.xlsx",
    "hb64-budget-in-detail-as-enrolled-131st.xlsx",
    "hb49-budget-in-detail-as-enrolled-132nd.xlsx",
    "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
    "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
    "hb33-appropriation-spreadsheet-as-enacted-135th.xlsx",
    "hb96-appropriation-spreadsheet-as-enacted-136th.xlsx",
    // The with-actuals siblings. Each reports two completed years where the as-enrolled workbook
    // reports one, so without them only alternate years carry a disbursement figure.
    "hb59-budget-in-detail-with-actuals-130th.xlsx",
    "hb64-budget-in-detail-with-actuals-131st.xlsx",
    "hb49-budget-in-detail-with-actuals-132nd.xlsx",
    "hb166-appropriation-spreadsheet-with-actuals-133rd.xlsx",
    "hb110-appropriation-spreadsheet-with-actuals-134th.xlsx",
    "hb33-appropriation-spreadsheet-with-actuals-135th.xlsx",
];

/// One fiscal year, by category.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Year {
    pub fiscal_year: String,
    pub own_source_cents: i64,
    pub shared_cents: i64,
    pub reimbursement_cents: i64,
    /// School foundation aid, outside the narrow boundary and reported beside it.
    pub school_foundation_cents: i64,
    /// The same from the workbooks' closed-book actual columns, where they exist.
    ///
    /// A reimbursement appropriated is not a reimbursement paid, and the distinction is not
    /// decorative here: the appropriation is an authority the state sets and the disbursement is
    /// what a formula produced. Where they diverge, the appropriation is the weaker evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<Actuals>,
    /// Present only where the price index reaches this year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real: Option<Real>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Actuals {
    pub own_source_cents: i64,
    pub shared_cents: i64,
    pub reimbursement_cents: i64,
    pub school_foundation_cents: i64,
    /// Categories with no actual reported at all this year; their totals are absent, not zero.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories_missing: Vec<Category>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Real {
    pub own_source_cents: i64,
    pub shared_cents: i64,
    pub reimbursement_cents: i64,
    pub school_foundation_cents: i64,
    pub base_period: String,
    pub series_name: String,
}

impl Year {
    /// Shared revenue plus reimbursement — the narrow boundary, and the headline.
    pub fn state_money_cents(&self) -> i64 {
        self.shared_cents + self.reimbursement_cents
    }
    /// The narrow boundary plus school foundation aid.
    pub fn state_money_wide_cents(&self) -> i64 {
        self.state_money_cents() + self.school_foundation_cents
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Findings {
    pub years: Vec<Year>,
    /// Fiscal years carrying a disbursement and no appropriation, named rather than shown as
    /// zero.
    ///
    /// FY2010 is the case and it is permanent. A workbook appropriates one biennium and reports
    /// two completed years beside it, so the earliest committed document reaches two years
    /// further back in outturn than in authority — and the 128th General Assembly published no
    /// workbook at all. Carrying the year with `enacted = 0` would put a $0.00 point on every
    /// chart at the start of the series and read as a collapse.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actual_only_years: Vec<String>,
    /// Every classified line item, with the reasoning, so the categories travel with the totals.
    pub classification: Vec<ClassifiedItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClassifiedItem {
    pub code: String,
    pub category: Category,
    pub why: &'static str,
}

fn category_of(code: &str) -> Option<Category> {
    CLASSIFIED.iter().find(|(c, _)| *c == code).map(|(_, k)| *k)
}

/// Which sheet carries the detail.
///
/// Every workbook from the 130th on calls it `EN`. HB 153 has three sheets, two of which carry
/// **summary rows** — totals interleaved with the line items they total. Reading one of those
/// would double every figure in it, silently, because a summary row looks exactly like a line
/// item and the extractor has no way to tell.
fn sheet_of(path: &Path) -> Result<String> {
    let names = lsc::xlsx::sheet_names(path)?;
    if names.iter().any(|n| n == "EN") {
        return Ok("EN".into());
    }
    // The with-actuals workbooks each carry one sheet under a name that changes per biennium —
    // `FY21Update`, `Update 9.30.22`, `HB33`. Where there is nothing to choose between, there is
    // no choice to get wrong.
    if names.len() == 1 {
        return Ok(names[0].clone());
    }
    names
        .iter()
        .find(|n| n.to_lowercase().contains("without summary"))
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{}: no EN sheet, more than one sheet, and none without summary rows: {names:?}",
                path.display()
            )
        })
}

/// Reads one workbook's enacted appropriations for the classified line items.
///
/// Keyed by (agency, ALI): a code repeats within a sheet as a memorandum breakdown, and the
/// largest row is the line item.
/// One workbook's figures: (measure, fiscal year, ALI) to (agency, cents).
type Figures = BTreeMap<(Measure, String, String), (String, i64)>;

/// What a figure is: authority granted, or money out the door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Measure {
    Enacted,
    Actual,
}

fn read(path: &Path) -> Result<Figures> {
    let table = lsc::xlsx::sheet_to_table(path, &sheet_of(path)?)?;
    let plan = ColumnPlan::of(&table.headers);
    let (Some(a), Some(l)) = (
        plan.identity_column(Identity::Agency),
        plan.identity_column(Identity::LineItemCode),
    ) else {
        return Ok(BTreeMap::new());
    };
    let mut out: Figures = BTreeMap::new();
    for (i, kind) in plan.kinds.iter().enumerate() {
        let (measure, fiscal_year) = match kind {
            lsc::columns::ColumnKind::Appropriation { stage, fiscal_year }
                if *stage == BillStage::AsEnacted =>
            {
                (Measure::Enacted, fiscal_year)
            }
            lsc::columns::ColumnKind::Actual { fiscal_year } => (Measure::Actual, fiscal_year),
            _ => continue,
        };
        for r in &table.rows {
            let (agency, code) = (r[a].trim(), r[l].trim());
            // The code filter already excludes grand totals, since none of them carries a code.
            // Stated rather than relied on: the guard here is a side effect of the
            // classification being a fixed list, and a future rule over names would lose it.
            if !lsc::is_line_item_code(code)
                || !AGENCIES.contains(&agency)
                || category_of(code).is_none()
            {
                continue;
            }
            let Ok(cents) = lsc::parse_money_to_cents(&r[i]) else {
                continue;
            };
            let e = out
                .entry((measure, fiscal_year.clone(), code.to_string()))
                .or_insert_with(|| (agency.to_string(), i64::MIN));
            if cents > e.1 {
                *e = (agency.to_string(), cents);
            }
        }
    }
    Ok(out)
}

/// Runs the separation over every committed workbook.
pub fn analyse(
    sources: &Path,
    workbooks: &[&str],
    deflator: Option<&Deflator>,
) -> Result<Findings> {
    let mut by_year: BTreeMap<(Measure, String), BTreeMap<String, i64>> = BTreeMap::new();
    for w in workbooks {
        let path = sources.join(w);
        if !path.is_file() {
            continue;
        }
        for ((measure, fy, code), (_, cents)) in read(&path)? {
            // A figure appearing in two workbooks is the same figure; take it once.
            by_year
                .entry((measure, fy))
                .or_default()
                .insert(code, cents);
        }
    }

    let sum = |codes: &BTreeMap<String, i64>, want: Category| -> Option<i64> {
        let mut total = 0i64;
        let mut any = false;
        for (code, cents) in codes {
            if category_of(code) == Some(want) {
                total += cents;
                any = true;
            }
        }
        any.then_some(total)
    };

    // Every fiscal year either measure reaches. Keying on the enacted years alone dropped any
    // year carrying actuals and no appropriation — which is every year before the earliest
    // committed workbook's biennium, since a workbook reports two completed years and
    // appropriates two later ones.
    let all_years: BTreeSet<&String> = by_year.keys().map(|(_, fy)| fy).collect();
    let empty: BTreeMap<String, i64> = BTreeMap::new();
    let actual_only_years: Vec<String> = all_years
        .iter()
        .filter(|fy| !by_year.contains_key(&(Measure::Enacted, (**fy).clone())))
        .map(|fy| (*fy).clone())
        .collect();

    let years: Vec<Year> = all_years
        .into_iter()
        .filter(|fy| by_year.contains_key(&(Measure::Enacted, (*fy).clone())))
        .map(|fiscal_year| {
            let codes = by_year
                .get(&(Measure::Enacted, fiscal_year.clone()))
                .unwrap_or(&empty);
            let mut y = Year {
                fiscal_year: fiscal_year.clone(),
                own_source_cents: sum(codes, Category::OwnSourceInTransit).unwrap_or(0),
                shared_cents: sum(codes, Category::SharedStateRevenue).unwrap_or(0),
                reimbursement_cents: sum(codes, Category::Reimbursement).unwrap_or(0),
                school_foundation_cents: sum(codes, Category::SchoolFoundationAid).unwrap_or(0),
                ..Default::default()
            };
            y.actual = by_year
                .get(&(Measure::Actual, fiscal_year.clone()))
                .map(|a| {
                    let mut out = Actuals::default();
                    for (cat, slot) in [
                        (Category::OwnSourceInTransit, 0),
                        (Category::SharedStateRevenue, 1),
                        (Category::Reimbursement, 2),
                    ] {
                        match sum(a, cat) {
                            Some(v) => match slot {
                                0 => out.own_source_cents = v,
                                1 => out.shared_cents = v,
                                _ => out.reimbursement_cents = v,
                            },
                            // Absent, not zero. A category with no actual reported is a gap in the
                            // source, and summing it as nought would report a programme that spent
                            // nothing.
                            None => out.categories_missing.push(cat),
                        }
                    }
                    out
                });
            y.real = deflator.and_then(|d| {
                let one = |c: i64| {
                    real_dollars::deflate(c, fiscal_year, d)
                        .ok()
                        .map(|r| r.real_cents)
                };
                Some(Real {
                    own_source_cents: one(y.own_source_cents)?,
                    shared_cents: one(y.shared_cents)?,
                    reimbursement_cents: one(y.reimbursement_cents)?,
                    school_foundation_cents: one(y.school_foundation_cents)?,
                    base_period: d.base_period.clone(),
                    series_name: d.series_name.clone(),
                })
            });
            y
        })
        .collect();

    Ok(Findings {
        years,
        actual_only_years,
        classification: CLASSIFIED
            .iter()
            .map(|(code, category)| ClassifiedItem {
                code: code.to_string(),
                category: *category,
                why: category.why(),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn run() -> Findings {
        let text = std::fs::read_to_string(
            root().join(".yidam/sources/price-index/fred-a829rd3q086sbea.csv"),
        )
        .expect("committed price index");
        let name = "test";
        let i = real_dollars::parse_fred_csv(&text, name)
            .and_then(|o| o.ohio_fiscal_years(2010, 2027))
            .unwrap();
        let pairs: Vec<(&str, f64)> = i.index.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let d = Deflator::new("FY2025", name, &pairs).unwrap();
        analyse(&root().join(".yidam/sources/lsc"), &WORKBOOKS, Some(&d)).unwrap()
    }

    /// The figures quoted in `three-kinds-of-money-to-local-government.yml`.
    ///
    /// Hand-typed percentages in a record are a claim about a computation with nothing checking
    /// them, and this repository has been caught by that shape four times. If a classification
    /// changes or a workbook is added, this fails and names the record to edit.
    #[test]
    fn the_figures_the_decision_record_quotes() {
        let f = run();
        let year = |fy: &str| {
            f.years
                .iter()
                .find(|y| y.fiscal_year == fy)
                .unwrap_or_else(|| panic!("no {fy}"))
                .real
                .clone()
                .unwrap_or_else(|| panic!("{fy} not restated"))
        };
        let b = |c: i64| (c as f64 / 1e11 * 100.0).round() / 100.0;

        let a = year("FY2012");
        assert_eq!(
            (
                b(a.own_source_cents),
                b(a.shared_cents),
                b(a.reimbursement_cents)
            ),
            (3.22, 2.23, 3.97)
        );
        let z = year("FY2026");
        assert_eq!(
            (
                b(z.own_source_cents),
                b(z.shared_cents),
                b(z.reimbursement_cents)
            ),
            (4.78, 1.75, 1.95)
        );

        let pct = |x: i64, y: i64| ((y as f64 / x as f64 - 1.0) * 1000.0).round() / 10.0;
        assert_eq!(pct(a.own_source_cents, z.own_source_cents), 48.1);
        assert_eq!(pct(a.shared_cents, z.shared_cents), -21.3);
        assert_eq!(pct(a.reimbursement_cents, z.reimbursement_cents), -50.8);
        assert_eq!(
            pct(
                a.shared_cents + a.reimbursement_cents,
                z.shared_cents + z.reimbursement_cents
            ),
            -40.2
        );
    }

    #[test]
    fn state_money_falls_across_the_window_without_falling_every_year() {
        // The shape, asserted as it is rather than as it would be tidier. A first draft claimed
        // a decline in every step; a second claimed two upticks. There are three — FY2020 by
        // 0.08%, which is invisible at the two decimal places the report prints, and FY2024 and
        // FY2025 as the reimbursement stabilises. None of them recovers anything.
        let f = run();
        let mut real: Vec<(String, i64)> = f
            .years
            .iter()
            .filter_map(|y| {
                y.real.as_ref().map(|r| {
                    (
                        y.fiscal_year.clone(),
                        r.shared_cents + r.reimbursement_cents,
                    )
                })
            })
            .collect();
        real.sort();

        let peak = real.iter().max_by_key(|(_, v)| *v).expect("years");
        assert_eq!(peak.0, "FY2012", "the window opens at its high-water mark");

        let rises: Vec<&str> = real
            .windows(2)
            .filter(|w| w[1].1 > w[0].1)
            .map(|w| w[1].0.as_str())
            .collect();
        assert_eq!(
            rises,
            vec!["FY2020", "FY2024", "FY2025"],
            "three upticks, and only three"
        );

        // Third draft. The first two claimed the upticks were smaller than they are — one that
        // they did not exist, one that neither reached the previous biennium, and FY2025 does.
        // What is true is the size of the fall, so that is what is asserted.
        let at = |fy: &str| real.iter().find(|(y, _)| y == fy).unwrap().1;
        assert!(
            (at("FY2026") as f64) < at("FY2012") as f64 * 0.60,
            "the window ends below three fifths of where it opened"
        );
    }

    #[test]
    fn both_boundaries_are_computed_and_differ_by_27_points() {
        // The figures `three-kinds-of-money-to-local-government.yml` quotes for the wide
        // boundary. Twenty-seven points sit between the two headlines and neither is wrong;
        // quoting one without the boundary is what is wrong.
        let f = run();
        let at = |fy: &str| {
            f.years
                .iter()
                .find(|y| y.fiscal_year == fy)
                .and_then(|y| y.real.clone())
                .unwrap_or_else(|| panic!("no restated {fy}"))
        };
        let (a, z) = (at("FY2012"), at("FY2026"));
        let pct = |x: i64, y: i64| ((y as f64 / x as f64 - 1.0) * 1000.0).round() / 10.0;

        let narrow = |r: &Real| r.shared_cents + r.reimbursement_cents;
        assert_eq!(pct(narrow(&a), narrow(&z)), -40.2);
        assert_eq!(
            pct(a.school_foundation_cents, z.school_foundation_cents),
            5.8
        );
        assert_eq!(
            pct(
                narrow(&a) + a.school_foundation_cents,
                narrow(&z) + z.school_foundation_cents
            ),
            -13.0
        );

        // School districts' share of all state money reaching local government.
        let share = |r: &Real| {
            r.school_foundation_cents as f64 / (narrow(r) + r.school_foundation_cents) as f64
                * 100.0
        };
        assert_eq!(share(&a).round(), 59.0);
        assert_eq!(share(&z).round(), 72.0);
    }

    #[test]
    fn disbursement_tracks_appropriation() {
        // The finding counts appropriations, which the state chooses. If what was actually paid
        // diverged from what was authorised, the series would be measuring a decision rather
        // than an outcome.
        let f = run();
        let mut ratios = Vec::new();
        for y in &f.years {
            let Some(a) = &y.actual else { continue };
            let enacted = y.state_money_cents();
            if enacted == 0 {
                continue;
            }
            let paid = a.shared_cents + a.reimbursement_cents;
            ratios.push((y.fiscal_year.clone(), paid as f64 / enacted as f64));
        }
        assert!(ratios.len() >= 12, "only {} years carry both", ratios.len());
        for (fy, r) in &ratios {
            assert!(
                (0.95..=1.06).contains(r),
                "{fy}: disbursement was {:.1}% of appropriation",
                r * 100.0
            );
        }
    }

    #[test]
    fn a_year_with_no_appropriation_is_named_not_zeroed() {
        // FY2010 has a disbursement and no appropriation anywhere, permanently. Carried at zero
        // it would open every chart with a collapse.
        let f = run();
        assert_eq!(f.actual_only_years, vec!["FY2010".to_string()]);
        assert!(!f.years.iter().any(|y| y.fiscal_year == "FY2010"));
    }

    #[test]
    fn a_workbook_with_summary_rows_is_never_read() {
        // HB 153's default sheet interleaves totals with the line items they total. Reading it
        // would double every figure and nothing downstream could tell.
        let p = root().join(".yidam/sources/lsc/hb153-budget-in-detail-as-enrolled-129th.xls");
        assert_eq!(sheet_of(&p).unwrap(), "All Funds without Summary");
    }

    #[test]
    fn every_classified_code_states_its_test() {
        for c in &CLASSIFIED {
            assert!(!c.1.why().is_empty());
        }
        // Own-source money is not the state's, and the distinction is the whole point.
        assert!(!Category::OwnSourceInTransit.is_state_money());
        assert!(Category::SharedStateRevenue.is_state_money());
        assert!(Category::Reimbursement.is_state_money());
    }
}
