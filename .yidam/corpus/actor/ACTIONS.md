# Actions — actor

## Queries

- All budget actions taken by a role, in a period or across the window
- Net dollar effect of one role's actions on a bill, by line item
- Forecasts issued by a role, and their variance from what materialized
- Which phase of the cycle a role acts in, and therefore which versions it can affect

## Transitions

- **Occupancy change.** The entity playing a role changes — a new governor, a new committee
  chair. The role node persists; the `held_by` property records the succession. This is the
  practical payoff of modeling actor as a role rather than as a person or office.

## Calculators

- [`stage-delta`](../../skills/) — attributes each change in an appropriation to the action
  and therefore the role that caused it.

## Cautions

- Attribution is not motive. A `stated_justification` on an action is what the actor said,
  which is evidence about the actor and not necessarily about the reason. Record the stated
  reason; tag any inferred one.
- Roles acting after enactment have real authority. Treating the enacted bill as final
  understates how much money moves through the controlling board and executive reductions.
