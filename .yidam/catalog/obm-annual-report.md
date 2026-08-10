---
slug: obm-annual-report
name: Ohio audited annual financial report and monthly financial reports
source_type: financial-report
publisher: Ohio Office of Budget and Management
location: https://archives.obm.ohio.gov/Files/Budget_and_Planning/Monthly_Financial_Report/
content_committed: true
feeds:
  - expenditure
  - fund
  - forecast
---

# OBM financial reporting

The Office of Budget and Management publishes monthly financial reports during a fiscal year
and an audited annual financial report after it closes. Together they are the only
authoritative source for what Ohio actually spent, as opposed to what it authorized.

## Two bases, deliberately kept apart

Monthly reporting gives `disbursed` figures; the annual report gives `actual-closed` figures.
They routinely disagree for the same line item and year, and the disagreement is not error —
it reflects timing, encumbrance, and year-end adjustment.

Any comparison that mixes them is invalid. The
[`obm-expenditure-row`](../schemas/extraction/obm-expenditure-row.schema.json) schema carries
`basis` as a required field for exactly this reason, and the
[`gap`](../skills/gap.md) calculator refuses to difference across bases.

## The open question that constrained the whole design, answered

This entry called it "the single largest unresolved risk to the repository's central question"
and said answering it should be part of the first extraction phase. It was not. It is answered
now, and the answer is the constraining one.

| File | sha256 | Contents |
|---|---|---|
| [`monthly-financial-report-2026-01.pdf`](../sources/obm/monthly-financial-report-2026-01.pdf) | `f16e1126…` | January 2026 report: GRF revenue and disbursement against estimate, fund balance tables |

**Monthly reporting is not available at line item granularity.** Disbursements are reported by
spending category — Primary and Secondary Education, Higher Education, Medicaid, Justice and
Public Protection, Health and Human Services, about a dozen in all — and by fund. Searching the
January 2026 report for every appropriation line item this corpus models returns **nothing**:
zero occurrences of 200550, 651525, 110969, 235501, 501321 or 110965. [verified]

Three consequences, and the third is structural:

1. **In-year gap analysis at line item level is impossible from this source.** It can compare a
   category against its estimate, which is a different and coarser question.
2. **The `disbursed` basis has no line-item source and probably never will.** It exists on the
   [`expenditure`](../corpus/expenditure.ont.yml) class to keep in-year figures apart from
   closed-book ones, and the corpus holds exactly one node carrying it — with an `[open]` amount.
   The distinction was correct and the data behind one side of it does not exist at this
   granularity.
3. **A currently-open fiscal year can never be gap-computed at line item level.** The corpus's
   line-item actuals come entirely from LSC's workbooks, which report completed years. FY2026 and
   FY2027 become computable when LSC publishes them, roughly two years after the fact, and not
   before.

That is a real limit on what this repository can ever answer, and it is better stated than
discovered by a reader wondering why the newest year is missing.

## The audited annual report: no line items either, and a correction

| File | sha256 | Contents |
|---|---|---|
| [`acfr-2024.pdf`](../sources/obm/acfr-2024.pdf) | `88b05006…` | Annual Comprehensive Financial Report, fiscal year ended 30 June 2024 |

**It carries no appropriation line item detail.** Same test as the monthly report and the same
result: zero occurrences of 200550, 651525, 110969, 235501, 501321 or 110965 in 1.2 million
characters. [verified] Budget-to-actual comparisons are presented per governmental fund. So
nothing in OBM's reporting reaches line-item granularity, and the corpus's line-item actuals
depend on LSC alone — which is the structural limit recorded above, now confirmed against both
documents rather than one.

**And it corrects something this corpus recorded.** The
[`expenditure`](../corpus/expenditure.ont.yml) class was annotated with the claim that authority
resolves three ways — disbursed, encumbered, lapsed — and that the corpus models the first and
third but not the second, so an apparent underspend might be money committed and not yet paid.
The ACFR says otherwise about the basis these figures are kept on:

> in the non-GAAP budgetary basis schedules, "actual" budgetary expenditures include cash
> disbursements **and outstanding encumbrances**, as of June 30

[verified] On the budgetary basis an encumbrance *is* an expenditure. That is also why OBM's
glossary can say a closed-year adjusted appropriation equals disbursements plus outstanding
encumbrances: the two definitions describe one quantity.

[inference] LSC's workbooks report against the same appropriation control the state uses, so
their `actual` columns are budgetary basis and already include encumbrances — which would mean
the corpus's figures were never missing them and the gap is not understating spending by the
encumbered amount.

[open] That last step is an inference and the corpus should not rest on it. LSC does not state a
basis anywhere in the workbooks. What would settle it: aggregating the corpus's General Revenue
Fund actuals for FY2024 and comparing against the ACFR's General Fund budgetary expenditure for
the same year — they should agree if both are budgetary basis, and differ by roughly the $2.08
billion of General Fund encumbrances the ACFR reports at 30 June 2024 if they are not.

## The URL convention, which the controlling board lacks

`archives.obm.ohio.gov/Files/Budget_and_Planning/Monthly_Financial_Report/YYYY-MM-mfr.pdf`,
with inconsistent separators — `2022-12_mfr.pdf` and `2022_11-mfr.pdf` both occur. Reports are
enumerable by date, which is exactly what the
[controlling board archive](./obm-controlling-board-manual.md) does not offer.

## Extraction

Feeds via the [`obm`](../../crates/obm/) connector.
