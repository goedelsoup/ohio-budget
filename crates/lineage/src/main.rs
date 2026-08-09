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

    // The slice check reads a workbook rather than the corpus, so it is skipped where the
    // sources are absent rather than failing the run.
    let fy = std::env::var("FY").unwrap_or_else(|_| "FY2026".into());
    let workbook =
        root.join(".yidam/sources/lsc/hb96-appropriation-spreadsheet-as-enacted-136th.xlsx");
    if workbook.is_file() {
        let findings = lineage::slices::check(&corpus, &workbook, &fy)?;
        print!("{}", lineage::slices::render(&findings, &fy));
        println!();
    }

    let report = lineage::propose(&corpus);
    print!("{}", lineage::render(&report));
    println!(
        "\nNothing was written. A wrong lineage edge fuses two unrelated funding histories \n\
         into one series, and nothing downstream can detect it."
    );
    Ok(())
}
