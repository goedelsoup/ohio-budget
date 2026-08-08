# Actions — budget-action

## Queries

- All actions applied to a bill version, in date order
- Net dollar effect of one actor's actions on a bill
- All execution-phase actions in a period — the money that moved after enactment
- Actions following a forecast revision, which is where the `triggered-by` edge earns its keep

## Transitions

Actions are events and do not transition. They are recorded once and not revised. A subsequent
action that reverses an earlier one is a second node, not an edit to the first.

## Calculators

- [`stage-delta`](../../skills/) — attributes movement in an appropriation to the actions that
  caused it.

## Cautions

- Stated justification is evidence about the record, not about motive. Do not promote a stated
  reason to an explanation without tagging the inference.
- Execution-phase actions are systematically under-reported in summary coverage of a budget.
  Absence of a controlling board action in this corpus means it has not been extracted, not
  that none occurred.
- A line-item veto reduces or eliminates; it cannot add. An apparent increase between as-passed
  and as-enacted is therefore never attributable to a veto.
