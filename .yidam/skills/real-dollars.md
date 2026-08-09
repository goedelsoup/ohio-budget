---
name: real-dollars
description: Restate nominal appropriation and expenditure amounts in constant dollars across the corpus window
---

# Skill: real-dollars

**Status:** implemented in [`crates/real-dollars`](../../crates/real-dollars/).

Deflates nominal amounts to a constant-dollar basis.

## Reads

- [`appropriation`](../corpus/appropriation.ont.yml), [`expenditure`](../corpus/expenditure.ont.yml) — nominal amounts
- [`fiscal-period`](../corpus/fiscal-period.ont.yml) — the period each is stated against

## Returns

The same series in constant dollars, with the base year and deflator stated.

## Not an enhancement — a prerequisite

This is listed as a peer of the other calculators and is really a precondition for them. The
corpus spans FY2010 to the present. A sixteen-year comparison in nominal dollars is not a
comparison, and treating this as optional would silently corrupt every long-series result from
[`gap`](./gap.md) and [`structural-balance`](./structural-balance.md).

That standing is recorded as a condition of approval in
[the proposals decision](../decisions/proposals.yml).

## Rules

1. **State the base year and the deflator, always.** A constant-dollar figure without both is
   not interpretable.
2. **Choice of deflator is a modeling decision, not a technical detail.** A general price index
   and a state-and-local-government-purchases index give materially different answers for a
   budget series, and the difference is not noise. Record the choice as a decision.
3. **Do not deflate a per-unit figure and a total with different vintages** of the same
   deflator.
4. **Nominal is correct for some questions.** Debt service and statutory dollar thresholds are
   nominal quantities. Do not deflate reflexively.

## No deflator ships with it

Deliberately. Which index to use is a modeling decision — a general price index and a
state-and-local-government-purchases index give materially different answers for a budget
series — so `Deflator` must be supplied and must name its base period and series. A result
carries both, and `Real::label()` exists so a constant-dollar figure is never quoted as a bare
number.

Three refusals, each with a test: a period outside the index errors rather than extrapolating;
a series containing any uncovered period fails whole rather than partly deflating, because a
chart mixing nominal and real points looks fine; and `is_nominal_by_nature` flags debt service
and statutory thresholds, which must not be deflated at all.

[`gap::gap_trend`](../../crates/gap/) refuses any cross-period comparison without one.

## The one this repository supplies

`corpus-export` builds an index from a committed source under
[a recorded decision](../decisions/deflator-choice.yml), and applies it: `findings.real_terms`
carries one restated enacted series per line item and `findings.gap_trend` one comparison per
adjacent period pair, each naming the index on every record.

**Supplying an index is not the same as applying it,** and this repository learned the
difference the expensive way. `gap_trend` took a `deflator` argument from the day it was
written and no caller ever passed one; the export built a `Deflator`, copied its name into the
manifest, and discarded the index. `deflator_available: true` then shipped beside figures that
had never been restated, and the web layer's own guard — which had been correctly refusing to
draw a nominal series — read the flag, passed, and dropped its caveat. See
[nominal growth is mostly price level](../decisions/nominal-growth-is-mostly-price-level.yml).

So: a boolean saying an index exists is not a claim that anything was deflated. Where a figure
is restated, the restatement itself travels — both dollars on the same record, with the series
name — and any consumer reading only `deflator_available` is reading the wrong field.

## What it cannot do

Deflation multiplies all of a period's figures by one positive number. It therefore cannot
change a sign, a within-period ratio, or a fund share. A stage delta, a gap variance, and a
line item's share of its programme are all unaffected in direction. Only comparisons *between*
periods change — which is most of what a sixteen-year corpus is for, and none of what a
single-year result rests on.
