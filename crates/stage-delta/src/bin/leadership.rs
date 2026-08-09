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

use std::collections::{BTreeMap, BTreeSet};
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
    executive_baseline(&dir)?;

    println!("\n  Counts are line items; net is dollars. Both are within one fiscal year, so no");
    println!("  deflator is involved and no sign here depends on the price level.\n");
    Ok(())
}

/// One line item's identity across workbooks.
///
/// Keyed on agency, ALI and fund group rather than on the ALI alone. A code appears more than
/// once in these sheets — as a memorandum breakdown, and occasionally under two fund groups —
/// and joining two workbooks on the code by itself would pair a line item in one with a
/// component of it in the other.
type Key = (String, String, String);

fn keyed(
    dir: &std::path::Path,
    file: &str,
    want: BillStage,
    fy: &str,
) -> Result<BTreeMap<Key, i64>> {
    let t = lsc::xlsx::sheet_to_table(&dir.join(file), "EN")?;
    let plan = ColumnPlan::of(&t.headers);
    let col = plan
        .appropriation_columns()
        .into_iter()
        .find(|(_, s, y)| *s == want && *y == fy)
        .map(|(i, _, _)| i);
    let (Some(c), Some(a), Some(l), Some(f)) = (
        col,
        plan.identity_column(lsc::columns::Identity::Agency),
        plan.identity_column(lsc::columns::Identity::LineItemCode),
        plan.identity_column(lsc::columns::Identity::FundGroup),
    ) else {
        bail!("{file}: no {want:?} column for {fy}, or no identity columns");
    };

    let mut out: BTreeMap<Key, i64> = BTreeMap::new();
    let mut duplicated: BTreeSet<Key> = BTreeSet::new();
    for r in &t.rows {
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
    // A key landing twice cannot be joined unambiguously, so it is dropped from both sides
    // rather than resolved by taking whichever row came last.
    for k in &duplicated {
        out.remove(k);
    }
    Ok(out)
}

/// Did the executive hand the chamber a lean budget or a generous one?
///
/// The alternative left open when leadership and plan-phase were both eliminated: a chamber that
/// receives a lean introduction raises line items and one that receives a generous introduction
/// trims them. If that is what the House substitute is tracking, the executive's own direction
/// should run **opposite** to the chamber's.
///
/// Measured against the previous biennium's **enacted** figure rather than against the estimate
/// column sitting in the same workbook. Both sides are then appropriations at a named stage —
/// an estimate is somebody's projection and is the one column this connector refuses to treat
/// as a figure.
fn executive_baseline(dir: &std::path::Path) -> Result<()> {
    // (bill, file, introduced year, previous bill's file, previous enacted year, speaker)
    let pairs = [
        (
            "HB 166",
            "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
            "FY2020",
            "hb49-budget-in-detail-as-enrolled-132nd.xlsx",
            "FY2019",
            "Householder",
        ),
        (
            "HB 110",
            "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
            "FY2022",
            "hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx",
            "FY2021",
            "Cupp",
        ),
        (
            "HB 33",
            "hb33-appropriation-spreadsheet-as-enacted-135th.xlsx",
            "FY2024",
            "hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx",
            "FY2023",
            "Stephens",
        ),
        (
            "HB 96",
            "hb96-appropriation-spreadsheet-as-enacted-136th.xlsx",
            "FY2026",
            "hb33-appropriation-spreadsheet-as-enacted-135th.xlsx",
            "FY2025",
            "Huffman",
        ),
    ];

    println!(
        "\n═══ What the executive handed over, against the previous biennium's enacted figure\n"
    );
    println!(
        "  {:<7} {:<12} {:<8} │ {:>6} {:>6} {:>7} {:>10} │ {:>9}",
        "bill", "speaker", "year", "up", "down", "up%", "net", "House up%"
    );
    println!("  {}", "─".repeat(78));

    for (bill, file, fy, prev_file, prev_fy, speaker) in pairs {
        let now = keyed(dir, file, BillStage::AsIntroduced, fy)?;
        let before = keyed(dir, prev_file, BillStage::AsEnacted, prev_fy)?;

        let mut t = Tally::default();
        for (k, v) in &now {
            if let Some(p) = before.get(k) {
                t.add(v - p);
            }
        }
        // The chamber's own figure for the same year, for the side-by-side.
        let intro = keyed(dir, file, BillStage::AsIntroduced, fy)?;
        let sub = keyed(dir, file, BillStage::HouseSubstitute, fy)?;
        let mut h = Tally::default();
        for (k, v) in &sub {
            if let Some(p) = intro.get(k) {
                h.add(v - p);
            }
        }

        println!(
            "  {bill:<7} {speaker:<12} {fy:<8} │ {:>6} {:>6} {:>7} {:>10} │ {:>9}",
            t.raised,
            t.cut,
            pct(t.raised_share()),
            billions(t.net_cents),
            pct(h.raised_share())
        );
    }
    println!("\n  Joined on (agency, ALI, fund group) across two workbooks; keys appearing twice");
    println!("  in either are dropped rather than paired with a component of themselves.");

    per_line_item(dir, &pairs)?;
    Ok(())
}

/// The same question asked where there are enough observations to answer it.
///
/// # Why the aggregate version cannot settle anything
///
/// Correlating four biennium-level numbers against four others is the shape of coincidence this
/// whole record exists to warn about. It produced `r = -0.61` here, in the hypothesised
/// direction, and with n=4 the threshold for significance is about 0.95. A pattern that
/// convincing on four points is what the Speaker alignment already was.
///
/// **The biennium is the wrong unit.** Ohio holds one every two years and the corpus has four;
/// it will still have four in 2028. The line item is a unit with roughly 1,400 per biennium, and
/// the question restated at that granularity is answerable now: *on the line items the executive
/// raised, what did the chamber do — and did it do something different on the ones the executive
/// cut?*
fn per_line_item(
    dir: &std::path::Path,
    pairs: &[(&str, &str, &str, &str, &str, &str)],
) -> Result<()> {
    println!("\n═══ Per line item: what the chamber did, given what the executive did\n");
    println!(
        "  {:<7} {:<12} │ {:>28} │ {:>28}",
        "bill", "speaker", "executive RAISED this line", "executive CUT this line"
    );
    println!(
        "  {:<7} {:<12} │ {:>8} {:>8} {:>10} │ {:>8} {:>8} {:>10}",
        "", "", "n", "House up", "up%", "n", "House up", "up%"
    );
    println!("  {}", "─".repeat(84));

    for (bill, file, fy, prev_file, prev_fy, speaker) in pairs {
        let before = keyed(dir, prev_file, BillStage::AsEnacted, prev_fy)?;
        let intro = keyed(dir, file, BillStage::AsIntroduced, fy)?;
        let sub = keyed(dir, file, BillStage::HouseSubstitute, fy)?;

        let (mut when_raised, mut when_cut) = (Tally::default(), Tally::default());
        for (k, i) in &intro {
            let (Some(p), Some(h)) = (before.get(k), sub.get(k)) else {
                continue;
            };
            let exec = i - p;
            let house = h - i;
            if house == 0 {
                continue;
            }
            if exec > 0 {
                when_raised.add(house)
            } else if exec < 0 {
                when_cut.add(house)
            }
        }
        println!(
            "  {bill:<7} {speaker:<12} │ {:>8} {:>8} {:>10} │ {:>8} {:>8} {:>10}",
            when_raised.moved(),
            when_raised.raised,
            pct(when_raised.raised_share()),
            when_cut.moved(),
            when_cut.raised,
            pct(when_cut.raised_share()),
        );
    }
    println!("\n  Line items the chamber left alone are excluded from both sides: the question is");
    println!("  which way it moved when it moved, not whether it moved.");

    println!("\n═══ Toward or away from the previous enacted level\n");
    println!("  The discriminating test. `Raises what the executive cut` is also what regression");
    println!("  to the mean looks like: condition on an unusually large proposed increase and the");
    println!(
        "  next measurement tends lower whether or not anybody intended it. If the chamber is"
    );
    println!("  actually pulling figures back toward last biennium's level, its substitute should");
    println!("  land *closer* to that level than the introduced figure did.\n");
    println!(
        "  {:<7} {:<12} │ {:>8} {:>8} {:>8} {:>9}",
        "bill", "speaker", "moved", "toward", "away", "toward%"
    );
    println!("  {}", "─".repeat(60));
    for (bill, file, fy, prev_file, prev_fy, speaker) in pairs {
        let before = keyed(dir, prev_file, BillStage::AsEnacted, prev_fy)?;
        let intro = keyed(dir, file, BillStage::AsIntroduced, fy)?;
        let sub = keyed(dir, file, BillStage::HouseSubstitute, fy)?;
        let (mut toward, mut away) = (0usize, 0usize);
        for (k, i) in &intro {
            let (Some(p), Some(h)) = (before.get(k), sub.get(k)) else {
                continue;
            };
            if h == i {
                continue;
            }
            // Distance to the level the previous biennium actually enacted.
            let was = (i - p).abs();
            let now = (h - p).abs();
            if now < was {
                toward += 1
            } else if now > was {
                away += 1
            }
        }
        let n = toward + away;
        println!(
            "  {bill:<7} {speaker:<12} │ {n:>8} {toward:>8} {away:>8} {:>9}",
            pct((n > 0).then(|| toward as f64 / n as f64 * 100.0))
        );
    }
    Ok(())
}
