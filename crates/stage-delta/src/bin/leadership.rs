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

    conference(dir, &pairs)?;

    for c in [
        Conditioning {
            prior: (Stage::PreviousEnacted, Stage::Here(BillStage::AsIntroduced)),
            theirs: (
                Stage::Here(BillStage::AsIntroduced),
                Stage::Here(BillStage::HouseSubstitute),
            ),
            label: "The House answering the executive",
        },
        Conditioning {
            prior: (
                Stage::Here(BillStage::AsIntroduced),
                Stage::Here(BillStage::AsPassedHouse),
            ),
            theirs: (
                Stage::Here(BillStage::AsPassedHouse),
                Stage::Here(BillStage::SenateSubstitute),
            ),
            label: "The Senate answering the House",
        },
        Conditioning {
            prior: (
                Stage::Here(BillStage::AsPassedHouse),
                Stage::Here(BillStage::AsPassedSenate),
            ),
            theirs: (
                Stage::Here(BillStage::AsPassedSenate),
                Stage::Here(BillStage::ConferenceReport),
            ),
            label: "Conference answering the Senate",
        },
    ] {
        conditional(dir, &pairs, &c)?;
    }
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
/// the question restated at that granularity is answerable now: *on the line items the previous
/// actor raised, what did this one do — and did it do something different on the ones the
/// previous actor cut?*
///
/// Run over each consecutive pair of actors in the sequence, because a conditioning found only
/// at one hand-off is a fact about that chamber and one found at all of them is a fact about how
/// the process works.
struct Conditioning {
    /// What the previous actor did, as (baseline, their figure).
    prior: (Stage, Stage),
    /// What this actor did, as (their input, their figure).
    theirs: (Stage, Stage),
    label: &'static str,
}

/// Where a figure comes from: a stage in this bill, or the previous bill's enacted column.
#[derive(Clone, Copy)]
enum Stage {
    Here(BillStage),
    PreviousEnacted,
}

fn figures(
    dir: &std::path::Path,
    b: &(&str, &str, &str, &str, &str, &str),
    stage: Stage,
) -> Result<BTreeMap<Key, i64>> {
    let (_, file, fy, prev_file, prev_fy, _) = *b;
    match stage {
        Stage::Here(s) => keyed(dir, file, s, fy),
        Stage::PreviousEnacted => keyed(dir, prev_file, BillStage::AsEnacted, prev_fy),
    }
}

/// The General Assembly a bill belongs to, for labelling rows.
///
/// Not the presiding officer: this table runs over the House, the Senate and conference, and
/// printing the Speaker beside the Senate's behaviour would attribute it to the wrong person.
fn ga_of(bill: &str) -> &'static str {
    BIENNIA
        .iter()
        .find(|b| b.bill == bill)
        .map(|b| b.ga)
        .unwrap_or("?")
}

fn conditional(
    dir: &std::path::Path,
    pairs: &[(&str, &str, &str, &str, &str, &str)],
    c: &Conditioning,
) -> Result<()> {
    println!("\n═══ {}\n", c.label);
    println!(
        "  {:<7} {:<12} │ {:>26} │ {:>26}",
        "bill", "GA", "previous actor RAISED it", "previous actor CUT it"
    );
    println!(
        "  {:<7} {:<12} │ {:>8} {:>7} {:>9} │ {:>8} {:>7} {:>9}",
        "", "", "n", "up", "up%", "n", "up", "up%"
    );
    println!("  {}", "─".repeat(80));

    let (mut pa, mut pb, mut ca, mut cb) = (0usize, 0usize, 0usize, 0usize);
    for b in pairs {
        let base = figures(dir, b, c.prior.0)?;
        let prior = figures(dir, b, c.prior.1)?;
        let input = figures(dir, b, c.theirs.0)?;
        let out = figures(dir, b, c.theirs.1)?;

        let (mut when_raised, mut when_cut) = (Tally::default(), Tally::default());
        for (k, p) in &prior {
            let (Some(z), Some(i), Some(o)) = (base.get(k), input.get(k), out.get(k)) else {
                continue;
            };
            let moved = o - i;
            if moved == 0 {
                continue;
            }
            match (p - z).cmp(&0) {
                std::cmp::Ordering::Greater => when_raised.add(moved),
                std::cmp::Ordering::Less => when_cut.add(moved),
                std::cmp::Ordering::Equal => {}
            }
        }
        pa += when_raised.raised;
        pb += when_raised.moved();
        ca += when_cut.raised;
        cb += when_cut.moved();
        println!(
            "  {:<7} {:<12} │ {:>8} {:>7} {:>9} │ {:>8} {:>7} {:>9}",
            b.0,
            ga_of(b.0),
            when_raised.moved(),
            when_raised.raised,
            pct(when_raised.raised_share()),
            when_cut.moved(),
            when_cut.raised,
            pct(when_cut.raised_share()),
        );
    }
    println!("  {}", "─".repeat(80));
    println!(
        "  {:<7} {:<12} │ {:>8} {:>7} {:>9} │ {:>8} {:>7} {:>9}",
        "POOLED",
        "",
        pb,
        pa,
        pct((pb > 0).then(|| pa as f64 / pb as f64 * 100.0)),
        cb,
        ca,
        pct((cb > 0).then(|| ca as f64 / cb as f64 * 100.0)),
    );
    Ok(())
}

/// Where conference lands when the two chambers disagree.
///
/// # Why this needs its own measurement
///
/// The conditioning table says conference restores 93.9% of what the Senate cut, and that a
/// line item is twice as likely to arrive there cut as raised. Both are consistent with
/// conference deciding something and with conference doing nothing but average two positions,
/// and those are very different claims about how Ohio's budget is settled.
///
/// The test is positional rather than directional. Where the House-passed and Senate-passed
/// figures differ, the conference report can only land in one of four places, and they mean
/// different things:
///
/// - **at one chamber's figure** — that chamber prevailed on this line;
/// - **strictly between them** — a split, which is what "splitting the difference" would mean;
/// - **outside both** — conference did something neither chamber had proposed, which is the only
///   one of the four that is conference exercising judgment of its own.
fn conference(dir: &std::path::Path, pairs: &[(&str, &str, &str, &str, &str, &str)]) -> Result<()> {
    println!("\n═══ Where conference lands when the chambers disagree\n");
    println!(
        "  {:<7} {:<6} │ {:>7} {:>8} {:>8} {:>8} {:>8} │ {:>9}",
        "bill", "GA", "differ", "House", "Senate", "between", "outside", "median"
    );
    println!("  {}", "─".repeat(74));

    let (mut th, mut ts, mut tb, mut to) = (0usize, 0usize, 0usize, 0usize);
    let (mut gh, mut gs, mut gb, mut go) = (0i128, 0i128, 0i128, 0i128);
    let mut all_positions: Vec<f64> = Vec::new();

    for b in pairs {
        let house = figures(dir, b, Stage::Here(BillStage::AsPassedHouse))?;
        let senate = figures(dir, b, Stage::Here(BillStage::AsPassedSenate))?;
        let conf = figures(dir, b, Stage::Here(BillStage::ConferenceReport))?;

        let (mut at_h, mut at_s, mut between, mut outside) = (0usize, 0usize, 0usize, 0usize);
        // Dollars beside counts, because on this corpus they have disagreed every time they
        // have been asked separately. 77% of *line items* is not 77% of the money.
        let (mut dh, mut ds, mut db, mut d_out) = (0i128, 0i128, 0i128, 0i128);
        let mut positions: Vec<f64> = Vec::new();
        for (k, h) in &house {
            let (Some(s), Some(c)) = (senate.get(k), conf.get(k)) else {
                continue;
            };
            if h == s {
                continue;
            }
            let spread = (h - s).unsigned_abs() as i128;
            if c == h {
                at_h += 1;
                dh += spread
            } else if c == s {
                at_s += 1;
                ds += spread
            } else if (c > h.min(s)) && (c < h.max(s)) {
                between += 1;
                db += spread
            } else {
                outside += 1;
                d_out += spread
            }
            // 0.0 at the Senate's figure, 1.0 at the House's. Meaningful for every case,
            // including the ones that land outside — those simply fall beyond [0, 1].
            positions.push((c - s) as f64 / (h - s) as f64);
        }
        let n = at_h + at_s + between + outside;
        positions.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        let median = positions.get(positions.len() / 2).copied();
        all_positions.extend(positions);
        th += at_h;
        ts += at_s;
        tb += between;
        to += outside;
        gh += dh;
        gs += ds;
        gb += db;
        go += d_out;
        println!(
            "  {:<7} {:<6} │ {n:>7} {at_h:>8} {at_s:>8} {between:>8} {outside:>8} │ {:>9}",
            b.0,
            ga_of(b.0),
            median
                .map(|m| format!("{m:.2}"))
                .unwrap_or_else(|| "—".into())
        );
    }
    let n = th + ts + tb + to;
    all_positions.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    println!("  {}", "─".repeat(74));
    println!(
        "  {:<7} {:<6} │ {n:>7} {th:>8} {ts:>8} {tb:>8} {to:>8} │ {:>9}",
        "POOLED",
        "",
        all_positions
            .get(all_positions.len() / 2)
            .map(|m| format!("{m:.2}"))
            .unwrap_or_else(|| "—".into())
    );
    let total = gh + gs + gb + go;
    let share = |v: i128| {
        if total > 0 {
            format!("{:.1}%", v as f64 / total as f64 * 100.0)
        } else {
            "—".into()
        }
    };
    println!(
        "\n  by contested dollars │ {:>7} {:>8} {:>8} {:>8} {:>8}",
        "",
        share(gh),
        share(gs),
        share(gb),
        share(go)
    );
    println!(
        "\n  Position is 0.00 at the Senate's figure and 1.00 at the House's. `outside` counts"
    );
    println!("  reports that fell beyond both chambers — the only column in which conference is");
    println!("  doing something neither chamber proposed.");
    Ok(())
}
