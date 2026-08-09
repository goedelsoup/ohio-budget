//! Whether the gap is explained by appropriations being moved to meet the outturn.
//!
//! # The alternative reading
//!
//! [`summarise`](crate::summarise) reports that Ohio spent 99% of what it appropriated across
//! 225 pairs, and that entitlement lines miss by twice as much as formula lines. A small
//! aggregate variance is consistent with careful forecasting. It is equally consistent with
//! **appropriations being adjusted mid-year to match what was going to be spent** — the
//! Controlling Board moves authority between line items throughout the year, and an appropriation
//! that chases its own outturn would produce exactly the same figure while meaning the opposite.
//!
//! The corpus recorded that as the sharpest caveat on its central finding and could not test it,
//! because nothing read the adjusted-appropriation column.
//!
//! # What this measures, and what it cannot
//!
//! Three workbooks carry an adjusted figure — FY2019, FY2021 and FY2023 — beside an enacted one
//! and, in a later document, an actual. If adjustment is what closes the gap, the adjusted
//! figure should sit much nearer the outturn than the enacted figure did, and the line items
//! adjusted should mostly move toward it.
//!
//! **Every adjusted figure here is taken early in its fiscal year** — 11 September 2018 for
//! FY2019, 30 September 2022 for FY2023. Adjustments made later are not in any committed source,
//! so this tests early adjustment and cannot speak for the rest of the year.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;
use corpus_schema::BillStage;
use lsc::columns::{ColumnKind, ColumnPlan, Identity};
use serde::Serialize;

/// One fiscal year read three ways.
///
/// Sources differ per year because LSC publishes the adjusted figure in whichever workbook
/// happened to be current. The joins are on ALI alone: the adjusted workbooks head their agency
/// column with a full department name where the others use a three-letter code, so a join on
/// (agency, ALI) silently matches nothing.
struct Case {
    fiscal_year: &'static str,
    enacted: (&'static str, &'static str),
    adjusted: (&'static str, &'static str),
    actual: (&'static str, &'static str),
    /// When the adjusted figure was taken, where the workbook says.
    taken: &'static str,
}

const CASES: [Case; 3] = [
    Case {
        fiscal_year: "FY2019",
        enacted: ("hb49-budget-in-detail-as-enrolled-132nd.xlsx", "EN"),
        adjusted: (
            "hb49-budget-in-detail-adjusted-appropriations-132nd.xlsx",
            "",
        ),
        actual: (
            "hb166-appropriation-spreadsheet-with-actuals-133rd.xlsx",
            "FY21Update",
        ),
        taken: "11 September 2018",
    },
    Case {
        fiscal_year: "FY2021",
        enacted: (
            "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
            "EN",
        ),
        adjusted: (
            "hb166-appropriation-spreadsheet-with-actuals-133rd.xlsx",
            "FY21Update",
        ),
        actual: (
            "hb110-appropriation-spreadsheet-with-actuals-134th.xlsx",
            "Update 9.30.22",
        ),
        taken: "[open] the workbook does not say",
    },
    Case {
        fiscal_year: "FY2023",
        enacted: (
            "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
            "EN",
        ),
        adjusted: (
            "hb110-appropriation-spreadsheet-with-actuals-134th.xlsx",
            "Update 9.30.22",
        ),
        actual: ("hb33-appropriation-spreadsheet-as-enacted-135th.xlsx", "EN"),
        taken: "30 September 2022",
    },
];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Year {
    pub fiscal_year: String,
    pub adjusted_taken: String,
    /// Line items carrying all three figures.
    pub line_items: usize,
    /// Total distance from the outturn, summed over line items.
    pub from_enacted_cents: i128,
    pub from_adjusted_cents: i128,
    /// Line items whose adjusted figure differs from the enacted one.
    pub adjusted: usize,
    /// Of those, how many moved toward the outturn and how many away.
    pub toward_outturn: usize,
    pub away_from_outturn: usize,
}

impl Year {
    /// How much of the enacted figure's distance from the outturn adjustment removed.
    pub fn distance_closed_pct(&self) -> Option<f64> {
        (self.from_enacted_cents != 0).then(|| {
            (self.from_enacted_cents - self.from_adjusted_cents) as f64
                / self.from_enacted_cents as f64
                * 100.0
        })
    }
    /// Share of adjusted line items that ended nearer the outturn.
    pub fn toward_share_pct(&self) -> Option<f64> {
        let n = self.toward_outturn + self.away_from_outturn;
        (n > 0).then(|| self.toward_outturn as f64 / n as f64 * 100.0)
    }
}

fn figures(
    dir: &Path,
    file: &str,
    sheet: &str,
    want: &dyn Fn(&ColumnKind) -> bool,
) -> Result<BTreeMap<String, i64>> {
    let path = dir.join(file);
    let sheet = if sheet.is_empty() {
        lsc::xlsx::sheet_names(&path)?
            .first()
            .cloned()
            .unwrap_or_default()
    } else {
        sheet.to_string()
    };
    let t = lsc::xlsx::sheet_to_table(&path, &sheet)?;
    let plan = ColumnPlan::of(&t.headers);
    let Some(l) = plan.identity_column(Identity::LineItemCode) else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    let mut duplicated = BTreeSet::new();
    for (i, kind) in plan.kinds.iter().enumerate() {
        if !want(kind) {
            continue;
        }
        for r in &t.rows {
            if !lsc::is_line_item_code(&r[l]) {
                continue;
            }
            let Ok(cents) = lsc::parse_money_to_cents(&r[i]) else {
                continue;
            };
            let k = r[l].trim().to_string();
            if out.insert(k.clone(), cents).is_some() {
                duplicated.insert(k);
            }
        }
    }
    for k in &duplicated {
        out.remove(k);
    }
    Ok(out)
}

/// Runs the comparison. Returns an empty list where the workbooks are absent.
pub fn analyse(repo_root: &Path) -> Result<Vec<Year>> {
    let dir = repo_root.join(".yidam/sources/lsc");
    let mut out = Vec::new();
    for c in &CASES {
        if !dir.join(c.enacted.0).is_file() || !dir.join(c.adjusted.0).is_file() {
            continue;
        }
        let fy = c.fiscal_year;
        let enacted = figures(&dir, c.enacted.0, c.enacted.1, &|k| {
            matches!(k, ColumnKind::Appropriation { stage, fiscal_year }
                if *stage == BillStage::AsEnacted && fiscal_year == fy)
        })?;
        let adjusted = figures(
            &dir,
            c.adjusted.0,
            c.adjusted.1,
            &|k| matches!(k, ColumnKind::AdjustedAppropriation { fiscal_year } if fiscal_year == fy),
        )?;
        let actual = figures(
            &dir,
            c.actual.0,
            c.actual.1,
            &|k| matches!(k, ColumnKind::Actual { fiscal_year } if fiscal_year == fy),
        )?;

        let mut y = Year {
            fiscal_year: fy.to_string(),
            adjusted_taken: c.taken.to_string(),
            line_items: 0,
            from_enacted_cents: 0,
            from_adjusted_cents: 0,
            adjusted: 0,
            toward_outturn: 0,
            away_from_outturn: 0,
        };
        for (code, e) in &enacted {
            let (Some(j), Some(a)) = (adjusted.get(code), actual.get(code)) else {
                continue;
            };
            y.line_items += 1;
            y.from_enacted_cents += (a - e).abs() as i128;
            y.from_adjusted_cents += (a - j).abs() as i128;
            if j == e {
                continue;
            }
            y.adjusted += 1;
            match (a - j).abs().cmp(&(a - e).abs()) {
                std::cmp::Ordering::Less => y.toward_outturn += 1,
                std::cmp::Ordering::Greater => y.away_from_outturn += 1,
                std::cmp::Ordering::Equal => {}
            }
        }
        out.push(y);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The figures `what-the-gap-actually-says.yml` quotes.
    #[test]
    fn adjustment_does_not_explain_the_gap() {
        let years = analyse(&root()).expect("committed workbooks");
        assert_eq!(
            years.len(),
            3,
            "FY2019, FY2021 and FY2023 carry an adjusted column"
        );

        for y in &years {
            assert!(
                y.line_items > 1_000,
                "{}: only {} joined",
                y.fiscal_year,
                y.line_items
            );
            // If adjustment were closing the gap, the adjusted figure would sit far nearer the
            // outturn. It removes under a fifth of the distance in every year.
            let closed = y.distance_closed_pct().unwrap();
            assert!(
                (0.0..20.0).contains(&closed),
                "{}: adjustment closed {closed:.1}% of the distance",
                y.fiscal_year
            );
            // And on the line items it touches, it moves away from the outturn more often than
            // toward it — in all three years.
            let toward = y.toward_share_pct().unwrap();
            assert!(
                toward < 50.0,
                "{}: {toward:.1}% of adjusted line items landed nearer the outturn",
                y.fiscal_year
            );
        }
    }
}
