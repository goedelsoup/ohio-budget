//! `corpus-export` — writes the JSON feed the web layer builds against.
//!
//! Usage: `corpus-export [repo-root] [--out <dir>]`
//! Default output is `web/src/data/`, which the Astro build reads directly.

use std::path::PathBuf;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut out: Option<PathBuf> = None;
    let mut positional: Vec<&String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--out requires a directory");
                };
                out = Some(PathBuf::from(v));
                i += 2;
            }
            a if a.starts_with("--") => bail!("unknown flag {a}"),
            _ => {
                positional.push(&args[i]);
                i += 1;
            }
        }
    }

    let root = positional
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if !root.join(".yidam/corpus").is_dir() {
        bail!("no corpus at {}", root.display());
    }
    let out = out.unwrap_or_else(|| root.join("web").join("src").join("data"));

    let feed = corpus_export::build(&root)?;
    let written = corpus_export::write(&feed, &out)?;

    let c = &feed.manifest.counts;
    println!(
        "contract {} @ {} — {} nodes across {} classes, {} catalog, {} decisions, {} skills",
        corpus_export::CONTRACT_VERSION,
        feed.manifest.commit.as_deref().unwrap_or("no-git"),
        c.nodes,
        c.classes,
        c.catalog,
        c.decisions,
        c.skills,
    );
    println!(
        "  {} node(s) carry a verified claim, {} carry an open one",
        c.verified_nodes, c.open_nodes
    );

    let computed = feed
        .findings
        .gap
        .iter()
        .filter(|c| c.outcome.is_computed())
        .count();
    println!(
        "  gap: {computed} of {} pair(s) computable; stage-delta: {} series",
        feed.findings.gap.len(),
        feed.findings.stage_delta.len(),
    );
    for path in written {
        println!("  → {}", path.display());
    }
    Ok(())
}
