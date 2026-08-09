---
slug: lsc-hb96-comparison
name: Legislative Service Commission comparison document, HB 96 (136th General Assembly)
source_type: legislative-document
location: https://www.lsc.ohio.gov/assets/legislation/136/hb96/en0/files/hb96-edu-comparison-document-as-enacted-136th-general-assembly.pdf
publisher: Ohio Legislative Service Commission
content_committed: true
feeds:
  - budget-action
  - bill-version
---

# LSC comparison document — HB 96

- **Retrieved:** 2026-08-08
- **Committed at:** [`.yidam/sources/lsc/hb96-edu-comparison-document-as-enacted-136th.pdf`](../sources/lsc/hb96-edu-comparison-document-as-enacted-136th.pdf) (Education and Workforce)
- **sha256:** `8ec8ec5c842fb697f68787e60c7490e5b7921f2a30bd8e1531558c1991a1a52f`

## This entry previously described the wrong document

This entry has now been wrong twice, in opposite directions, and both corrections came from
reading further into the document rather than from any external check.

**At genesis** it claimed the source supplies "the amount, per line item, per stage" and fed
`appropriation` and `line-item`. It does not. A comparison document compares *provisions*: four
columns of prose — Executive, As Passed By House, As Passed By Senate, As Enacted — with one
entry per point of one provision.

**The correction then overshot**, saying the document held "no dollar amounts at all". It holds
a great many, in `Fiscal effect:` sentences appended to provisions, and they are the figures
this entry's own analysis now rests on. What is true is narrower and more useful: the amounts
here are **distribution estimates**, not appropriation authority, and the per-line-item stage
series still comes from the
[appropriation spreadsheet](./lsc-hb96-appropriation-spreadsheet.md).

## What it is actually for

The `stated_justification` field on every [`budget-action`](../corpus/budget-action.ont.yml)
node in this corpus was `[open]`. This is the source that closes them, and for HB 96 foundation
funding it now has.

The earlier draft of this section guessed at the content and got the attributions wrong. It
said the document shows the House changing "base cost per pupil, career-technical base cost,
the disadvantaged pupil calculation, the treatment of temporary transitional aid". Reading
provision EDUCD26:

- the FY2024 statewide average **base cost per pupil** and **career-technical base cost** were
  the *Executive's* changes; the House recorded "Same as the Executive" on both;
- the **disadvantaged pupil impact aid** recalculation was the *Senate's*, and the House had no
  provision on it at all;
- what the House actually did was retain the executive's foundation aid calculations "only for
  purposes of calculating a district's **temporary foundation funding**" — not "temporary
  transitional aid", which is a different thing that this entry invented.

Three of four attributions went to the wrong chamber. That is the failure mode of writing a
catalog entry from what a document ought to contain: the entry stays plausible, cites a real
source, and misassigns the decisions.

The division it drew is still right, and is the reason to hold both sources: the spreadsheet
carries the arithmetic, this carries the decision.

## Extraction

Implemented in [`lsc::ruled`](../../crates/lsc/src/ruled.rs) and
[`lsc::comparison`](../../crates/lsc/src/comparison.rs). Read with
`lsc-comparison <pdf> [--provision CODE] [--changed] [--emit FILE]`.

Coverage on this document: **212 pages, 0 refused, 165 provisions, 781 entries, 3,124
records.** Every column of every entry carried text; no provision lacked an LSC code; all four
column headings mapped to stages. The 781 entries match the 781 drawn rule rows counted
independently of the parser.

### The previous note here described the wrong mechanism

It said the document "separates its own columns with a literal `|` character in the body text"
and that an extractor should split on that. The `|` glyphs are real, but they are not inline
separators — they are **column rules**, drawn on their own baseline beneath the first line of
each entry, three per entry at x = 251.9, 499.4, and 746.9. They appear at those exact
coordinates on all 781 entries across all 212 pages without deviation.

That makes them better than the note supposed. Splitting text on a delimiter recovers columns;
reading the rules recovers the column *boundaries*, which additionally survives a column that
prints nothing on a page, and the same glyphs delimit the entries.

### Why river detection is not merely imperfect here

Measured across the first forty pages at a 12pt body font:

| boundary | left column ends | right column starts | gutter |
|---|---|---|---|
| Executive → As Passed By House | 251.62 | 252.06 | **0.44** |
| As Passed By House → As Passed By Senate | 494.96 | 513.00 | 18.04 |
| As Passed By Senate → As Enacted | 737.51 | 762.48 | 24.97 |

Only the first boundary is flush. The earlier note said there was no river between Executive
and As Passed By House, which was right, and inferred that the whole table was unreadable by
geometry, which was wrong. Rivers find the two wide gutters and miss the narrow one, returning
a well-formed table of the wrong shape — three columns, with the executive proposal and the
House's position fused into a single cell. That is worse than a refusal, and it is why the
ruled reader does not fall back to river detection.

## It also records the line-item vetoes

The document marks vetoes twice, independently, and both are extracted:

- on a provision **heading**, as a `**VETOED**` or `**PARTIALLY VETOED**` prefix — whether the
  provision survived;
- **inline**, as `[***VETOED: … ***]` around each struck passage — which parts did not.

In this document: 8 provisions marked, **56 struck passages**, all recovered with none left
unterminated. The count matches an independent scan of the document's raw text.

Four strikes are mid-sentence rather than whole-passage — the governor removed `of up to
$10,000` from a research grant provision and left the rest standing. An extractor treating the
marker as a whole-cell wrapper, which the first version here did, leaves those four unparsed
with the raw annotation sitting inside the stored text.

`LscProvisionRow.position` keeps the struck words, because the chamber did pass them; the
strikes are listed separately in `vetoed_spans`. The position is what a chamber did, the spans
are what the executive then removed.

**Its veto markers are dated, not final.** The document is published at enactment, 30 June
2025. Veto item 66 was overridden by the House on 21 July and the Senate on 1 October, so the
ten passages this document marks struck under provision TAXCD91 became law on or about 30
December 2025. An extraction run over this document produces records that were true when it was
published; nothing in the document says so. See
[the override](../corpus/budget-action/hb96-veto-override-item-66.yml).

**This does not establish the vetoes' dollar effect**, and a claim resting on it was withdrawn
from three nodes. This document says what language was struck; it does not say whether the
appropriation spreadsheet's as-enacted column is taken before or after the governor acts. See
[the veto action](../corpus/budget-action/hb96-line-item-veto.yml).

## What its figures are and are not

`Fiscal effect:` sentences quote dollar amounts. Those are **distribution estimates for a
recipient class**, not appropriation authority, and across HB 96's FY2026 school funding the
two move in opposite directions at two of three transitions. They are never differenced
against spreadsheet figures — see
[appropriation-is-not-distribution](../decisions/appropriation-is-not-distribution.yml).
