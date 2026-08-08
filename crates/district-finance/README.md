# district-finance

**Capability type:** connector. **Status:** stub — deferred, not implemented.

Retrieves school district finance and enrollment reporting.

## Feeds

| Class | What it supplies |
|---|---|
| [`jurisdiction`](../../.yidam/corpus/jurisdiction.ont.yml) | District identifiers, enrollment, valuation |
| [`program`](../../.yidam/corpus/program.ont.yml) | Formula inputs and computed state shares |

## Deferred

This connector serves the local pass-through and incidence framings, which were ranked fourth
in priority. It is the natural partner to the deferred `population` class and the discarded
`jurisdiction →[levies]→ revenue-source` edge — all three should return together, not
piecemeal.

## The vintage problem

Funding formulas frequently compute against a lagged or frozen enrollment count rather than a
current one. This connector must return the count **with its vintage**, because normalizing a
payment computed on a frozen count by current population produces a per-pupil figure that is
wrong in a way that looks entirely reasonable. See the cautions in
[jurisdiction ACTIONS](../../.yidam/corpus/jurisdiction/ACTIONS.md).

## Interface sketch

```
fetch_district(irn: &str) -> Result<District>
fetch_formula_inputs(irn: &str, period: FiscalPeriod) -> Result<FormulaInputs>
```
