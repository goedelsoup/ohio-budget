//! `corpus-validate` — checks the corpus against its own ontology.
//!
//! Exits non-zero when any error-severity finding is present, so it can gate a commit.

use std::path::PathBuf;

use anyhow::{bail, Result};
use corpus_validate::{check, load, Severity};

fn main() -> Result<()> {
    let repo_root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let corpus_root = repo_root.join(".yidam").join("corpus");

    if !corpus_root.is_dir() {
        bail!("no corpus at {}", corpus_root.display());
    }

    let corpus = load(&repo_root)?;
    let findings = check(&corpus);

    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warns = findings.len() - errors;

    for f in &findings {
        println!(
            "[{}] {} — {} ({})",
            f.severity.label(),
            f.path,
            f.message,
            f.rule
        );
    }

    if !findings.is_empty() {
        println!();
    }
    println!(
        "checked {} instances across {} classes — {errors} error(s), {warns} warning(s)",
        corpus.instances.len(),
        corpus.classes.len()
    );

    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}
