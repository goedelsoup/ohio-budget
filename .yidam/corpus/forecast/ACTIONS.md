# Actions — forecast

## Queries

- All forecasts issued for a period, and by which role
- Variance from actual, by instrument and period, across the window
- Which budget actions followed a forecast revision
- Whether forecast error is systematically directional for a given issuer

## Transitions

- **Revision.** An updated projection is issued. Creates a new node rather than editing the
  existing one — the superseded projection is the record of what the budget was sized against
  at the time.
- **Close.** The period ends and actuals are known. Populates `variance_from_actual`, which is
  what turns the node from a document into evidence.

## Calculators

- [`structural-balance`](../../skills/) — reads forecasts against appropriations to separate
  recurring capacity from one-time money.

## Cautions

- Do not compare a forecast to a later forecast and call the difference an error. Error is
  distance from actual, not distance from a revision.
- Forecast and reporting frequently come from the same office, so forecast error and reporting
  basis are not independent sources of uncertainty. [inference]
- A projection made before a tax law change is not comparable to one made after it. Record the
  assumptions, or the variance is uninterpretable.
