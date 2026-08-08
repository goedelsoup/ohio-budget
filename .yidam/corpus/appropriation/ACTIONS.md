# Actions — appropriation

## Queries

- The same line item and period across every bill version, in stage order
- All appropriations drawing on a fund in a period
- Appropriations modified by a given actor's actions
- Appropriated against actual disbursed, for a line item and period

## Transitions

- **Stage restatement.** A later bill version states a different amount. Creates a new
  appropriation node rather than editing the existing one — the prior figure is the record of
  what that stage decided.
- **Modification.** A [`budget-action`](../budget-action/) changes, creates, or eliminates the
  amount. Recorded as an inbound `modifies` edge, preserving both the action and the actor.

## Calculators

- [`gap`](../../skills/) — appropriated against actual, with reversion rate.
- [`stage-delta`](../../skills/) — movement across versions, attributed to actions.
- [`real-dollars`](../../skills/) — required before any comparison across periods.

## Cautions

- Appropriation is authority, not cash. An appropriation can exceed available fund balance and
  frequently does.
- Never quote an appropriation without its stage. "The budget appropriated X for this" is
  ambiguous between the proposal, each chamber's version, and the enacted figure — and those
  differ by design.
- Biennial and annual figures are both legitimate and are not interchangeable. State which.
