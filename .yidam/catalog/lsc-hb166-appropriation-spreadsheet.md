---
slug: lsc-hb166-appropriation-spreadsheet
name: LSC appropriation spreadsheets, HB 166 as enrolled (133rd General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/budget/133/main-operating-budget
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - expenditure
  - line-item
---

# LSC appropriation spreadsheets — HB 166

Two workbooks for the FY2020-21 biennium, the corpus's third.

| File | sha256 | Contents |
|---|---|---|
| [`hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx`](../sources/lsc/hb166-appropriation-spreadsheet-as-enrolled-133rd.xlsx) | `b5533209…` | sheet `EN`: 18 stage columns × FY2020-21, plus FY2018 actual and an FY2019 estimate |
| [`hb166-appropriation-spreadsheet-with-actuals-133rd.xlsx`](../sources/lsc/hb166-appropriation-spreadsheet-with-actuals-133rd.xlsx) | `ffeb622d…` | sheet `FY21Update`: actuals for FY2018, FY2019 and FY2020, and an adjusted appropriation for FY2021 |

- **Retrieved:** 2026-08-09

## It settles the pre-veto question

The final column is headed **`As Enacted after Governor's Vetoes`**. HB 96 and HB 33 head the
same column, in the same position of the same series of workbooks, simply `As Enacted`.

That had been an open question here for several commits: whether LSC's enacted column is taken
before or after the governor acts. It matters because the corpus once inferred "the vetoes
changed no appropriation figure" from those two columns being identical, and the inference only
holds if the columns straddle the veto.

They do — and this workbook proves it twice over, because its own conference-to-enacted delta is
**not** zero. See below.

## Four label forms this connector had never seen

All four were reported by `ColumnPlan::unclassified` rather than dropped, which is the only
reason they were found:

- `House Substitute (LSC 133 0001-4) FY 2020` — a drafting reference inside the header. It
  survives normalisation as `lsc 133 0001 4` and turned four known labels into unknown ones.
- `As Enacted after Governor's Vetoes` — above.
- `Actual FY 2020` and `Adjusted Appropriations FY 2021` — HB 96 and HB 33 head a completed
  year with a bare fiscal year; these say so. `Adjusted Appropriations` became its own
  `ColumnKind`: it is authority partway through execution, and calling it an appropriation
  would put a post-transfer figure in the enacted series while calling it an actual would count
  authority as spending.
- `$ Change` / `% Change` — the publisher's own arithmetic on two other columns. Classified as
  `Other` and skipped deliberately, since reporting them as unclassified would imply a figure
  was lost.

After the fixes all three biennia classify at 18 appropriation columns with nothing
unclassified, and HB 96 and HB 33 read identically to before.
