//! Reports gap coverage across the corpus: what is computable, what is blocked, and why.
//!
//! With every amount still `[open]` the useful output is the second column — this is the
//! repository's own inventory of what extraction has to deliver before its central question
//! can be answered at all.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Result};
use gap::{gap_for, Outcome};

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if !root.join(".yidam/corpus").is_dir() {
        bail!("no corpus at {}", root.display());
    }
    let corpus = corpus_validate::load(&root)?;

    // Every (line item, period) pair the corpus holds an expenditure for — the only pairs
    // where a gap could exist at all.
    let mut pairs: Vec<(String, String)> = Vec::new();
    for e in corpus
        .instances
        .iter()
        .filter(|i| i.inst.class == "expenditure")
    {
        let Some(period) = corpus_validate::property_text(&e.inst, "period_label") else {
            continue;
        };
        let dir = e.abs_path.parent().unwrap_or(&e.abs_path);
        for l in e
            .inst
            .links
            .iter()
            .filter(|l| l.relationship == "disburses-against")
        {
            let t = corpus_validate::normalize_join(dir, &l.target);
            if let Some(name) = t.file_stem().and_then(|s| s.to_str()) {
                pairs.push((name.to_string(), period.trim().to_string()));
            }
        }
    }
    pairs.sort();
    pairs.dedup();

    let mut computed = 0usize;
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
    println!(
        "{} (line item, period) pair(s) with both sides present in principle\n",
        pairs.len()
    );

    for (li, period) in &pairs {
        match gap_for(&corpus, li, period) {
            Outcome::Computed(r) => {
                computed += 1;
                println!(
                    "  COMPUTED {li} {period}: variance {} cents ({:.1}%) — {}",
                    r.variance_cents,
                    r.variance_pct,
                    r.character.how_to_read()
                );
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
            "\nNo gap is computable yet. Every amount in the corpus is [open], which is the \n\
             honest state rather than a failure: the figures require extraction from sources \n\
             whose content is not committed."
        );
    }
    Ok(())
}
