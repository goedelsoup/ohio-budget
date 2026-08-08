# Actions — fiscal-period

## Queries

- All appropriations covering a period, across every bill that made them
- All expenditures occurring in a period
- The bill or bills enacted for a period, including any supplemental that amended them
- Which forecasts were issued for a period, and how each compared to what materialized

## Transitions

- **Open → closed.** A period closes when the books are finalized. Expenditure nodes shift
  basis from `disbursed` to `actual-closed`, and any `gap` result computed before closing
  should be recomputed rather than trusted.

## Calculators

- [`real-dollars`](../../skills/) — deflates amounts across periods. Required before any
  comparison spanning more than one period.
- [`structural-balance`](../../skills/) — computes recurring revenue against recurring
  obligation for a biennium.

## Cautions

- A biennium figure is not two annual figures added together in every case. Some line items
  carry biennial authority that may be spent in either year.
- Do not compare a period's appropriation to another period's expenditure without stating
  that is what you are doing.
