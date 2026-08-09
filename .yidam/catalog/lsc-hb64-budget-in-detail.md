---
slug: lsc-hb64-budget-in-detail
name: LSC budget in detail, HB 64 as enrolled (131st General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/budget/131/main-operating-budget
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - expenditure
  - line-item
---

# LSC budget in detail — HB 64

The FY2016-17 biennium.

| File | sha256 | Contents |
|---|---|---|
| [`hb64-budget-in-detail-as-enrolled-131st.xlsx`](../sources/lsc/hb64-budget-in-detail-as-enrolled-131st.xlsx) | `6c4c7beb…` | sheet `EN`: enacted appropriations for FY2016 and FY2017, one prior-year actual, one estimate |

- **Retrieved:** 2026-08-09
- **Upstream:** `https://www.lsc.ohio.gov/assets/legislation/131/hb64/en/files/hb64-budget-in-detail-as-enrolled-131st-general-assembly-10075109.xlsx`

## It carries no stages

Thirteen columns: agency, fund group, fund, ALI, ALI title, one prior-year actual, one estimate,
**two appropriation columns**, and four sort keys. There is no substitute column, no
reported-by-committee column and no conference column.

LSC did not publish stage-by-stage appropriation detail before the FY2020-21 budget, and the
filename changes at the same boundary — `budget-in-detail` through the 132nd General Assembly,
`appropriation-spreadsheet` from the 133rd. See
[the leadership record](../decisions/leadership-and-the-anomalies.yml), which asserted the
opposite before it was checked.

The first of these workbooks to carry a bare `Medicaid/Health Care Services` row alongside the
`- State`, `- Federal` and `- Total` memorandum rows. The bare row equals the `- Total` row to the
cent, which is the check that the extraction selected the line item rather than a component.

## The with-actuals sibling

| File | sha256 | Contents |
|---|---|---|
| [`hb64-budget-in-detail-with-actuals-131st.xlsx`](../sources/lsc/hb64-budget-in-detail-with-actuals-131st.xlsx) | `186fb943…` | closed-book actuals for **FY2014 and FY2015**, beside the same two appropriation columns |

Same thirteen columns, with the prior-year estimate replaced by a second completed year. It is
what carries appropriated-against-actual back before FY2020 — the calculation this repository
exists for, which until these were committed could be computed for five fiscal years.

Overlaps are the check. Each workbook reports two completed years and the next reports the later
of them again, so every actual except the first and last is stated twice by independent
documents. Across every line item the corpus extracts, all overlapping figures agree to the
cent. [verified]
