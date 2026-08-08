//! `lsc-extract` — reconstructs the table on one page of a comparison document.
//!
//! Exists so the geometry can be pointed at a real document and inspected. Tuning
//! [`TableOptions`] against actual LSC layouts is not something synthetic tests can do.

use std::path::PathBuf;

use anyhow::{bail, Result};
use lsc::geometry::TableOptions;

use lsc::map_columns as map;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path: PathBuf = args.next().map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("usage: lsc-extract <pdf> [page] [--river R] [--rows N]");
        std::process::exit(2)
    });
    let want_page: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let rest: Vec<String> = args.collect();

    let mut opts = TableOptions::default();
    let mut max_rows = 12usize;
    let mut raw = false;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--river" => {
                opts.min_river_ratio = rest[i + 1].parse()?;
                i += 2;
            }
            "--raw" => {
                raw = true;
                i += 1;
            }
            "--rows" => {
                max_rows = rest[i + 1].parse()?;
                i += 2;
            }
            other => bail!("unknown flag {other}"),
        }
    }

    if path.extension().is_some_and(|e| e == "xlsx") {
        let sheets = lsc::xlsx::sheet_names(&path)?;
        let sheet = std::env::var("SHEET").unwrap_or_else(|_| sheets[0].clone());
        println!("sheets: {sheets:?}; reading {sheet}");
        let t = lsc::xlsx::sheet_to_table(&path, &sheet)?;
        println!("{} column(s): {:?}", t.headers.len(), t.headers);
        match crate::map(&t.headers) {
            Ok(m) => println!(
                "mapped: agency={} ali={} name={} fiscal_years={:?}",
                m.agency_code,
                m.line_item_code,
                m.line_item_name,
                m.fiscal_years
                    .iter()
                    .map(|f| &f.fiscal_year)
                    .collect::<Vec<_>>()
            ),
            Err(e) => println!("column mapping failed: {e}"),
        }
        println!("{} data row(s)\n", t.rows.len());
        for (n, row) in t.rows.iter().take(max_rows).enumerate() {
            println!("{n:>3} {row:?}");
        }
        return Ok(());
    }

    let bytes = std::fs::read(&path)?;
    let pages = lsc::pdf::extract_pages(&bytes)?;
    println!(
        "{} page(s); river ratio {}",
        pages.len(),
        opts.min_river_ratio
    );

    let Some(page) = pages.iter().find(|p| p.number == want_page) else {
        bail!("page {want_page} not present");
    };
    println!("page {} has {} glyph(s)\n", page.number, page.glyphs.len());

    if raw {
        let empty = page
            .glyphs
            .iter()
            .filter(|g| g.text.trim().is_empty())
            .count();
        println!(
            "{} glyph(s), {} with empty/whitespace text ({:.0}%)",
            page.glyphs.len(),
            empty,
            empty as f64 / page.glyphs.len().max(1) as f64 * 100.0
        );
        for g in page.glyphs.iter().take(40) {
            println!(
                "  x={:>8.2} y={:>8.2} adv={:>6.2} fs={:>5.2} {:?}",
                g.x, g.y, g.advance, g.font_size, g.text
            );
        }
        return Ok(());
    }

    match lsc::pdf::page_to_table(page, &opts) {
        Err(e) => println!("could not reconstruct: {e:#}"),
        Ok(t) => {
            println!("{} column(s): {:?}\n", t.headers.len(), t.headers);
            for (n, row) in t.rows.iter().take(max_rows).enumerate() {
                println!("{n:>3} {row:?}");
            }
            if t.rows.len() > max_rows {
                println!("... {} more row(s)", t.rows.len() - max_rows);
            }
        }
    }
    Ok(())
}
