# Actions — revenue-source

## Queries

- Which funds an instrument deposits into, and in what proportions
- Which programs draw directly on an instrument rather than through general appropriation
- Forecast against actual receipts for an instrument in a period
- Rate history for an instrument across the corpus window

## Transitions

- **Rate change.** A statutory change to rate, base, or brackets. Does not create a new
  instrument node — it revises the existing one and records the prior state, since the
  instrument's identity persists through rate changes.
- **Disposition change.** A change to how proceeds split across funds. Affects
  `deposits-into` edges without touching the instrument's own properties.

## Calculators

- [`structural-balance`](../../skills/) — separates recurring receipts from one-time money.
- [`real-dollars`](../../skills/) — required before comparing receipts across periods.

## Cautions

- Constitutional restrictions bind harder than statutory ones. Motor fuel tax proceeds cannot
  be redirected by an appropriation act, which means a shortfall in a restricted fund cannot
  be covered from general revenue the way an unrestricted one can.
- Rate is not revenue. A rate cut and a receipts decline are different claims with different
  evidence, and conflating them is the most common error in this domain.
