//! `stage-delta` — decomposes appropriation movement across bill stages.

use std::path::PathBuf;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if !root.join(".yidam/corpus").is_dir() {
        bail!("no corpus at {}", root.display());
    }
    if let Ok(path) = std::env::var("ROWS") {
        let rows: Vec<corpus_schema::LscComparisonRow> =
            serde_yaml::from_str(&std::fs::read_to_string(&path)?)?;
        let fy = std::env::var("FY").unwrap_or_else(|_| "FY2026".into());
        println!("{} row(s) from {path}\n", rows.len());
        print!(
            "{}",
            stage_delta::render_aggregate(&stage_delta::aggregate(&rows, &fy), &fy)
        );
        return Ok(());
    }
    print!("{}", stage_delta::run(&root)?);
    Ok(())
}
