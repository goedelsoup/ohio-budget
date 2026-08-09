---
name: gap
description: Compute appropriated authority against actual disbursement for a line item and period
---

# Skill: gap

**Status:** implemented in [`crates/gap`](../../crates/gap/). Run `mise run gap`.

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

## Running it

`mise run gap` reports every (line item, period) pair the corpus could compute a gap for, and
why each is blocked. As of the current corpus: **1 computable, 7 blocked on `[open]` amounts,
5 refused.**

## The first result

```
COMPUTED foundation-funding FY2024: variance -775359689 cents (-0.1%)
  distribution follows a formula; a divergence is evidence about the
  formula's inputs rather than about execution
```

Enacted authority was $7,967,250,000; actual spending was $7,975,003,596.89. Spending exceeded
enacted authority by **$7,753,596.89**, about a tenth of one percent.

Two things about that are worth more than the number. First, the sign: a gap is usually
discussed as authority left unspent, and this one runs the other way, which means enacted
authority was not the operative limit at year end and something in the execution phase moved
it. Second, the reading attached to it is not decoration — foundation funding is
formula-driven, so a divergence here is evidence that the formula's inputs came in above the
estimate the appropriation was sized on, **not** evidence about anyone's spending discipline.
The identical number on a discretionary line would mean something else entirely.

All five refusals are the recipient-slice guard firing on real data — the corpus holds
district- and county-level disbursements against whole-line appropriations, and differencing
those is wrong by orders of magnitude while looking entirely reasonable.

That report is the repository's own inventory of what extraction has to deliver before its
central question can be answered at all.

## Why the blocked pairs cluster at the start of the window

Twenty-seven of 197 pairs are not computed, and most of them fail the same way: the corpus holds
an expenditure and no appropriation to measure it against. That is not a backlog. It follows from
the shape of the source.

An LSC workbook appropriates one biennium and reports **two completed years** beside it. So each
document reaches two years further back in actuals than in appropriations, and the earliest one
committed — HB 153, for the 129th General Assembly — supplies an FY2010 actual with no FY2010
appropriation anywhere. The 128th published no workbook at all, so nothing can supply one.
**Twelve of the blocked pairs are FY2010 and are permanently blocked.** [verified]

The same asymmetry accounts for most of the rest, one line item at a time: `foundation-funding-dpf`
has actuals from FY2018 and appropriations from FY2020; `medicaid-health-care-services` has
FY2012-13 actuals of $0 against a code that did not carry money until FY2014, its predecessor
[ALI 600525](../corpus/line-item/medicaid-services-odjfs.yml) having held it.

Three more are recipient slices — a Cleveland or Cuyahoga disbursement against a whole line item —
which `gap` refuses by design rather than for want of data.

**None of these is a defect and none is a todo.** What would be a defect is a reader taking a
list of twenty-seven blocked pairs as twenty-seven pieces of missing work, so the distinction is
recorded here: a pair blocked because the corpus has not reached the figure is different from one
blocked because the figure was never published, and the second kind does not shrink.
