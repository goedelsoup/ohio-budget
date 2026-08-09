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

use std::collections::BTreeMap;
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
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::OwnSourceInTransit => "own-source revenue in transit",
            Category::SharedStateRevenue => "shared state revenue",
            Category::Reimbursement => "reimbursement",
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
        }
    }
    /// True where the money is the state's own.
    pub fn is_state_money(&self) -> bool {
        !matches!(self, Category::OwnSourceInTransit)
    }
}

/// The classification, as (ALI, category).
///
/// Listed rather than inferred at runtime. A rule over names would be shorter and would be a
/// second, invisible set of judgments; these are the judgments, and they are reviewable.
pub const CLASSIFIED: [(&str, Category); 28] = [
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
];

/// Agencies these line items are appropriated through.
///
/// Three, not one, and that is the point. ALI 110901 sits under `TAX` while its successor 110908
/// sits under `RDF`; an extract taken from `RDF` alone omits four years of the largest
/// reimbursement and produces a spurious jump at the renumbering. That happened.
pub const AGENCIES: [&str; 3] = ["RDF", "EDU", "TAX"];

/// The committed workbooks carrying enacted appropriations, oldest first.
pub const WORKBOOKS: [&str; 8] = [
    "hb153-budget-in-detail-as-enrolled-129th.xls",
    "hb59-budget-in-detail-as-enrolled-130th.xlsx",
    "hb64-budget-in-detail-as-enrolled-131st.xlsx",
    "hb49-budget-in-detail-as-enrolled-132nd.xlsx",
    "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
    "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
    "hb33-appropriation-spreadsheet-as-enacted-135th.xlsx",
    "hb96-appropriation-spreadsheet-as-enacted-136th.xlsx",
];

/// One fiscal year's appropriations, by category.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Year {
    pub fiscal_year: String,
    pub own_source_cents: i64,
    pub shared_cents: i64,
    pub reimbursement_cents: i64,
    /// Present only where the price index reaches this year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real: Option<Real>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Real {
    pub own_source_cents: i64,
    pub shared_cents: i64,
    pub reimbursement_cents: i64,
    pub base_period: String,
    pub series_name: String,
}

impl Year {
    /// Shared revenue plus reimbursement: the money that is actually the state's.
    pub fn state_money_cents(&self) -> i64 {
        self.shared_cents + self.reimbursement_cents
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Findings {
    pub years: Vec<Year>,
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
    names
        .iter()
        .find(|n| n.to_lowercase().contains("without summary"))
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{}: no EN sheet and none without summary rows: {names:?}",
                path.display()
            )
        })
}

/// Reads one workbook's enacted appropriations for the classified line items.
///
/// Keyed by (agency, ALI): a code repeats within a sheet as a memorandum breakdown, and the
/// largest row is the line item.
fn read(path: &Path) -> Result<BTreeMap<(String, String), (String, i64)>> {
    let table = lsc::xlsx::sheet_to_table(path, &sheet_of(path)?)?;
    let plan = ColumnPlan::of(&table.headers);
    let (Some(a), Some(l)) = (
        plan.identity_column(Identity::Agency),
        plan.identity_column(Identity::LineItemCode),
    ) else {
        return Ok(BTreeMap::new());
    };
    let mut out: BTreeMap<(String, String), (String, i64)> = BTreeMap::new();
    for (i, kind) in plan.kinds.iter().enumerate() {
        let lsc::columns::ColumnKind::Appropriation { stage, fiscal_year } = kind else {
            continue;
        };
        if *stage != BillStage::AsEnacted {
            continue;
        }
        for r in &table.rows {
            let (agency, code) = (r[a].trim(), r[l].trim());
            if !AGENCIES.contains(&agency) || category_of(code).is_none() {
                continue;
            }
            let Ok(cents) = lsc::parse_money_to_cents(&r[i]) else {
                continue;
            };
            let e = out
                .entry((fiscal_year.clone(), code.to_string()))
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
    let mut by_year: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
    for w in workbooks {
        let path = sources.join(w);
        if !path.is_file() {
            continue;
        }
        for ((fy, code), (_, cents)) in read(&path)? {
            // A figure appearing in two workbooks is the same figure; take it once.
            by_year.entry(fy).or_default().insert(code, cents);
        }
    }

    let years = by_year
        .into_iter()
        .map(|(fiscal_year, codes)| {
            let mut y = Year {
                fiscal_year: fiscal_year.clone(),
                ..Default::default()
            };
            for (code, cents) in &codes {
                match category_of(code) {
                    Some(Category::OwnSourceInTransit) => y.own_source_cents += cents,
                    Some(Category::SharedStateRevenue) => y.shared_cents += cents,
                    Some(Category::Reimbursement) => y.reimbursement_cents += cents,
                    None => {}
                }
            }
            y.real = deflator.and_then(|d| {
                let one = |c: i64| {
                    real_dollars::deflate(c, &fiscal_year, d)
                        .ok()
                        .map(|r| r.real_cents)
                };
                Some(Real {
                    own_source_cents: one(y.own_source_cents)?,
                    shared_cents: one(y.shared_cents)?,
                    reimbursement_cents: one(y.reimbursement_cents)?,
                    base_period: d.base_period.clone(),
                    series_name: d.series_name.clone(),
                })
            });
            y
        })
        .collect();

    Ok(Findings {
        years,
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
