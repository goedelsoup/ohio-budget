---
slug: fred-price-indices
name: Price index series (BEA and BLS, retrieved via FRED)
source_type: dataset
location: https://fred.stlouisfed.org/
publisher: Federal Reserve Bank of St. Louis, redistributing BEA and BLS series
content_committed: true
feeds:
  - fiscal-period
---

# Price index series

Two candidate deflators and one sensitivity probe, all committed so the choice between them can
be re-argued against the data rather than from memory.

| File | Series | Cadence | sha256 |
|---|---|---|---|
| [`fred-a829rd3q086sbea.csv`](../sources/price-index/fred-a829rd3q086sbea.csv) | State and local government consumption expenditures and gross investment, implicit price deflator (BEA) | quarterly | `b7adcaa1…` |
| [`fred-cpiaucns.csv`](../sources/price-index/fred-cpiaucns.csv) | CPI-U, all items, not seasonally adjusted (BLS) | monthly | `4eb7e334…` |
| [`fred-dhlcrg3q086sbea.csv`](../sources/price-index/fred-dhlcrg3q086sbea.csv) | Personal consumption expenditures, health care, chain-type price index (BEA) | quarterly | `2fa646c4…` |

The third is not a candidate deflator. It is the direct sibling of the chosen one — same agency,
same cadence, same construction — committed so that every restated series can be reported under a
**sectorally** different index as well as a generally different one. That distinction turned out
to matter: CPI-U against state and local purchases differs by 0.4 percentage points, health care
prices against state and local purchases by 8 to 14. See
[the deflator decision](../decisions/deflator-choice.yml).

- **Retrieved:** 2026-08-09, covering 2008 to present.

Not seasonally adjusted, deliberately: a fiscal-year average over all twelve months removes
seasonality by construction, and adjusting first would apply the correction twice.

## The index is derived, not transcribed

Neither file states a fiscal-year figure. Ohio's fiscal year runs 1 July to 30 June, so
`real_dollars::Observations::ohio_fiscal_years` averages the observations falling inside it —
twelve for a monthly series, four for a quarterly one, with the cadence inferred from the
spacing rather than declared.

A fiscal year missing any observation is reported as uncovered rather than averaged from what
is present. The mean of eleven months is not the price level of a twelve-month year, and a
figure deflated by one would be wrong by an amount no downstream check could see.

## CPI-U cannot cover FY2026

BLS never published an October 2025 CPI-U. FRED carries the row with an empty value, and that
one gap makes FY2026 permanently unaverageable from this series — the month cannot be collected
retrospectively.

It is committed anyway. It is the obvious alternative index, the corpus should be able to show
what it would have said, and a source that rules itself out for a stated reason is more useful
than one nobody kept.
