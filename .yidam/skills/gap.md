---
name: gap
description: Compute appropriated authority against actual disbursement for a line item and period
---

# Skill: gap

**Status:** stub. Not runnable — every `amount` in the seed corpus is `[open]`.

The repository's central calculation. Differences authority granted against money actually
spent, which is the question the whole ontology was shaped to make answerable.

## Reads

- [`appropriation`](../corpus/appropriation.ont.yml) — the authority, at a stated stage
- [`expenditure`](../corpus/expenditure.ont.yml) — the disbursement, on a stated basis
- [`line-item`](../corpus/line-item.ont.yml) — the identity both attach to
- [`fiscal-period`](../corpus/fiscal-period.ont.yml) — the frame both are stated against

## Returns

Variance in dollars and percent, plus reversion rate where authority lapsed unspent.

## Prerequisites

- The [`obm`](../../crates/obm/) connector, for the actuals side.
- The [`lsc`](../../crates/lsc/) connector, for the authority side.
- [`real-dollars`](./real-dollars.md) for any result spanning more than one period.

## Rules

1. **Use the enacted stage, or say which stage you used.** Comparing a proposal figure to an
   actual disbursement is a category error.
2. **Never mix bases.** A `disbursed` figure and an `actual-closed` figure for the same line
   item and period will differ. Compare like to like.
3. **Aggregate before differencing.** A district-level disbursement is a slice of a line item,
   not the line item. Differencing one against the full appropriation is wrong by orders of
   magnitude — see
   [the seeded expenditure](../corpus/expenditure/foundation-funding-fy2024-disbursed.yml).
4. **Do not interpret the number uniformly.** A gap on an entitlement-driven line item is
   evidence about forecasting; on a discretionary one it may be evidence about execution. The
   calculator produces the figure; reading it requires knowing the line item's character.
