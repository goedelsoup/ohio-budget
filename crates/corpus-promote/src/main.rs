//! `corpus-promote` — reports what an extraction run would change, and writes nothing.
//!
//! Dry-run only, deliberately. Every proposal is a factual claim about Ohio's budget being
//! attached to a node, and the failure mode of an automatic writer is a corpus that fills
//! itself with plausible figures nobody read.

use std::path::PathBuf;

use anyhow::{bail, Result};
use corpus_promote::{propose_from_lsc, render};
use lsc::Source;

fn main() -> Result<()> {
    let repo_root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if !repo_root.join(".yidam/corpus").is_dir() {
        bail!("no corpus at {}", repo_root.display());
    }

    let corpus = corpus_validate::load(&repo_root)?;
    let rows = lsc::FixtureSource::from_repo(&repo_root).fetch_all()?;

    println!(
        "loaded {} appropriation node(s) and {} extraction row(s)\n",
        corpus
            .instances
            .iter()
            .filter(|i| i.inst.class == "appropriation")
            .count(),
        rows.len()
    );

    let report = propose_from_lsc(&corpus, &rows);
    print!("{}", render(&report));

    if !report.proposals.is_empty() {
        println!("\nNothing was written. Review each proposal before applying it by hand.");
    }
    if !report.refused.is_empty() {
        println!(
            "\n{} row(s) refused. Running against fixtures is expected to refuse everything — \
             that is the guard working, not a failure.",
            report.refused.len()
        );
    }
    Ok(())
}
