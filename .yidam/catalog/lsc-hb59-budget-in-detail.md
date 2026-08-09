---
slug: lsc-hb59-budget-in-detail
name: LSC budget in detail, HB 59 as enrolled (130th General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/budget/130/main-operating-budget
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - expenditure
  - line-item
---

# LSC budget in detail — HB 59

The FY2014-15 biennium.

| File | sha256 | Contents |
|---|---|---|
| [`hb59-budget-in-detail-as-enrolled-130th.xlsx`](../sources/lsc/hb59-budget-in-detail-as-enrolled-130th.xlsx) | `e68ff534…` | sheet `EN`: enacted appropriations for FY2014 and FY2015, one prior-year actual, one estimate |

- **Retrieved:** 2026-08-09
- **Upstream:** `https://www.lsc.ohio.gov/assets/legislation/130/hb59/en/files/hb59-budget-in-detail-as-enrolled-130th-general-assembly-10075125.xlsx`

## It carries no stages

Thirteen columns: agency, fund group, fund, ALI, ALI title, one prior-year actual, one estimate,
**two appropriation columns**, and four sort keys. There is no substitute column, no
reported-by-committee column and no conference column.

LSC did not publish stage-by-stage appropriation detail before the FY2020-21 budget, and the
filename changes at the same boundary — `budget-in-detail` through the 132nd General Assembly,
`appropriation-spreadsheet` from the 133rd. See
[the leadership record](../decisions/leadership-and-the-anomalies.yml), which asserted the
opposite before it was checked.

Carries the Medicaid agency transition in a single document: ALI 600525 under `JFS` is present
and zeroed in both years, while ALI 651525 under `MCD` holds the money. The predecessor line item
was retained at zero rather than removed, which is what makes the succession checkable.

## The with-actuals sibling

| File | sha256 | Contents |
|---|---|---|
| [`hb59-budget-in-detail-with-actuals-130th.xlsx`](../sources/lsc/hb59-budget-in-detail-with-actuals-130th.xlsx) | `0f468e0d…` | closed-book actuals for **FY2012 and FY2013**, beside the same two appropriation columns |

Same thirteen columns, with the prior-year estimate replaced by a second completed year. It is
what carries appropriated-against-actual back before FY2020 — the calculation this repository
exists for, which until these were committed could be computed for five fiscal years.

Overlaps are the check. Each workbook reports two completed years and the next reports the later
of them again, so every actual except the first and last is stated twice by independent
documents. Across every line item the corpus extracts, all overlapping figures agree to the
cent. [verified]
