---
slug: lsc-hb33-actuals
name: LSC appropriation spreadsheet with actual expenditures and adjusted appropriations, HB 33
source_type: financial-report
location: https://www.lsc.ohio.gov/assets/legislation/135/hb33/en0/files/hb33-appropriation-spreadsheet-with-actual-expenditures-and-adjusted-appropriations-as-enacted-135th-general-assembly-10077872.xlsx
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - expenditure
  - appropriation
---

# LSC appropriation spreadsheet with actuals — HB 33

- **Retrieved:** 2026-08-08
- **Committed at:** [`.yidam/sources/lsc/hb33-appropriation-spreadsheet-with-actuals-135th.xlsx`](../sources/lsc/hb33-appropriation-spreadsheet-with-actuals-135th.xlsx)
- **sha256:** `c979accc1fbaca183997dc229ec0b548569c927c39684fcd292f2bbc2a1fd801`
- **Stated as of:** September 30, 2024

## The expenditure side

FY2022, FY2023, and FY2024 are actual expenditures; FY2025 is an **adjusted** appropriation
rather than an actual, because the year had not closed when the sheet was published. The
column header says so, and the distinction is exactly the one
[`gap`](../skills/gap.md) refuses to let a caller ignore.

## Adjusted, not enacted

The FY2025 column being *adjusted* is the corpus's first direct evidence of the execution
phase in a figure. An adjusted appropriation is the enacted amount plus whatever controlling
board action, supplemental, or transfer has since moved it. Enacted, adjusted, and actual are
three numbers, and the corpus has so far modelled only the first and third.

## Layout

Headers sit on the **second** row, under a title row. Reading the first non-empty row as
headers shifts every figure by one row and produces a table that parses cleanly and is
entirely false. `lsc::xlsx::find_header_row` scores rows by how well their cells classify
rather than trusting position, and this document is why.
