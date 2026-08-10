# obm

**Capability type:** connector. **Status:** stub — **reachable, and coarser than the corpus needs.**

Retrieves Office of Budget and Management monthly financial reports and the state's audited
annual financial report.

## Feeds

| Class | What it supplies |
|---|---|
| [`expenditure`](../../.yidam/corpus/expenditure.ont.yml) | Disbursed and actual-closed figures by line item and period |
| [`fund`](../../.yidam/corpus/fund.ont.yml) | Balances and transfers |
| [`forecast`](../../.yidam/corpus/forecast.ont.yml) | Revenue estimates and their revisions |

## Why it is ordered third

It is the only authoritative source for actual disbursement and therefore a hard prerequisite
for [`gap`](../../.yidam/skills/gap.md), the repository's central calculation. It is ordered
after [`lsc`](../lsc/) only because a gap needs both sides — actuals with nothing to compare
against answer no question.

## Basis handling

The connector must preserve the distinction between in-year `disbursed` figures and
`actual-closed` figures rather than normalizing them. They routinely disagree, and a gap
computed across mixed bases is not a gap.

## Interface sketch

```
fetch_monthly(period: FiscalPeriod, month: u8) -> Result<Vec<ExpenditureRow>>
fetch_annual(period: FiscalPeriod) -> Result<AnnualReport>
fetch_revenue_estimate(period: FiscalPeriod) -> Result<Vec<Forecast>>
```

## Open questions

- [open] Whether monthly reporting is available at line item granularity or only at fund and
  agency level. If the latter, the gap calculator cannot operate at the granularity the corpus
  models, and that is a material constraint on the whole design.

## Scouted 2026-08-09

Unlike the [controlling board](../controlling-board/README.md), which cannot be enumerated, and
the [checkbook](../checkbook/README.md), which guards against automation, this source is fully
reachable:

    archives.obm.ohio.gov/Files/Budget_and_Planning/Monthly_Financial_Report/YYYY-MM-mfr.pdf

Reports are enumerable by date, roughly 1.5 MB each, and the text extracts cleanly. Separators
are inconsistent — `2022-12_mfr.pdf` and `2022_11-mfr.pdf` both occur — so a fetcher needs to try
both.

**What it cannot do is the thing the corpus wanted.** Disbursements are reported by spending
category and by fund, never by appropriation line item: the January 2026 report contains zero
occurrences of any ALI this corpus models. [verified] The catalog entry called that question
["the single largest unresolved risk to the repository's central
question"](../../.yidam/catalog/obm-annual-report.md) and it is answered against.

So a connector here would feed `fund` and `forecast` usefully and would not feed `expenditure`
at the granularity `gap` operates on. That is worth writing and is not what it was queued for.

[open] The audited **annual** report is a different document and was not scouted. If it carries
line-item detail where the monthly reports do not, it would close FY2025 two years before LSC
does — which is the one thing that would change what this corpus can answer about an open year.
