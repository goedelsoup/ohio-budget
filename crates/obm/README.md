# obm

**Capability type:** connector. **Status:** stub — not implemented.

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
