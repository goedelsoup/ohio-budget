//! Reports gap coverage across the corpus: what is computable, what is blocked, and why.
//!
//! The second column is the substance of the report — this is the repository's own inventory
//! of what extraction still has to deliver before its central question can be answered in
//! more than the handful of places it currently can.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Result};
use gap::{Coverage, Outcome};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let root = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if !root.join(".yidam/corpus").is_dir() {
        bail!("no corpus at {}", root.display());
    }

    let coverage = gap::all(&root)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&coverage)?);
        return Ok(());
    }

    let mut computed = 0usize;
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
    println!(
        "{} (line item, period) pair(s) with both sides present in principle\n",
        coverage.len()
    );

    for Coverage {
        line_item: li,
        period,
        outcome,
    } in &coverage
    {
        match outcome {
            Outcome::Computed(r) => {
                computed += 1;
                println!(
                    "  COMPUTED {li} {period}: variance {} cents ({}) — {}",
                    r.variance_cents,
                    r.variance_pct
                        .map(|p| format!("{p:.1}%"))
                        .unwrap_or_else(|| "no authority to take a share of".into()),
                    r.character.how_to_read()
                );
                if let Some(a) = &r.anomaly {
                    println!("    ANOMALY: {a}");
                }
            }
            Outcome::Unavailable { reason } => {
                println!("  BLOCKED   {li} {period}: {reason}");
                *by_reason.entry("unavailable".into()).or_default() += 1;
            }
            Outcome::Refused { reason } => {
                println!("  REFUSED   {li} {period}: {reason}");
                *by_reason.entry("refused".into()).or_default() += 1;
            }
        }
    }

    println!("\n{computed} computable, {by_reason:?}");
    if computed == 0 {
        println!(
            "\nNo gap is computable yet. The figures require extraction from sources whose \n\
             content is not committed, which is the honest state rather than a failure."
        );
    }
    Ok(())
}
