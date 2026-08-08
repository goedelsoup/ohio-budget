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
