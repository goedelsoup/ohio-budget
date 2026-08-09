---
name: per-capita
description: Normalize expenditure to a jurisdiction by population or enrollment for cross-jurisdiction comparison
---

# Skill: per-capita

**Status:** stub. Not runnable — ten jurisdictions are seeded and every `population_basis`
among them is `[open]`, so no denominator exists to normalize by.

Normalizes state money received so that jurisdictions of different sizes can be compared.

## Reads

- [`expenditure`](../corpus/expenditure.ont.yml) — money paid to a jurisdiction
- [`jurisdiction`](../corpus/jurisdiction.ont.yml) — population or enrollment, with vintage

## Returns

Per-resident or per-pupil figures, comparable across jurisdictions, with the population vintage
stated.

## The vintage trap

Funding formulas frequently compute against a **lagged or frozen** count rather than a current
one. Normalizing a payment computed on a frozen count by current population produces a number
that is wrong in a way that looks entirely reasonable — it is the wrong denominator, silently.

The calculator must therefore take the vintage as an explicit input and refuse to run when it
is unknown, rather than defaulting to the most recent figure available. This is why
[`district-finance`](../../crates/district-finance/) is specified to return counts with their
vintage attached.

## Rules

1. **Refuse rather than default** when the population vintage is unstated.
2. **State money received is not total revenue.** A district with a strong local tax base may
   receive little state money and be well funded — state share is designed to be inverse to
   local capacity, so a low per-pupil state figure may mean the formula is working rather than
   failing.
3. **Requires [`real-dollars`](./real-dollars.md)** for any comparison across periods.
4. Report the denominator alongside the result, always.
