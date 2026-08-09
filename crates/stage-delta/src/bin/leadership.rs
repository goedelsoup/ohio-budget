//! `leadership` — tests whether a chamber's substitute moves money the way its presiding
//! officer's record suggests.
//!
//! # The question, and why one line item could not answer it
//!
//! [`leadership-and-the-anomalies`](../../../../.yidam/decisions/leadership-and-the-anomalies.yml)
//! records that the House substitute's *sign* on foundation funding splits exactly with the
//! Speaker across four biennia — Householder and Cupp cut it, Stephens and Huffman raised it —
//! and refuses to treat that as a finding. Four observations, two per side, and a confound of
//! identical shape: the Fair School Funding Plan was enacted in HB 110 and phased in across the
//! two budgets after it, so plan-phase and speakership change together over exactly this window.
//!
//! That record twice concluded the separating test needed *older biennia*, and twice was wrong
//! about it — LSC published no stage columns before FY2020-21. The separating test needs **more
//! line items in the same biennia**. The plan touches school funding and nothing else, so a
//! Speaker effect should appear on prison operations and tax administration too, and a
//! plan effect should not.
//!
//! Every committed workbook from the 133rd General Assembly on carries all nine stages for
//! roughly 1,400 line items. The evidence was in hand the whole time.
//!
//! # What it reports, and why two of everything
//!
//! Counts of line items and sums of dollars answer different questions and disagree here. A
//! chamber that raises nine hundred small lines and cuts Medicaid has raised most things and
//! reduced the budget; both statements are true and only one is usually quoted. So every row
//! carries both, and where they disagree the disagreement is the result.

use std::path::PathBuf;

use anyhow::{bail, Result};
use corpus_schema::BillStage;
use lsc::columns::ColumnPlan;

/// One bill, its presiding officers, and the two fiscal years it appropriates.
struct Biennium {
    bill: &'static str,
    ga: &'static str,
    speaker: &'static str,
    president: &'static str,
    file: &'static str,
    years: [&'static str; 2],
}

const BIENNIA: [Biennium; 4] = [
    Biennium {
        bill: "HB 166",
        ga: "133rd",
        speaker: "Householder",
        president: "Obhof",
        file: "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
        years: ["FY2020", "FY2021"],
    },
    Biennium {
        bill: "HB 110",
        ga: "134th",
        speaker: "Cupp",
        president: "Huffman",
        file: "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
        years: ["FY2022", "FY2023"],
    },
    Biennium {
        bill: "HB 33",
        ga: "135th",
        speaker: "Stephens",
        president: "Huffman",
        file: "hb33-appropriation-spreadsheet-as-enacted-135th.xlsx",
        years: ["FY2024", "FY2025"],
    },
    Biennium {
        bill: "HB 96",
        ga: "136th",
        speaker: "Huffman",
        president: "McColley",
        file: "hb96-appropriation-spreadsheet-as-enacted-136th.xlsx",
        years: ["FY2026", "FY2027"],
    },
];

/// Agencies whose appropriations the Fair School Funding Plan can reach.
///
/// `EDU` is primary and secondary education; `KID` is the Department of Children and Youth,
/// which took part of it in the 135th. The plan is a school funding formula and touches nothing
/// else, which is the whole reason this partition exists — if leadership explains the pattern it
/// should appear on both sides of this line, and if the plan explains it, only on one.
const PLAN_REACHES: [&str; 2] = ["EDU", "KID"];

#[derive(Default)]
struct Tally {
    raised: usize,
    cut: usize,
    net_cents: i128,
    gross_cents: i128,
}

impl Tally {
    fn add(&mut self, delta: i64) {
        if delta > 0 {
            self.raised += 1
        } else if delta < 0 {
            self.cut += 1
        } else {
            return;
        }
        self.net_cents += delta as i128;
        self.gross_cents += delta.unsigned_abs() as i128;
    }
    fn moved(&self) -> usize {
        self.raised + self.cut
    }
    /// Share of moved line items that went up. `None` when nothing moved — a ratio out of zero
    /// is undefined, and reporting it as 0% would read as "everything was cut".
    fn raised_share(&self) -> Option<f64> {
        (self.moved() > 0).then(|| self.raised as f64 / self.moved() as f64 * 100.0)
    }
}

fn pct(v: Option<f64>) -> String {
    v.map(|p| format!("{p:5.1}%"))
        .unwrap_or_else(|| "    —".into())
}

fn billions(c: i128) -> String {
    format!("{:+8.2}B", c as f64 / 1e11)
}

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = root.join(".yidam/sources/lsc");
    if !dir.is_dir() {
        bail!("no committed LSC sources at {}", dir.display());
    }

    // Both substitute stages: the House substitute is where the record's claim sits, and the
    // Senate substitute is the control — the same test on the other chamber's officer.
    let transitions = [
        (
            "House substitute",
            BillStage::AsIntroduced,
            BillStage::HouseSubstitute,
        ),
        (
            "Senate substitute",
            BillStage::AsPassedHouse,
            BillStage::SenateSubstitute,
        ),
    ];

    for (label, from, to) in transitions {
        println!("\n═══ {label}: what the chamber did to every line item it touched\n");
        println!(
            "  {:<7} {:<12} {:<8} │ {:>6} {:>6} {:>7} {:>10} │ {:>6} {:>6} {:>7} {:>10}",
            "bill", "officer", "year", "up", "down", "up%", "net", "up", "down", "up%", "net"
        );
        println!(
            "  {:<7} {:<12} {:<8} │ {:^32} │ {:^32}",
            "", "", "", "school funding (EDU, KID)", "everything else"
        );
        println!("  {}", "─".repeat(104));

        for b in &BIENNIA {
            let path = dir.join(b.file);
            let table = lsc::xlsx::sheet_to_table(&path, "EN")?;
            let plan = ColumnPlan::of(&table.headers);
            let agency = plan.identity_column(lsc::columns::Identity::Agency);

            for fy in b.years {
                let col = |stage: BillStage| {
                    plan.appropriation_columns()
                        .into_iter()
                        .find(|(_, s, y)| *s == stage && *y == fy)
                        .map(|(i, _, _)| i)
                };
                let (Some(a), Some(z)) = (col(from), col(to)) else {
                    continue;
                };

                let (mut plan_side, mut rest) = (Tally::default(), Tally::default());
                for r in &table.rows {
                    let (Ok(x), Ok(y)) = (
                        lsc::parse_money_to_cents(&r[a]),
                        lsc::parse_money_to_cents(&r[z]),
                    ) else {
                        continue;
                    };
                    let agy = agency
                        .and_then(|i| r.get(i))
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if PLAN_REACHES.contains(&agy) {
                        plan_side.add(y - x)
                    } else {
                        rest.add(y - x)
                    }
                }

                let officer = if label.starts_with("House") {
                    b.speaker
                } else {
                    b.president
                };
                println!(
                    "  {:<7} {:<12} {:<8} │ {:>6} {:>6} {:>7} {:>10} │ {:>6} {:>6} {:>7} {:>10}",
                    if fy == b.years[0] { b.bill } else { "" },
                    if fy == b.years[0] { officer } else { "" },
                    fy,
                    plan_side.raised,
                    plan_side.cut,
                    pct(plan_side.raised_share()),
                    billions(plan_side.net_cents),
                    rest.raised,
                    rest.cut,
                    pct(rest.raised_share()),
                    billions(rest.net_cents),
                );
            }
            let _ = b.ga;
        }
    }
    println!("\n  Counts are line items; net is dollars. Both are within one fiscal year, so no");
    println!("  deflator is involved and no sign here depends on the price level.\n");
    Ok(())
}
