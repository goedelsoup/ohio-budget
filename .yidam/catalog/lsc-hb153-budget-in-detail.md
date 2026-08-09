---
slug: lsc-hb153-budget-in-detail
name: LSC budget in detail, HB 153 as enrolled (129th General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/budget/129/main-operating-budget
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - line-item
---

# LSC budget in detail — HB 153

The FY2012-13 biennium.

| File | sha256 | Contents |
|---|---|---|
| [`hb153-budget-in-detail-as-enrolled-129th.xls`](../sources/lsc/hb153-budget-in-detail-as-enrolled-129th.xls) | `0916adca…` | sheet `All Funds without Summary`: enacted appropriations for FY2012 and FY2013, one prior-year actual, one estimate |

- **Retrieved:** 2026-08-09
- **Upstream:** `https://www.lsc.ohio.gov/assets/legislation/129/hb153/en/files/hb153-budget-in-detail-as-enrolled-129th-general-assembly-10075143.xlsx`

## It carries no stages

Thirteen columns: agency, fund group, fund, ALI, ALI title, one prior-year actual, one estimate,
**two appropriation columns**, and four sort keys. There is no substitute column, no
reported-by-committee column and no conference column.

LSC did not publish stage-by-stage appropriation detail before the FY2020-21 budget, and the
filename changes at the same boundary — `budget-in-detail` through the 132nd General Assembly,
`appropriation-spreadsheet` from the 133rd. See
[the leadership record](../decisions/leadership-and-the-anomalies.yml), which asserted the
opposite before it was checked.

Served at a `.xlsx` URL and is a legacy OLE2 `.xls`. Committed under its true extension. The
connector opens workbooks by sniffing their content rather than trusting the name, because the
zip reader's failure on this file — `Could not find EOCD` — says nothing about the real problem.

Its three sheets include two carrying summary rows; `All Funds without Summary` is the one read,
since summing a summary row with the detail beneath it double-counts.

## There is no with-actuals sibling

LSC's page for the 129th General Assembly links a `-with-actual-expenditures-` file, and it is
**byte-identical** to the as-enrolled workbook — the same sha256, the same three sheets. One
document served at two URLs. [verified]

That matters for what the corpus can reach. This workbook's only completed year is FY2010; FY2011
appears solely as an `Estimate` column, and no later workbook reaches back that far. **FY2011 has
no actual anywhere in this source family**, and an estimate is somebody's projection rather than
an outturn, so the year is left absent rather than filled with the nearest available number.
