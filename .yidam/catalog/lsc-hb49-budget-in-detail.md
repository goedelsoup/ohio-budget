---
slug: lsc-hb49-budget-in-detail
name: LSC budget in detail, HB 49 as enrolled (132nd General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/budget/132/main-operating-budget
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - appropriation
  - line-item
---

# LSC budget in detail — HB 49

The FY2018-19 biennium.

| File | sha256 | Contents |
|---|---|---|
| [`hb49-budget-in-detail-as-enrolled-132nd.xlsx`](../sources/lsc/hb49-budget-in-detail-as-enrolled-132nd.xlsx) | `964f0734…` | sheet `EN`: enacted appropriations for FY2018 and FY2019, one prior-year actual, one estimate |

- **Retrieved:** 2026-08-09
- **Upstream:** `https://www.lsc.ohio.gov/assets/legislation/132/hb49/en/files/hb49-budget-in-detail-as-enrolled-132nd-general-assembly-10075086.xlsx`

## It carries no stages

Thirteen columns: agency, fund group, fund, ALI, ALI title, one prior-year actual, one estimate,
**two appropriation columns**, and four sort keys. There is no substitute column, no
reported-by-committee column and no conference column.

LSC did not publish stage-by-stage appropriation detail before the FY2020-21 budget, and the
filename changes at the same boundary — `budget-in-detail` through the 132nd General Assembly,
`appropriation-spreadsheet` from the 133rd. See
[the leadership record](../decisions/leadership-and-the-anomalies.yml), which asserted the
opposite before it was checked.

Committed with its adjusted-appropriations sibling, which is what established the stage of the
`Appropriation` column. See below.

## Establishing what stage `Appropriation` means

Every later workbook names its final column — `As Enacted`, or `As Enacted after Governor's
Vetoes`. These four say only `Appropriation`, so whether the figure is the legislature's enrolled
amount or the operative post-veto authority had to be established rather than assumed. Ohio's
governor may strike an item but not raise one, and a struck appropriation is rare but not unknown
— a veto zeroed ALI 651691 in HB 166.

| File | sha256 | Contents |
|---|---|---|
| [`hb49-budget-in-detail-adjusted-appropriations-132nd.xlsx`](../sources/lsc/hb49-budget-in-detail-adjusted-appropriations-132nd.xlsx) | `57278412…` | `FY 2019 Adjusted Approp. OAKS as of 9/11/2018` — operative authority inside the fiscal year |

OAKS is the state accounting system, and a figure recorded there in September 2018 already
reflects any veto, since a veto precedes the bill taking effect. Comparing the enrolled
`Appropriation FY 2019` column against it across all 1,400 line items: [verified]

    987  identical
    401  OAKS higher than enrolled
     11  OAKS lower than enrolled
     14  ALI absent from the OAKS sheet

**The direction settles it.** A veto cannot increase an appropriation, so 401 upward moves are
in-year adjustment — controlling board transfers and reallocation — and cannot be veto. Of the
eleven downward, none is a zeroing: they are partial reductions between 1% and 30%, the shape of
a transfer out rather than of a struck item. A vetoed appropriation would appear as enrolled
above zero and OAKS at zero, as ALI 651691 does in HB 166. None does.

So the `Appropriation` columns are read as `as-enacted`.

[open] This establishes the pattern is inconsistent with the column being systematically
pre-veto. It does not prove no veto touched any amount in these four biennia. The stronger
evidence would be the veto messages themselves, which are catalogued for HB 96 and HB 33 only.
