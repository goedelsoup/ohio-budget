//! `local-finance` — renders the separation. All computation lives in the library.

use std::path::PathBuf;

use anyhow::{bail, Result};
use local_finance::{Category, WORKBOOKS};

fn b(cents: i64) -> String {
    format!("{:7.2}", cents as f64 / 1e11)
}

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let sources = root.join(".yidam/sources/lsc");
    if !sources.is_dir() {
        bail!("no committed LSC sources at {}", sources.display());
    }
    let (deflator, _) = corpus_export_deflator(&root);
    let f = local_finance::analyse(&sources, &WORKBOOKS, deflator.as_ref())?;

    let real = f.years.iter().any(|y| y.real.is_some());
    println!(
        "\n  {:<9}{:>12}{:>10}{:>13}{:>14}   {}\n",
        "",
        "own-source",
        "shared",
        "reimburse",
        "state total",
        if real {
            "real, FY2025 dollars, $B"
        } else {
            "NOMINAL, $B"
        }
    );
    for y in &f.years {
        let (o, s, r) = match &y.real {
            Some(x) => (x.own_source_cents, x.shared_cents, x.reimbursement_cents),
            None => (y.own_source_cents, y.shared_cents, y.reimbursement_cents),
        };
        println!(
            "  {:<9}{:>12}{:>10}{:>13}{:>14}{}",
            y.fiscal_year,
            b(o),
            b(s),
            b(r),
            b(s + r),
            if y.real.is_none() {
                "   (not restated)"
            } else {
                ""
            }
        );
    }

    if let (Some(first), Some(last)) = (
        f.years.iter().find(|y| y.real.is_some()),
        f.years.iter().rev().find(|y| y.real.is_some()),
    ) {
        let (a, z) = (first.real.as_ref().unwrap(), last.real.as_ref().unwrap());
        let pct = |x: i64, y: i64| {
            if x != 0 {
                format!("{:+6.1}%", (y as f64 / x as f64 - 1.0) * 100.0)
            } else {
                "     —".into()
            }
        };
        println!(
            "\n  {} to {}, in real terms:",
            first.fiscal_year, last.fiscal_year
        );
        println!(
            "    own-source in transit  {}",
            pct(a.own_source_cents, z.own_source_cents)
        );
        println!(
            "    shared state revenue   {}",
            pct(a.shared_cents, z.shared_cents)
        );
        println!(
            "    reimbursement          {}",
            pct(a.reimbursement_cents, z.reimbursement_cents)
        );
        println!(
            "    state money combined   {}",
            pct(
                a.shared_cents + a.reimbursement_cents,
                z.shared_cents + z.reimbursement_cents
            )
        );
    }

    println!("\n  Classification is a reading, not a record. What each category means:");
    for c in [
        Category::OwnSourceInTransit,
        Category::SharedStateRevenue,
        Category::Reimbursement,
    ] {
        println!("    {:<30} {}", c.label(), c.why());
    }
    println!();
    Ok(())
}

/// The deflator, built the way the feed builds it, so the two cannot disagree.
fn corpus_export_deflator(root: &std::path::Path) -> (Option<real_dollars::Deflator>, ()) {
    let path = root.join(".yidam/sources/price-index/fred-a829rd3q086sbea.csv");
    let Ok(text) = std::fs::read_to_string(path) else {
        return (None, ());
    };
    let name = "state and local government consumption expenditures and gross investment, \
                implicit price deflator (BEA, via FRED A829RD3Q086SBEA)";
    let d = real_dollars::parse_fred_csv(&text, name)
        .and_then(|o| o.ohio_fiscal_years(2010, 2027))
        .ok()
        .and_then(|i| {
            let pairs: Vec<(&str, f64)> = i.index.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            real_dollars::Deflator::new("FY2025", name, &pairs).ok()
        });
    (d, ())
}
