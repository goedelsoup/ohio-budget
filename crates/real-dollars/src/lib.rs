//! Restates nominal budget amounts in constant dollars.
//!
//! Listed in [`proposals.yml`](../../.yidam/decisions/proposals.yml) as a peer of the other
//! calculators, and recorded there as a **precondition** for them. This corpus spans FY2010 to
//! the present; a sixteen-year comparison in nominal dollars is not a comparison, and treating
//! deflation as optional would silently corrupt every long-series result.
//!
//! # It refuses rather than defaulting
//!
//! There is no built-in deflator series, and that is deliberate. Which index to use is a
//! modeling decision, not a technical detail: a general price index and a
//! state-and-local-government-purchases index give materially different answers for a budget
//! series, and the difference is not noise. Baking one in would make that choice invisible.
//!
//! So [`Deflator`] must be supplied, it must name its base year, and a period it does not
//! cover is an error rather than an extrapolation.

use std::collections::BTreeMap;

use anyhow::{bail, Result};

/// A price index keyed by fiscal period label, e.g. `FY2016` or `FY2016-17`.
#[derive(Debug, Clone, PartialEq)]
pub struct Deflator {
    /// The period whose dollars the output is expressed in.
    pub base_period: String,
    /// Which index this is. Carried so a result can state it, because the choice is
    /// contestable and a constant-dollar figure without it is not interpretable.
    pub series_name: String,
    /// Index value per period. Higher means higher prices.
    pub index: BTreeMap<String, f64>,
}

impl Deflator {
    pub fn new(base_period: &str, series_name: &str, index: &[(&str, f64)]) -> Result<Self> {
        let map: BTreeMap<String, f64> = index.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        if !map.contains_key(base_period) {
            bail!("base period {base_period} is not in the index");
        }
        if map.values().any(|v| *v <= 0.0) {
            bail!("index values must be positive");
        }
        Ok(Self {
            base_period: base_period.to_string(),
            series_name: series_name.to_string(),
            index: map,
        })
    }

    fn factor(&self, period: &str) -> Result<f64> {
        let from = self.index.get(period).ok_or_else(|| {
            anyhow::anyhow!(
                "no index value for {period}; extrapolating would invent a price level, so the \
                 series must be extended explicitly"
            )
        })?;
        let base = self.index[&self.base_period];
        Ok(base / from)
    }
}

/// A nominal amount restated, carrying enough context to be quoted responsibly.
#[derive(Debug, Clone, PartialEq)]
pub struct Real {
    pub nominal_cents: i64,
    pub real_cents: i64,
    pub from_period: String,
    pub base_period: String,
    pub series_name: String,
}

impl Real {
    /// How a constant-dollar figure must be labelled: never a bare number.
    pub fn label(&self) -> String {
        format!("{} dollars ({})", self.base_period, self.series_name)
    }
}

/// Restates one amount.
pub fn deflate(nominal_cents: i64, from_period: &str, d: &Deflator) -> Result<Real> {
    let factor = d.factor(from_period)?;
    // Round half away from zero, so a restated figure never drifts toward zero on repeated
    // conversion and negatives behave the same as positives.
    let scaled = nominal_cents as f64 * factor;
    let real_cents = if scaled >= 0.0 {
        (scaled + 0.5).floor() as i64
    } else {
        (scaled - 0.5).ceil() as i64
    };
    Ok(Real {
        nominal_cents,
        real_cents,
        from_period: from_period.to_string(),
        base_period: d.base_period.clone(),
        series_name: d.series_name.clone(),
    })
}

/// Restates a series, failing as a whole if any period is uncovered.
///
/// All-or-nothing on purpose: a partially deflated series mixes nominal and real figures,
/// which is worse than no result because the mixture is invisible in a chart.
pub fn deflate_series(points: &[(String, i64)], d: &Deflator) -> Result<Vec<Real>> {
    let missing: Vec<&str> = points
        .iter()
        .map(|(p, _)| p.as_str())
        .filter(|p| !d.index.contains_key(*p))
        .collect();
    if !missing.is_empty() {
        bail!(
            "index does not cover {missing:?}; a partly deflated series silently mixes nominal \
             and real figures"
        );
    }
    points.iter().map(|(p, c)| deflate(*c, p, d)).collect()
}

/// Quantities that are nominal by nature and must not be deflated.
///
/// Debt service on a fixed-rate obligation and a statutory dollar threshold are nominal
/// amounts; restating them answers a question nobody asked.
pub fn is_nominal_by_nature(line_item_purpose: &str) -> bool {
    let p = line_item_purpose.to_ascii_lowercase();
    [
        "debt service",
        "bond",
        "statutory threshold",
        "principal and interest",
    ]
    .iter()
    .any(|k| p.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d() -> Deflator {
        Deflator::new(
            "FY2026",
            "test index",
            &[("FY2010", 80.0), ("FY2018", 90.0), ("FY2026", 120.0)],
        )
        .unwrap()
    }

    #[test]
    fn earlier_dollars_are_worth_more_in_base_terms() {
        let r = deflate(100_00, "FY2010", &d()).unwrap();
        // 120/80 = 1.5
        assert_eq!(r.real_cents, 150_00);
        assert_eq!(r.nominal_cents, 100_00);
    }

    #[test]
    fn the_base_period_is_unchanged() {
        assert_eq!(deflate(12_345, "FY2026", &d()).unwrap().real_cents, 12_345);
    }

    #[test]
    fn a_result_cannot_be_quoted_without_its_index() {
        let r = deflate(100_00, "FY2010", &d()).unwrap();
        assert_eq!(r.label(), "FY2026 dollars (test index)");
    }

    #[test]
    fn an_uncovered_period_is_an_error_not_an_extrapolation() {
        let e = deflate(1, "FY1999", &d()).unwrap_err().to_string();
        assert!(e.contains("extrapolating"), "{e}");
    }

    #[test]
    fn a_series_fails_whole_rather_than_partly_deflating() {
        // The dangerous case: a chart mixing nominal and real points looks fine.
        let points = vec![("FY2010".to_string(), 100), ("FY1999".to_string(), 100)];
        let e = deflate_series(&points, &d()).unwrap_err().to_string();
        assert!(e.contains("silently mixes"), "{e}");
    }

    #[test]
    fn a_covered_series_deflates_every_point() {
        let points = vec![
            ("FY2010".to_string(), 100_00),
            ("FY2018".to_string(), 100_00),
        ];
        let out = deflate_series(&points, &d()).unwrap();
        assert_eq!(out.len(), 2);
        assert!(
            out[0].real_cents > out[1].real_cents,
            "older dollars restate higher"
        );
    }

    #[test]
    fn negatives_round_the_same_way_as_positives() {
        let dd = Deflator::new("FY2026", "t", &[("FY2010", 3.0), ("FY2026", 4.0)]).unwrap();
        let pos = deflate(101, "FY2010", &dd).unwrap().real_cents;
        let neg = deflate(-101, "FY2010", &dd).unwrap().real_cents;
        assert_eq!(pos, -neg, "asymmetric rounding drifts a series toward zero");
    }

    #[test]
    fn a_base_period_outside_the_index_is_rejected_at_construction() {
        assert!(Deflator::new("FY2030", "t", &[("FY2026", 1.0)]).is_err());
    }

    #[test]
    fn nominal_quantities_are_flagged() {
        assert!(is_nominal_by_nature(
            "Debt service on general obligation bonds"
        ));
        assert!(!is_nominal_by_nature(
            "Distribution of the state share of school district operating costs"
        ));
    }
}

// ─── building an index from a published series ───────────────────────────────

/// A published price series, keyed by (year, month) of observation.
#[derive(Debug, Clone, PartialEq)]
pub struct Observations {
    pub series_name: String,
    pub by_month: BTreeMap<(i32, u32), f64>,
}

/// Parses a two-column FRED CSV: `observation_date,VALUE`.
///
/// Rows with an empty value are skipped rather than treated as zero. FRED writes one for a
/// month the publisher never released — October 2025 CPI-U, for instance, which BLS did not
/// collect — and a missing observation is exactly what must not be silently filled.
pub fn parse_fred_csv(text: &str, series_name: &str) -> Result<Observations> {
    let mut by_month = BTreeMap::new();
    for (n, line) in text.lines().enumerate().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (date, value) = line
            .split_once(',')
            .ok_or_else(|| anyhow::anyhow!("line {} is not two columns: {line:?}", n + 1))?;
        let value = value.trim();
        if value.is_empty() || value == "." {
            continue;
        }
        let y: i32 = date
            .get(0..4)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow::anyhow!("line {} has no year: {date:?}", n + 1))?;
        let m: u32 = date
            .get(5..7)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow::anyhow!("line {} has no month: {date:?}", n + 1))?;
        let v: f64 = value
            .parse()
            .map_err(|_| anyhow::anyhow!("line {} value is not a number: {value:?}", n + 1))?;
        by_month.insert((y, m), v);
    }
    if by_month.is_empty() {
        bail!("no observations parsed");
    }
    Ok(Observations {
        series_name: series_name.to_string(),
        by_month,
    })
}

impl Observations {
    /// Observations per year, inferred from the spacing of the series.
    ///
    /// Monthly and quarterly series both appear in this corpus's sources and neither announces
    /// which it is. Inferring it from the gap between consecutive observations means a fiscal
    /// year can be checked for completeness without the caller having to know.
    pub fn per_year(&self) -> Result<usize> {
        let months: Vec<u32> = self.by_month.keys().map(|(_, m)| *m).collect();
        let gap = months
            .windows(2)
            .map(|w| (w[1] + 12 - w[0]) % 12)
            .filter(|g| *g > 0)
            .min()
            .ok_or_else(|| anyhow::anyhow!("cannot infer cadence from one observation"))?;
        match gap {
            1 => Ok(12),
            3 => Ok(4),
            6 => Ok(2),
            12 => Ok(1),
            other => bail!("unrecognised cadence: {other} month(s) between observations"),
        }
    }

    /// Averages into Ohio fiscal years, which run 1 July through 30 June.
    ///
    /// Returns the index alongside the fiscal years it could **not** cover. A partial year is
    /// never averaged: the mean of eleven months is not the price level of a twelve-month year,
    /// and a figure deflated by one would be wrong by an amount nobody could see.
    pub fn ohio_fiscal_years(&self, first: i32, last: i32) -> Result<FiscalYearIndex> {
        let want = self.per_year()?;
        let mut index = Vec::new();
        let mut incomplete = Vec::new();
        for fy in first..=last {
            let vals: Vec<f64> = self
                .by_month
                .iter()
                .filter(|((y, m), _)| (*y == fy - 1 && *m >= 7) || (*y == fy && *m <= 6))
                .map(|(_, v)| *v)
                .collect();
            if vals.len() == want {
                index.push((format!("FY{fy}"), vals.iter().sum::<f64>() / want as f64));
            } else {
                incomplete.push(format!("FY{fy} ({} of {want} observations)", vals.len()));
            }
        }
        Ok(FiscalYearIndex { index, incomplete })
    }
}

/// A price series averaged onto Ohio fiscal years, with the years it could not reach.
#[derive(Debug, Clone, PartialEq)]
pub struct FiscalYearIndex {
    /// `("FY2024", 129.103)`, ascending.
    pub index: Vec<(String, f64)>,
    /// Fiscal years lacking a complete set of observations, with the count found.
    pub incomplete: Vec<String>,
}

#[cfg(test)]
mod fiscal_year_index {
    use super::*;

    fn monthly(range: &[(i32, u32, f64)]) -> String {
        let mut s = String::from("observation_date,X\n");
        for (y, m, v) in range {
            s.push_str(&format!("{y}-{m:02}-01,{v}\n"));
        }
        s
    }

    #[test]
    fn a_fiscal_year_runs_july_through_june() {
        let rows: Vec<(i32, u32, f64)> = (7..=12)
            .map(|m| (2023, m, 100.0))
            .chain((1..=6).map(|m| (2024, m, 112.0)))
            .collect();
        let o = parse_fred_csv(&monthly(&rows), "X").unwrap();
        let FiscalYearIndex {
            index: idx,
            incomplete: missing,
        } = o.ohio_fiscal_years(2024, 2024).unwrap();
        assert!(missing.is_empty(), "{missing:?}");
        assert_eq!(idx.len(), 1);
        assert_eq!(idx[0].0, "FY2024");
        assert!((idx[0].1 - 106.0).abs() < 1e-9, "{:?}", idx[0]);
    }

    #[test]
    fn a_month_the_publisher_never_released_leaves_the_year_uncovered() {
        // The real case: BLS did not collect October 2025 CPI-U, and FRED writes the row with
        // an empty value. Averaging the other eleven months would produce a price level for a
        // year that has none, and every figure deflated by it would be wrong invisibly.
        let mut s = String::from("observation_date,X\n");
        for m in 7..=12 {
            let row = if m == 10 {
                "2025-10-01,\n".to_string()
            } else {
                format!("2025-{m:02}-01,100.0\n")
            };
            s.push_str(&row);
        }
        for m in 1..=6 {
            s.push_str(&format!("2026-{m:02}-01,100.0\n"));
        }
        let o = parse_fred_csv(&s, "X").unwrap();
        let FiscalYearIndex {
            index: idx,
            incomplete: missing,
        } = o.ohio_fiscal_years(2026, 2026).unwrap();
        assert!(idx.is_empty());
        assert_eq!(missing, vec!["FY2026 (11 of 12 observations)".to_string()]);
    }

    #[test]
    fn a_quarterly_series_needs_four_observations_not_twelve() {
        // Both cadences appear in this corpus's sources and neither says which it is.
        let rows = vec![
            (2023, 7, 100.0),
            (2023, 10, 102.0),
            (2024, 1, 104.0),
            (2024, 4, 106.0),
        ];
        let o = parse_fred_csv(&monthly(&rows), "X").unwrap();
        assert_eq!(o.per_year().unwrap(), 4);
        let FiscalYearIndex {
            index: idx,
            incomplete: missing,
        } = o.ohio_fiscal_years(2024, 2024).unwrap();
        assert!(missing.is_empty());
        assert!((idx[0].1 - 103.0).abs() < 1e-9);
    }

    #[test]
    fn the_committed_series_build_the_years_they_claim() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.yidam/sources/price-index");
        let sl = parse_fred_csv(
            &std::fs::read_to_string(root.join("fred-a829rd3q086sbea.csv")).unwrap(),
            "state and local government purchases",
        )
        .unwrap();
        let FiscalYearIndex {
            index: idx,
            incomplete: missing,
        } = sl.ohio_fiscal_years(2020, 2026).unwrap();
        assert_eq!(
            idx.len(),
            7,
            "state/local should cover FY2020-FY2026: {missing:?}"
        );

        let cpi = parse_fred_csv(
            &std::fs::read_to_string(root.join("fred-cpiaucns.csv")).unwrap(),
            "CPI-U",
        )
        .unwrap();
        let FiscalYearIndex {
            index: idx,
            incomplete: missing,
        } = cpi.ohio_fiscal_years(2020, 2026).unwrap();
        assert_eq!(idx.len(), 6, "CPI-U cannot cover FY2026");
        assert_eq!(missing, vec!["FY2026 (11 of 12 observations)".to_string()]);
    }
}
