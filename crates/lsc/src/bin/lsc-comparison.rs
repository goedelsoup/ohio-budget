//! `lsc-comparison` — reads an LSC comparison document into provisions and positions.
//!
//! Companion to `lsc-extract`, which reads the appropriation spreadsheet. That one answers
//! what a figure became; this one answers why.

use std::path::PathBuf;

use anyhow::{bail, Result};
use lsc::comparison::{assemble, to_rows, DocumentContext};
use lsc::ruled::RuleOptions;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next().map(PathBuf::from) else {
        eprintln!(
            "usage: lsc-comparison <pdf> [--provision CODE] [--changed] [--emit FILE]\n\
             env: BILL GA CATALOG RETRIEVED"
        );
        std::process::exit(2);
    };

    let mut want: Option<String> = None;
    let mut changed_only = false;
    let mut emit: Option<PathBuf> = None;
    let rest: Vec<String> = args.collect();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--provision" => {
                want = rest.get(i + 1).cloned();
                i += 2;
            }
            "--changed" => {
                changed_only = true;
                i += 1;
            }
            "--emit" => {
                emit = rest.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            other => bail!("unknown flag {other}"),
        }
    }

    let bytes = std::fs::read(&path)?;
    let pages: Vec<(u32, Vec<lsc::geometry::Glyph>)> = lsc::pdf::extract_pages(&bytes)?
        .into_iter()
        .map(|p| (p.number, p.glyphs))
        .collect();

    let doc = assemble(&pages, &RuleOptions::default())?;

    println!(
        "{} page(s): {} read directly, {} inherited rules from the page before, {} refused",
        pages.len(),
        pages.len() - doc.continuation_pages.len() - doc.refused_pages.len(),
        doc.continuation_pages.len(),
        doc.refused_pages.len()
    );
    for (n, why) in &doc.refused_pages {
        println!("  REFUSED page {n}: {why}");
    }
    println!(
        "columns: {}",
        doc.columns
            .iter()
            .map(|c| match c.stage {
                Some(s) => format!("{} -> {}", c.label, s.as_str()),
                None => format!("{} -> UNMAPPED", c.label),
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "{} provision(s), {} entr(ies)",
        doc.provisions.len(),
        doc.provisions
            .iter()
            .map(|p| p.positions.len())
            .sum::<usize>()
    );

    let ctx = DocumentContext {
        bill_number: std::env::var("BILL").unwrap_or_else(|_| "HB 96".into()),
        general_assembly: std::env::var("GA").unwrap_or_else(|_| "136th".into()),
        catalog_slug: std::env::var("CATALOG").unwrap_or_else(|_| "lsc-hb96-comparison".into()),
        retrieved: std::env::var("RETRIEVED").unwrap_or_else(|_| "2026-08-08".into()),
    };
    let report = to_rows(&doc, &ctx);
    println!(
        "{} record(s); {} column(s) printed nothing; {} provision(s) had no LSC code",
        report.rows.len(),
        report.empty_cells,
        report.uncoded_provisions.len()
    );
    for c in &report.unmapped_columns {
        println!("  UNMAPPED COLUMN {c:?}: its prose is not emitted");
    }
    for t in report.uncoded_provisions.iter().take(5) {
        println!("  UNCODED {:?}", &t[..t.len().min(70)]);
    }

    if let Some(code) = &want {
        let Some(p) = doc.find(code) else {
            bail!("no provision {code} in this document");
        };
        println!(
            "\n=== {code} {} ===\nsection {:?}, page {}, heading veto marker: {:?}",
            p.title, p.section, p.page, p.veto_status
        );
        for (i, pos) in p.positions.iter().enumerate() {
            if changed_only && pos.is_unchanged_after_executive() {
                continue;
            }
            println!("\n--- entry {i} (page {}) ---", pos.page);
            for (c, cell) in pos.cells.iter().enumerate() {
                if cell.trim().is_empty() {
                    continue;
                }
                let split = lsc::comparison::split_inline_veto(cell);
                println!("  [{}] {}", doc.columns[c].label, split.text);
                for s in &split.struck {
                    println!(
                        "        VETOED{}: {s}",
                        if split.unterminated {
                            " (extent uncertain)"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
    } else if changed_only {
        for p in &doc.provisions {
            let n = p
                .positions
                .iter()
                .filter(|x| !x.is_unchanged_after_executive())
                .count();
            if n > 0 {
                println!(
                    "  {:<10} {:<58} {n} changed",
                    p.code.clone().unwrap_or_else(|| "-".into()),
                    &p.title[..p.title.len().min(58)]
                );
            }
        }
    }

    if let Some(out) = emit {
        std::fs::write(&out, serde_yaml::to_string(&report.rows)?)?;
        println!(
            "\nwrote {} record(s) to {}",
            report.rows.len(),
            out.display()
        );
    }
    Ok(())
}
