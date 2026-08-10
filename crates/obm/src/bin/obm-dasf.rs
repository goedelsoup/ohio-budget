//! Extracts rows from OBM's Detailed Appropriation Summary by Fund.
//!
//! ```text
//! obm-dasf <file.pdf> <fiscal-year> [--yaml] [--dump <page>]
//! ```
//!
//! Reports rather than writes. Promotion into the corpus goes through `corpus-promote`, and
//! this binary's job is to say what the document yields and what it could not read.

use anyhow::{bail, Result};
use corpus_schema::Provenance;
use obm::dasf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.first() else {
        bail!("usage: obm-dasf <file.pdf> <fiscal-year> [--yaml] [--dump <page>]");
    };
    let bytes = std::fs::read(path)?;

    if let Some(page) = args
        .iter()
        .position(|a| a == "--dump")
        .and_then(|p| args.get(p + 1))
        .and_then(|s| s.parse::<u32>().ok())
    {
        return dump(&bytes, page);
    }

    let Some(year) = args.get(1).filter(|a| !a.starts_with("--")) else {
        bail!("a fiscal year is required: obm-dasf <file.pdf> FY2021");
    };
    let provenance = Provenance {
        catalog_slug: "obm-dasf".to_string(),
        document_ref: path.clone(),
        locator: None,
        retrieved: "unrecorded".to_string(),
    };

    let (mut rows, mut problems, mut refused) = (Vec::new(), Vec::new(), 0usize);
    for (page, table) in dasf::tables(&bytes)? {
        let table = match table {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("page {page}: {e:#}"));
                continue;
            }
        };
        let text = table
            .rows
            .iter()
            .map(|r| r.join(" "))
            .collect::<Vec<_>>()
            .join("\n");
        // Before anything is parsed. A page that did not decode yields figures that are
        // well-formed and wrong, so it must not reach the parser at all.
        if let Err(e) = dasf::verify_decode(&text) {
            refused += 1;
            if refused == 1 {
                problems.push(format!("page {page}: {e:#}"));
            }
            continue;
        }
        let report = dasf::rows_from_table(&table, year, &provenance);
        problems.extend(
            report
                .problems
                .into_iter()
                .map(|p| format!("page {page}: {p}")),
        );
        rows.extend(report.rows);
    }

    if args.iter().any(|a| a == "--yaml") && !rows.is_empty() {
        println!("{}", serde_yaml::to_string(&rows)?);
        return Ok(());
    }

    println!("{} row(s) read from {path}", rows.len());
    if refused > 0 {
        println!(
            "{refused} page(s) refused because they did not decode — no figures were read \
             from them"
        );
    }
    for problem in problems.iter().take(10) {
        println!("  {problem}");
    }
    if problems.len() > 10 {
        println!("  … and {} more", problems.len() - 10);
    }
    if rows.is_empty() {
        bail!("nothing was extracted");
    }
    Ok(())
}

fn dump(bytes: &[u8], want: u32) -> Result<()> {
    for (page, table) in dasf::tables(bytes)? {
        if page != want {
            continue;
        }
        match table {
            Ok(t) => {
                println!("page {page}: {} row(s)", t.rows.len());
                for row in &t.rows {
                    println!("{row:?}");
                }
            }
            Err(e) => println!("page {page}: {e:#}"),
        }
    }
    Ok(())
}
