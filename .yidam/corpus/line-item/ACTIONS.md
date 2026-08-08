# Actions — line-item

## Queries

- Appropriations for a line item across every version of every bill, in stage order
- Appropriated against actual disbursed, for a line item and period
- The full lineage chain for a line item across renumberings
- All programs drawing on a line item, and all jurisdictions receiving from it

## Transitions

- **Renumbering.** The code changes while the thing funded persists. Record the prior code in
  `historical_codes`; do not create a new node.
- **Restructuring, split, or consolidation.** The thing funded genuinely changes shape. Create
  new nodes and assert `succeeds` / `superseded-by` edges between old and new.
- **Retirement.** The line item stops receiving appropriations. Set `status` to `retired`;
  never delete, since the historical series depends on it.

## Calculators

- [`lineage`](../../skills/) — proposes succession candidates across renumberings. **Output
  requires human confirmation before being written as an edge.**
- [`gap`](../../skills/) — appropriated against actual, with reversion rate.
- [`stage-delta`](../../skills/) — how the amount moved across bill versions.
- [`real-dollars`](../../skills/) — required before any multi-period comparison.

## Cautions

- A line item series that shows a sharp break at a biennium boundary is far more likely to be
  a renumbering than a funding decision. Check lineage before drawing a conclusion.
- Line item totals do not equal program costs. A program may draw on several line items, and a
  line item may fund several programs.
- Do not compare line item appropriations across funds without saying so — a general revenue
  figure and an all-funds figure for the same line item are different numbers.
