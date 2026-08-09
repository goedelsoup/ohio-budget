---
slug: lsc-hb33-appropriation-spreadsheet
name: LSC appropriation spreadsheet, HB 33 as enacted (135th General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/assets/legislation/135/hb33/en0/files/hb33-appropriation-spreadsheet-as-enacted-135th-general-assembly-10077370.xlsx
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - line-item
---

# LSC appropriation spreadsheet — HB 33 as enacted

- **Retrieved:** 2026-08-08
- **Committed at:** [`.yidam/sources/lsc/hb33-appropriation-spreadsheet-as-enacted-135th.xlsx`](../sources/lsc/hb33-appropriation-spreadsheet-as-enacted-135th.xlsx)
- **sha256:** `140d62e98eaa2e2e8d8296c5f72758248743860ea6e98e852a32a2c886f0caa6`

**Three sheets, and which one you open changes what this source appears to be.**

| Sheet | Columns |
|---|---|
| `Final` | agency, fund group, fund, ALI, ALI name, FY2022–23 actuals, **enacted only** for FY2024 and FY2025 |
| `EN` | the same identity columns, FY2022–23 actuals, and **all nine stages** × two fiscal years — introduced, House substitute, House reported, House passed, Senate substitute, Senate reported, Senate passed, conference, enacted |
| `ENGRF` | as `EN`, restricted to the general revenue fund |

`EN` classifies as 18 appropriation columns and 2 actuals across 1,591 rows, with nothing
unclassified.

## This entry described only the summary sheet, and a question was closed as unanswerable

An earlier reading opened `Final`, found nine columns, and concluded that the workbook carried
enacted figures only — from which it followed that LSC had changed publication format between
the 135th and 136th General Assemblies and that no cross-biennium stage comparison was
possible. Two `actor` nodes recorded the question as **blocked on an unreachable source**.

The source was in this repository, and `EN` had the answer. Nothing external was missing; a
sheet went unopened and the conclusion was generalised from the one that was.

Worth noting what made it durable: "blocked" reads as a finished investigation. An `[open]`
question invites someone to look again, and a blocked one tells them not to bother. Recording
a limit is useful only if the limit is real, and the cost of getting it wrong is that nobody
retries.

The comparison it unblocked is in
[house-finance-committee](../corpus/actor/house-finance-committee.yml): across both bills and
all four fiscal years, neither chamber's floor moved a dollar.

## A label that differed by one word

HB 96 heads the column `Conference Report`; HB 33 heads it `Conference`. `BillStage::parse_label`
accepted only the longer form, so every 135th General Assembly conference figure was dropped —
2 of 18 money columns, silently absent from the extraction rather than wrong in it.

It was visible only because `ColumnPlan::unclassified` reports the columns it could not place
instead of skipping them. A connector that quietly ignored unknown columns would have produced
a stage series with a hole in it and no indication.

## Why it was needed

The [HB 96 spreadsheet](./lsc-hb96-appropriation-spreadsheet.md) carries FY2024 as an
*actual*, not an appropriation — its appropriation columns cover FY2026 and FY2027 only. A
gap needs both sides for the same period, so the FY2024 authority had to come from the budget
that granted it.

That is a general property of this source family worth recording: each biennium's spreadsheet
holds appropriations for its own years and actuals for earlier ones. Any gap spanning a
biennium boundary reads two documents.
