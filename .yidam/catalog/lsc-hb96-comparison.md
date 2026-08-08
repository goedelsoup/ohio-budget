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

It claimed this source supplies "the amount, per line item, per stage" and fed
`appropriation` and `line-item`. **That was wrong, and reading the document is what showed
it.** A comparison document compares *provisions*, not figures. It is four columns of prose —
Executive, As Passed By House, As Passed By Senate, As Enacted — with one entry per policy
provision and no dollar amounts at all.

The numeric stage series comes from the
[appropriation spreadsheet](./lsc-hb96-appropriation-spreadsheet.md), which was catalogued
separately and does carry every stage.

## What it is actually for

The `stated_justification` field on every [`budget-action`](../corpus/budget-action.ont.yml)
node in this corpus is `[open]`. This is the source that closes them. Where the spreadsheet
says foundation funding moved +$92,250,000 at the House substitute, this document says what
changed in the formula — base cost per pupil, career-technical base cost, the disadvantaged
pupil calculation, the treatment of temporary transitional aid.

That division is worth stating plainly because it maps onto a distinction the corpus already
makes: the spreadsheet carries the arithmetic, this carries the decision. For a formula
program those are different things, and the genesis note on the House amendment said the
decision matters more.

## Extraction caution

The four stage columns are adjacent with no whitespace river between Executive and As Passed
By House, so `lsc::geometry` merges them regardless of how the river threshold is tuned —
verified at 1.2, 0.5, 0.35, and 0.2. Lowering it further fragments other columns instead.

The document separates its own columns with a literal `|` character in the body text. An
extractor for this document type should split on that rather than on whitespace geometry.
That is a per-document-type strategy, not a tuning value, and it is not yet implemented.
