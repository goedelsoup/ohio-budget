//! Reports succession candidates for review. There is no apply path.

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
    let corpus = corpus_validate::load(&root)?;
    let report = lineage::propose(&corpus);
    print!("{}", lineage::render(&report));
    println!(
        "\nNothing was written. A wrong lineage edge fuses two unrelated funding histories \n\
         into one series, and nothing downstream can detect it."
    );
    Ok(())
}
