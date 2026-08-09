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
