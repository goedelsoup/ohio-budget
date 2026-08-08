# Actions — jurisdiction

## Queries

- All expenditures paid to a jurisdiction in a period, across programs
- Which programs serve a jurisdiction, and under what formula
- State money received per resident or per pupil, compared across peer jurisdictions

## Transitions

- **Boundary or identity change.** Districts consolidate, municipalities annex, entities
  dissolve. Handle as identity succession the same way line items are handled, not by editing
  the existing node.
- **Formula reclassification.** A jurisdiction moves between formula tiers or categories,
  changing what it receives without any appropriation changing.

## Calculators

- [`per-capita`](../../skills/) — normalizes receipts by population or enrollment.

## Cautions

- Per-capita comparison requires a population figure with a stated vintage. Formulas often
  use a lagged or frozen count rather than a current one, and using current population to
  normalize a payment computed on a frozen count produces a number that is wrong in a way
  that looks reasonable.
- State money received is not total revenue. A district with a strong local tax base may
  receive little state money and still be well funded — the state share is designed to be
  inverse to local capacity.
