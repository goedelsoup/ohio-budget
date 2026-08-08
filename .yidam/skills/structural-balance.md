---
name: structural-balance
description: Compute recurring revenue against recurring obligation for a biennium, isolating one-time money
---

# Skill: structural-balance

**Status:** stub. Not runnable — requires populated amounts, forecasts, and fund transfers.

Separates recurring capacity from one-time money to determine whether a budget balances
structurally or only arithmetically.

## Reads

- [`revenue-source`](../corpus/revenue-source.ont.yml) — recurring receipts by instrument
- [`appropriation`](../corpus/appropriation.ont.yml) — obligations undertaken
- [`forecast`](../corpus/forecast.ont.yml) — what receipts were projected to be
- [`fiscal-period`](../corpus/fiscal-period.ont.yml) — the biennium frame
- [`fund`](../corpus/fund.ont.yml) — reserve balances and transfers

## Returns

Structural surplus or deficit per biennium, with one-time money isolated and identified.

## Why fund transfers are load-bearing here

Ohio's constitution requires a balanced budget, so every enacted budget balances by
construction. The interesting question is *how*. Using reserve draws or fund transfers to cover
recurring obligations is the textbook signature of a structural deficit, and it is visible only
because [`fund →[transfers-to]→ fund`](../corpus/fund.ont.yml) was approved as an edge — money
moving between funds involves no appropriation and would otherwise be invisible to this
calculation entirely.

## Rules

1. **Requires [`real-dollars`](./real-dollars.md).** A nominal structural series is meaningless.
2. **Classify each revenue item as recurring or one-time explicitly, and record the
   classification.** This is the judgment the whole result rests on, and it is contestable.
3. **A phase-in creates an out-year obligation not visible in the current biennium.** Count it.
4. **Report the result as a range under stated assumptions, not a point estimate.** The inputs
   are forecasts.
