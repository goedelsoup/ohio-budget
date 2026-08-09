//! `leadership` — renders [`stage_delta::process`].
//!
//! # What it was built to answer, and what happened to the question
//!
//! [`leadership-and-the-anomalies`](../../../../.yidam/decisions/leadership-and-the-anomalies.yml)
//! recorded that the House substitute's direction on foundation funding splits exactly with the
//! Speaker across four biennia, and refused to treat it as a finding: four observations, two per
//! side, with the Fair School Funding Plan phasing in over the same window as a confound of
//! identical shape. That record twice concluded the separating test needed *older* biennia, and
//! was twice wrong — LSC published no stage columns before FY2020-21.
//!
//! The separating test needed more line items in the *same* biennia, and every committed workbook
//! from the 133rd on carries all nine stages for roughly 1,400 of them.
//!
//! Both hypotheses died. The pattern appears just as strongly outside anything the plan can
//! reach, and Matt Huffman — the one officer who presides more than once — behaves differently
//! each time. What replaced them is in [the unit of observation]
//! (../../../../.yidam/decisions/the-unit-of-observation.yml): every actor reverses the one
//! before it, which is a property of the process rather than of anybody presiding.
//!
//! The name is a fossil of the question, kept because the decision records refer to it.
//! All computation lives in the library so the feed and this renderer cannot disagree.

use std::path::PathBuf;

use anyhow::{bail, Result};
use stage_delta::process::{self, ProcessFindings};

fn pct(v: Option<f64>) -> String {
    v.map(|p| format!("{p:5.1}%"))
        .unwrap_or_else(|| "    —".into())
}

fn share(part: i128, whole: i128) -> String {
    if whole > 0 {
        format!("{:5.1}%", part as f64 / whole as f64 * 100.0)
    } else {
        "    —".into()
    }
}

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let Some(ProcessFindings {
        conditioning,
        conference,
        plan_reaches,
    }) = process::analyse(&root)?
    else {
        bail!("no committed LSC workbooks under {}", root.display());
    };

    for h in process::hand_offs() {
        let rows: Vec<_> = conditioning.iter().filter(|c| c.actor == h.actor).collect();
        println!("\n═══ The {} answering {}\n", h.actor, h.answering);
        println!(
            "  {:<7} {:<13} │ {:>24} │ {:>24} │ {:>8} {:>8}",
            "bill", "officer", "predecessor RAISED it", "predecessor CUT it", "gap", "toward"
        );
        println!(
            "  {:<7} {:<13} │ {:>8} {:>7} {:>7} │ {:>8} {:>7} {:>7} │",
            "", "", "n", "up", "up%", "n", "up", "up%"
        );
        println!("  {}", "─".repeat(94));
        let (mut ra, mut rn, mut ca, mut cn) = (0usize, 0usize, 0usize, 0usize);
        for c in &rows {
            ra += c.when_prior_raised.raised;
            rn += c.when_prior_raised.moved();
            ca += c.when_prior_cut.raised;
            cn += c.when_prior_cut.moved();
            println!(
                "  {:<7} {:<13} │ {:>8} {:>7} {:>7} │ {:>8} {:>7} {:>7} │ {:>8} {:>8}",
                c.bill,
                c.officer.as_deref().unwrap_or("—"),
                c.when_prior_raised.moved(),
                c.when_prior_raised.raised,
                pct(c.when_prior_raised.raised_share()),
                c.when_prior_cut.moved(),
                c.when_prior_cut.raised,
                pct(c.when_prior_cut.raised_share()),
                c.reversal_gap()
                    .map(|g| format!("{g:+.1}pp"))
                    .unwrap_or_else(|| "—".into()),
                pct(c.toward_share()),
            );
        }
        println!("  {}", "─".repeat(94));
        let (rs, cs) = (
            (rn > 0).then(|| ra as f64 / rn as f64 * 100.0),
            (cn > 0).then(|| ca as f64 / cn as f64 * 100.0),
        );
        println!(
            "  {:<7} {:<13} │ {:>8} {:>7} {:>7} │ {:>8} {:>7} {:>7} │ {:>8}",
            "POOLED",
            "",
            rn,
            ra,
            pct(rs),
            cn,
            ca,
            pct(cs),
            match (rs, cs) {
                (Some(r), Some(c)) => format!("{:+.1}pp", c - r),
                _ => "—".into(),
            },
        );
    }

    println!("\n═══ Where conference lands when the chambers disagree\n");
    println!(
        "  {:<7} {:<6} │ {:>8} {:>8} {:>8} {:>8} {:>8} │ {:>8}",
        "bill", "GA", "contested", "House", "Senate", "between", "outside", "median"
    );
    println!("  {}", "─".repeat(78));
    let mut total = process::ConferencePosition::default();
    for c in &conference {
        total.contested += c.contested;
        total.at_house += c.at_house;
        total.at_senate += c.at_senate;
        total.between += c.between;
        total.outside_both += c.outside_both;
        total.contested_cents += c.contested_cents;
        total.cents_at_house += c.cents_at_house;
        total.cents_at_senate += c.cents_at_senate;
        total.cents_between += c.cents_between;
        total.cents_outside += c.cents_outside;
        println!(
            "  {:<7} {:<6} │ {:>8} {:>8} {:>8} {:>8} {:>8} │ {:>8}",
            c.bill,
            c.general_assembly,
            c.contested,
            c.at_house,
            c.at_senate,
            c.between,
            c.outside_both,
            c.median_position
                .map(|m| format!("{m:.2}"))
                .unwrap_or_else(|| "—".into())
        );
    }
    println!("  {}", "─".repeat(78));
    println!(
        "  {:<7} {:<6} │ {:>8} {:>8} {:>8} {:>8} {:>8} │",
        "POOLED",
        "",
        total.contested,
        total.at_house,
        total.at_senate,
        total.between,
        total.outside_both
    );
    println!(
        "  {:<7} {:<6} │ {:>8} {:>8} {:>8} {:>8} {:>8} │  by contested dollars",
        "",
        "",
        "",
        share(total.cents_at_house, total.contested_cents),
        share(total.cents_at_senate, total.contested_cents),
        share(total.cents_between, total.contested_cents),
        share(total.cents_outside, total.contested_cents),
    );

    println!("\n  `toward` is the share of moved line items that landed closer to the level the");
    println!("  predecessor started from — the test that separates reversing from restoring.");
    println!("  School funding partition: {plan_reaches:?}. Every figure is within one fiscal");
    println!("  year, so no deflator is involved and no sign depends on the price level.\n");
    Ok(())
}
