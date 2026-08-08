---
slug: lsc-hb96-appropriation-spreadsheet
name: LSC appropriation spreadsheet, HB 96 as enacted (136th General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/assets/legislation/136/hb96/en0/files/hb96-appropriation-spreadsheet-as-enacted-136th-general-assembly-10080152.xlsx
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - line-item
  - agency
  - fund
---

# LSC appropriation spreadsheet — HB 96 as enacted

The first source in this repository whose content is actually committed, and therefore the
first that can support a `[verified]` claim.

- **Retrieved:** 2026-08-08
- **Committed at:** [`.yidam/sources/lsc/hb96-appropriation-spreadsheet-as-enacted-136th.xlsx`](../sources/lsc/hb96-appropriation-spreadsheet-as-enacted-136th.xlsx)
- **sha256:** `8153e8f55c2b38c670430261d644ffcfe3f38f8c4c04a9438aae9855db0b9d92`
- **Size:** 463,639 bytes

## What it contains

Five sheets. `EN` is all funds; `ENGRF` is general revenue only and holds 479 line items.
Columns run: agency, fund group, fund, ALI, ALI name, then FY2024 actual, an FY2025 OBM
estimate, and **every stage of the bill for both fiscal years** — introduced, House
substitute, House reported, House passed, Senate substitute, Senate reported, Senate passed,
conference report, and as enacted.

## Why the xlsx and not the PDF

Both are published. The PDF was tried first and cannot be read: it embeds a subset font with
no usable ToUnicode mapping, so 93% of its glyphs decode to empty strings. The table geometry
recovers perfectly and every cell comes back blank.

That is a property of this document rather than a limitation of the extractor — the
per-agency comparison documents decode at 87% and reconstruct correctly. The right response
was to read the structured sibling instead of fighting the font, which is why
[`lsc::xlsx`](../../crates/lsc/src/xlsx.rs) exists.

## What it corrected

Two claims this corpus had carried as `[inference]`:

- **ALI 200550 is Foundation Funding** — confirmed. The sheet names it "Foundation Funding -
  All Students", agency `EDU`, fund `GRF`.
- **Agency codes are not numeric** — refuted. LSC identifies agencies by three-letter code
  (`EDU`, `ADJ`, `DEW`), not by the numeric prefix this corpus had assumed. The `200` in
  `200550` is part of the line item code, not an agency identifier, and the corpus was
  conflating two systems.

## What it settles

The stage series for foundation funding in FY2026 shows the conference report and the enacted
figure are **identical**. Foundation funding was therefore not touched by the governor's
line-item vetoes — an open question in
[`hb96-line-item-veto`](../corpus/budget-action/hb96-line-item-veto.yml) since genesis.

## Caution for extraction

This one spreadsheet carries every stage as separate *columns*, which the
[`lsc-comparison-row`](../schemas/extraction/lsc-comparison-row.schema.json) contract does not
anticipate — it assumes one stage per document and carries stage in `TableContext`. Column
headers here encode stage and fiscal year together ("House Passed FY 2026"), so
`map_columns` currently collapses them and reports `FY2026` five times. Extracting this source
properly needs per-column stage detection.
