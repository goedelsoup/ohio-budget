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
