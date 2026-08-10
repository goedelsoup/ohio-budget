# orc

**Capability type:** connector. **Status:** stub — not implemented.

Retrieves Ohio Revised Code sections governing funds, revenue instruments, and distribution
formulas.

## Feeds

| Class | What it supplies |
|---|---|
| [`fund`](../../.yidam/corpus/fund.ont.yml) | Statutory basis, purpose, and use restrictions |
| [`revenue-source`](../../.yidam/corpus/revenue-source.ont.yml) | Authorizing chapter, rate basis, disposition percentages |
| [`program`](../../.yidam/corpus/program.ont.yml) | Authorizing statute and formula basis |

## Character

Slow-changing. This is a one-time seed plus occasional refresh rather than a live connector,
and it is the cheapest way to convert a large number of `[inference]` tags in the seed corpus
into `[verified]` ones — every statutory citation currently asserted from memory is a candidate.

It also supplies the historical percentages that make the
[local government fund](../../.yidam/corpus/fund/local-government-fund.yml) story checkable
rather than remembered.

## Interface sketch

```
fetch_section(citation: &str) -> Result<StatuteSection>
fetch_chapter(chapter: &str) -> Result<Vec<StatuteSection>>
```

## Open questions

- ~~[open] Whether superseded versions are retrievable.~~ **Answered — they are.** See below.

## Scouted 2026-08-09 — superseded versions are retrievable and addressable

The question this connector hung on is settled. `codes.ohio.gov` states that "as a convenience
for users, LSC provides access to some prior, superseded versions of Revised Code sections",
and each section page carries an **Available Versions of this Section** list. [verified]

A prior version is addressed by effective date, with no search step:

    codes.ohio.gov/ohio-revised-code/section-<number>/<M-D-YYYY>

Fetched `section-5747.50/9-29-2015` and it returns the version House Bill 64 of the 131st General
Assembly enacted — a biennium this corpus already models. [verified] The versions listed for that
section run back to 30 June 2007, comfortably before the FY2010 window.

**This is the cheapest `[inference]` to `[verified]` conversion available**, and unlike the other
unbuilt connectors nothing blocks it: the text is public, static, unauthenticated, and needs no
enumeration, because the corpus already knows which citations it wants to check.

### One caution found while scouting

The word "some" in LSC's own sentence is load-bearing and is not quantified. Coverage should be
measured per citation the corpus actually relies on rather than assumed from one good result.

And the local government fund is **not a percentage** in the 2015 text. It distributes on "the
county's proportionate share of the calendar year 2007 LGF and LGRAF distributions multiplied by
the 2007 LGF and LGRAF county distribution base available in that month". [verified] The claim
above that this connector "supplies the historical percentages" describes the pre-2011 mechanism;
after that the statute freezes a 2007 base and distributes shares of it. Anything reading the
[local government fund](../../.yidam/corpus/fund/local-government-fund.yml) story as a percentage
cut is describing the wrong instrument for most of the window this corpus covers — which makes
this connector a correction to a seeded claim, not only a confirmation of one.
