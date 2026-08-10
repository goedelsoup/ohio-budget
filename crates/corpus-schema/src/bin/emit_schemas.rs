//! Generates the JSON Schemas under `.yidam/schemas/` from the Rust types.
//!
//! Run via `mise run schemas`. The emitted files are build output committed to the
//! repository so that editors and other-language tooling can consume them without a Rust
//! toolchain — but they are not the source of truth and must never be hand-edited.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use schemars::{schema_for, JsonSchema};

use corpus_schema::*;

fn emit<T: JsonSchema>(dir: &Path, name: &str) -> Result<()> {
    let schema = schema_for!(T);
    let mut json = serde_json::to_string_pretty(&schema)
        .with_context(|| format!("serializing schema for {name}"))?;
    json.push('\n');
    let path = dir.join(format!("{name}.schema.json"));
    std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    println!("  {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".yidam/schemas"));

    let corpus = root.join("corpus");
    let extraction = root.join("extraction");
    std::fs::create_dir_all(&corpus).context("creating corpus schema dir")?;
    std::fs::create_dir_all(&extraction).context("creating extraction schema dir")?;

    println!("corpus file schemas:");
    emit::<ClassDefinition>(&corpus, "class-definition")?;
    emit::<CorpusInstance>(&corpus, "corpus-instance")?;
    emit::<DecisionRecord>(&corpus, "decision-record")?;
    emit::<CatalogEntry>(&corpus, "catalog-entry")?;

    println!("connector extraction contracts:");
    emit::<LscComparisonRow>(&extraction, "lsc-comparison-row")?;
    emit::<LscProvisionRow>(&extraction, "lsc-provision-row")?;
    emit::<ObmExpenditureRow>(&extraction, "obm-expenditure-row")?;
    emit::<ObmBudgetaryRow>(&extraction, "obm-budgetary-row")?;
    emit::<ControllingBoardRequest>(&extraction, "controlling-board-request")?;
    emit::<LegislatureBill>(&extraction, "legislature-bill")?;
    emit::<VetoItemRow>(&extraction, "veto-item-row")?;

    Ok(())
}
