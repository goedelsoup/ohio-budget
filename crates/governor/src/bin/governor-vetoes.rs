//! `governor-vetoes` — reads a veto message into one record per item.

use std::path::PathBuf;

use anyhow::{bail, Result};
use governor::{parse, MessageContext};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next().map(PathBuf::from) else {
        eprintln!(
            "usage: governor-vetoes <pdf|txt> [--item N] [--emit FILE]\n\
                   env: BILL GA CATALOG RETRIEVED"
        );
        std::process::exit(2);
    };
    let rest: Vec<String> = args.collect();
    let mut want: Option<u32> = None;
    let mut emit: Option<PathBuf> = None;
    let mut dump: Option<PathBuf> = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--item" => {
                want = rest.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--emit" => {
                emit = rest.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            // The extracted text is the parser's actual input, and it is not the same as what
            // another PDF tool would produce. Being able to see it is the difference between
            // diagnosing a parse and guessing at one.
            "--dump-text" => {
                dump = rest.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            other => bail!("unknown flag {other}"),
        }
    }

    let bytes = std::fs::read(&path)?;
    let text = if path.extension().is_some_and(|e| e == "pdf") {
        governor::pdf::text_from_pdf(&bytes)?
    } else {
        String::from_utf8(bytes)?
    };

    if let Some(out) = &dump {
        std::fs::write(out, &text)?;
        println!("wrote extracted text to {}", out.display());
    }

    let ctx = MessageContext {
        bill_number: std::env::var("BILL").unwrap_or_else(|_| "HB 96".into()),
        general_assembly: std::env::var("GA").unwrap_or_else(|_| "136th".into()),
        catalog_slug: std::env::var("CATALOG")
            .unwrap_or_else(|_| "governor-hb96-veto-messages".into()),
        retrieved: std::env::var("RETRIEVED").unwrap_or_else(|_| "2026-08-08".into()),
    };
    let r = parse(&text, &ctx)?;

    println!(
        "{} item(s), {} deletion instruction(s)",
        r.items.len(),
        r.deletion_count()
    );
    for (label, v) in [
        (
            "missing from the numbering",
            format!("{:?}", r.missing_item_numbers),
        ),
        (
            "no deletion instruction",
            format!("{:?}", r.items_without_deletions),
        ),
        (
            "reason lacks the closing formula",
            format!("{:?}", r.items_missing_closing_formula),
        ),
    ] {
        if !v.is_empty() && v != "[]" {
            println!("  {label}: {v}");
        }
    }
    for (n, s) in &r.unrecognised_deletions {
        println!("  UNRECOGNISED item {n}: {s}");
    }
    for (n, s) in &r.deletions_with_repaired_quotes {
        println!("  QUOTE REPAIRED item {n}: {s}");
    }
    let rejoined: Vec<u32> = r
        .items
        .iter()
        .filter(|i| i.title_rejoined)
        .map(|i| i.item_number)
        .collect();
    if !rejoined.is_empty() {
        println!("  headings rejoined across lines: {rejoined:?}");
    }
    if r.is_clean() {
        println!("  clean: every item numbered, deleted something, and gave a reason");
    }

    // An item veto may not add money, so any figure here would be a red flag worth surfacing.
    let with_amounts: Vec<u32> = r
        .items
        .iter()
        .filter(|i| {
            i.deletions.iter().any(|d| match &d.extent {
                corpus_schema::DeletionExtent::Text { text } => text.contains('$'),
                corpus_schema::DeletionExtent::Range { begins, ends } => {
                    begins.contains('$') || ends.contains('$')
                }
                _ => false,
            })
        })
        .map(|i| i.item_number)
        .collect();
    println!("  items whose deleted text quotes a dollar figure: {with_amounts:?}");

    if let Some(n) = want {
        let Some(it) = r.items.iter().find(|i| i.item_number == n) else {
            bail!("no item {n}");
        };
        println!("\n=== ITEM {n}: {} ===", it.title);
        for d in &it.deletions {
            println!("  page {:?}: {:?}", d.bill_page, d.extent);
        }
        println!("\n{}", it.rationale);
    }

    if let Some(out) = emit {
        std::fs::write(&out, serde_yaml::to_string(&r.items)?)?;
        println!("\nwrote {} item(s) to {}", r.items.len(), out.display());
    }
    Ok(())
}
