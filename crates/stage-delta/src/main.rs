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
    print!("{}", stage_delta::run(&root)?);
    Ok(())
}
