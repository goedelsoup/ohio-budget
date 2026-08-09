---
slug: lsc-hb110-appropriation-spreadsheet
name: LSC appropriation spreadsheets, HB 110 as enrolled (134th General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/budget/134/main-operating-budget
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - expenditure
  - line-item
---

# LSC appropriation spreadsheets — HB 110

The FY2022-23 biennium, the corpus's fourth.

| File | sha256 | Contents |
|---|---|---|
| [`hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx`](../sources/lsc/hb110-appropriation-spreadsheet-as-enrolled-134th.xlsx) | `77d0cbc3…` | sheet `EN`: 18 stage columns × FY2022-23 |
| [`hb110-appropriation-spreadsheet-with-actuals-134th.xlsx`](../sources/lsc/hb110-appropriation-spreadsheet-with-actuals-134th.xlsx) | `ba6ab512…` | sheet `Update 9.30.22`: actuals for FY2020, FY2021, FY2022 and an adjusted appropriation for FY2023 |

- **Retrieved:** 2026-08-09

Both classified with no changes to the connector — the first workbooks to do so, after HB 166
required four new label forms.

## It is the reason a $3.5 billion error was caught

Its actuals overlap HB 166's on FY2020 and HB 33's on FY2022. On four line items the three
workbooks agree **to the cent**. On the fifth they did not, and the difference was exactly the
state share of Medicaid — which is how a corpus node claiming a 25% Medicaid underspend was
found to be reading a memorandum sub-row. See
[appropriation-is-not-distribution](../decisions/appropriation-is-not-distribution.yml).

One smaller disagreement survives and is worth stating because this repository insists on exact
cents. Medicaid FY2022 actual reads **$15,710,496,829.68** here and **$15,710,496,830.00** in
HB 33's workbook — the later publication rounds to whole dollars. Thirty-two cents is not an
analytical problem, but it means the two sources are not interchangeable at the precision the
corpus stores, and a check that demands byte equality between them will fail for a reason that
has nothing to do with either being wrong. The corpus takes the unrounded figure.

Overlapping coverage is the property that makes this source family self-checking, and it is
worth preferring a workbook that repeats a year already held over one that only extends the
range.

## The stage pattern, fourth measurement

Both chamber floors moved zero dollars in FY2022 and FY2023, and the conference-to-enacted delta
is zero. So HB 166 remains the sole exception across four biennia — see
[house-finance-committee](../corpus/actor/house-finance-committee.yml).
