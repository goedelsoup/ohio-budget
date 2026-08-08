---
name: lineage
description: Propose whether a line item continues an earlier differently-coded one, for human confirmation
---

# Skill: lineage

**Status:** implemented in [`crates/lineage`](../../crates/lineage/). Run `mise run lineage`.

Proposes succession candidates across renumberings, restructurings, and agency reorganizations.

## Reads

- [`line-item`](../corpus/line-item.ont.yml) — codes, titles, purposes, periods, status
- [`appropriation`](../corpus/appropriation.ont.yml) — amount continuity across the boundary
- [`agency`](../corpus/agency.ont.yml) — reorganizations that explain a code change

## Returns

Candidate successor pairs, each with a confidence score and the evidence supporting it —
title similarity, purpose overlap, amount continuity, and whether an agency reorganization
accounts for the change.

## This calculator returns hypotheses, not values

**Its output must never be written to `succeeds` or `superseded-by` edges without a
contributor confirming the pair.** This is a standing condition of its approval, recorded in
[the proposals decision](../decisions/proposals.yml), not a matter of taste.

The failure mode is specific and quiet. A wrong lineage assertion fuses two unrelated funding
histories into one series. Nothing downstream detects it, every chart drawn from it looks
reasonable, and the error is indistinguishable from a real funding change. A missing lineage
edge leaves an obvious gap; a wrong one manufactures false continuity.

## Rules

1. Propose. Never write edges.
2. Rank by evidence, and state the evidence rather than only the score.
3. Prefer a null result to a low-confidence guess. An unresolved break is a correct answer.
4. An agency reorganization is strong evidence for a renumbering and weak evidence for
   substantive continuity — the money may have been restructured at the same moment.

## What it found, and why the rule exists

Run against the current corpus it returns two candidates:

```
[0.60] medicaid-services-odjfs -> medicaid-health-care-services          (AlreadyAsserted)
[0.58] medicaid-services-odjfs -> medicaid-health-care-services-federal  (Proposed)
```

The first is correct and already asserted. **The second is wrong**, and it scores within 0.02
of the right answer. The federal-share line item is a *sibling* of the state-share line, not a
successor of the pre-2013 combined line — they split one program across two funds, they do not
continue one another.

Every signal the calculator can see points the wrong way here: the predecessor is superseded,
the titles overlap, and the holding agency genuinely succeeds the predecessor's agency. There
is no additional evidence that would separate them, because the distinction is about what the
money *is*, not about how the records look.

This is the standing condition from [proposals](../decisions/proposals.yml) earning its keep on
the first real run. A tool that applied its own output would have fused two unrelated funding
histories into one series, and no downstream check would have caught it.
