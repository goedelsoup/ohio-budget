# Actions — expenditure

## Queries

- Total disbursed against a line item in a period, against what was appropriated
- All expenditures paid to a jurisdiction, across programs and periods
- Authority that reverted unspent at period end, by line item
- Expenditures whose basis is still `disbursed` rather than `actual-closed`

## Transitions

- **Close.** A period's books are finalized. Basis moves from `disbursed` to `actual-closed`,
  and any previously computed gap should be recomputed rather than trusted.
- **Reversion.** Authority lapses unspent at period end. Recorded on the expenditure node and,
  once the approved `reverts-to` edge is populated, as an edge back to the fund.

## Calculators

- [`gap`](../../skills/) — the central computation: appropriated against actual, with reversion
  rate.
- [`per-capita`](../../skills/) — normalizes disbursement by recipient population.
- [`real-dollars`](../../skills/) — required before comparing across periods.

## Cautions

- A gap is not by itself evidence of underspending. On an entitlement-driven line item it is
  evidence about forecasting; on a discretionary one it may be evidence about execution. Read
  the result against the line item's character.
- Disbursed and actual-closed figures for the same line item and period will differ. Never mix
  bases within a single comparison.
- Money leaving the treasury is not money reaching a beneficiary. A pass-through disbursement to
  a jurisdiction is the state's last observation point, not the end of the chain.
