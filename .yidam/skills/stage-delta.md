---
name: stage-delta
description: Compute how an appropriation moved across bill versions and attribute each change to an action and actor
---

# Skill: stage-delta

**Status:** stub. Not runnable — appropriation amounts are `[open]`.

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

1. **An incomplete chain yields aggregate movement, not attribution.** The seed corpus holds
   HB 96 as introduced, as passed by the House, and as enacted — so the second hop spans Senate
   action, conference resolution, and executive veto at once. Report that as unattributed
   aggregate movement; do not distribute it across the three.
2. **A veto can only reduce or eliminate.** Under the governor's item veto authority nothing can
   be added or increased, so an apparent increase between as-passed and as-enacted is never
   attributable to a veto and must have another explanation. Treat it as a signal that a version
   is missing.
3. **Report the stated justification alongside the delta.** The dollar change and the reason
   given are both part of the answer, and the reason given is not the same as the motive.
4. **For formula programs, the delta may not be the decision.** An amendment can raise an
   appropriation while slowing a phase-in. The arithmetic and the policy point in opposite
   directions, and both are true.
