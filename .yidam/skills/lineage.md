---
name: lineage
description: Propose whether a line item continues an earlier differently-coded one, for human confirmation
---

# Skill: lineage

**Status:** stub. Not runnable — the corpus holds four line items with mostly `[open]` codes.

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
