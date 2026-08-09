---
name: stage-delta
description: Compute how an appropriation moved across bill versions and attribute each change to an action and actor
---

# Skill: stage-delta

**Status:** implemented in [`crates/stage-delta`](../../crates/stage-delta/). Run
`mise run stage-delta`.

Differences a line item across consecutive bill versions and attributes each movement to the
action that caused it.

## Reads

- [`appropriation`](../corpus/appropriation.ont.yml) — the amount at each stage
- [`bill-version`](../corpus/bill-version.ont.yml) — the stage sequence
- [`budget-action`](../corpus/budget-action.ont.yml) — the events between stages

## Returns

Per-stage deltas, each attributed to an action and therefore to an
[`actor`](../corpus/actor.ont.yml).

## Rules

1. **An incomplete chain yields aggregate movement, not attribution.** Where a stage carries
   no figure, the delta across the gap spans everything that happened inside it — chamber
   action, conference resolution, and executive edit at once. Report that as unattributed
   aggregate movement; do not distribute it across the stages it covers. The corpus now
   holds every stage of HB 96 from as-introduced to as-enacted, so no step in that series is
   aggregate; the rule holds for every series that is not yet as complete.
2. **A veto can only reduce or eliminate.** Under the governor's item veto authority nothing can
   be added or increased, so an apparent increase between as-passed and as-enacted is never
   attributable to a veto and must have another explanation. Treat it as a signal that a version
   is missing.
3. **Report the stated justification alongside the delta.** The dollar change and the reason
   given are both part of the answer, and the reason given is not the same as the motive.
4. **For formula programs, the delta may not be the decision.** An amendment can raise an
   appropriation while slowing a phase-in. The arithmetic and the policy point in opposite
   directions, and both are true.

## Running it

`mise run stage-delta` decomposes every (line item, period) the corpus holds two or more
staged amounts for. As of the current corpus: **one decomposable series.**

## The first result

```
foundation-funding FY2026 — net 3261179800 cents across 8 step(s), chain complete

       as-introduced -> house-substitute        +9225000000   house-finance-committee
    house-substitute -> house-reported           +150000000   no action recorded
      house-reported -> as-passed-house                  +0
     as-passed-house -> senate-substitute       -7288820200   senate-finance-committee
   senate-substitute -> senate-reported          +125000000   no action recorded
     senate-reported -> as-passed-senate                 +0
    as-passed-senate -> conference-report       +1050000000   no action recorded
   conference-report -> as-enacted                       +0   governor (line-item veto)
```

Foundation funding rose $92,250,000 in the House substitute, fell $72,888,202 in the Senate
substitute, and came back up $10,500,000 in conference — ending $32,611,798 above where it
was introduced. Three of the eight steps carry an action; the rest moved money with nothing
in the corpus accounting for it, which is a gap in the record rather than a finding about
the legislature.

The last step is the one worth reading twice. The governor's line-item veto attaches to it
and the figure did not move: the veto struck language on this line, not money. Rule 2 exists
because the opposite pattern — an *increase* across a veto — would be impossible, and would
mean a bill version was missing rather than that a veto added anything.

The `post-veto` stage carries no figure anywhere in the corpus, so nothing after enactment is
decomposable yet.

## Aggregate movement, and the leadership test

`aggregate` runs the same decomposition across every line item in an extraction rather than one,
and reports each transition as counts and dollars together: `line_items_moved`, `raised`, `cut`,
`gross_cents`, `net_cents`.

Both because they disagree. A chamber that raises nine hundred small lines and cuts Medicaid has
raised most things and reduced the budget, and only one of those usually gets quoted. Where a
claim about a chamber's *disposition* is being tested, the counts are the less misleading of the
two — a dollar total is one observation wearing a thousand costumes.

`cargo run --bin leadership -- .` is the worked case. It asks whether the House substitute's
direction tracks the Speaker, which
[a decision record](../decisions/leadership-and-the-anomalies.yml) had recorded as an unresolved
coincidence on four observations of a single line item.

Two things it does that the single-line-item version could not:

1. **Partitions by whether the Fair School Funding Plan can reach the line item** (agencies `EDU`
   and `KID`). The plan is a school funding formula, so a plan effect must be confined to that
   side and a leadership effect must not be. The pattern appears equally on both, which
   eliminates the plan.
2. **Follows the one officer who presides more than once.** Matt Huffman is Senate President for
   the 134th and 135th and Speaker for the 136th, and the share of line items his chamber raised
   goes 68-72%, then 35-38%, then 44-46%. A personal disposition does not reverse between two
   adjacent budgets under the same officer.

Both halves come out negative, which is the point: the calculator's job was to make a recorded
coincidence testable, and the test dissolved it.

3. **Runs the same conditional at every hand-off.** For each consecutive pair of actors —
   executive→House, House→Senate, Senate→conference — it asks what this actor did to the line
   items the previous one raised, against those it cut. All three show the same reversal, twelve
   cells of twelve in the same direction, so it is a property of the process rather than of a
   chamber. See [the unit of observation](../decisions/the-unit-of-observation.yml).

The cross-workbook join is on `(agency, ALI, fund group)`, not on the ALI alone. A code appears
more than once in these sheets, and joining on it by itself would pair a line item in one
workbook with a memorandum component of itself in another.

**All figures are within one fiscal year**, so no deflator is involved and no sign depends on the
price level — see [real-dollars](./real-dollars.md) for why that matters, and for what would need
saying if the comparison ever crossed periods.
