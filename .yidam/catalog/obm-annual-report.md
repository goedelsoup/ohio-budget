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

## The URL convention, which the controlling board lacks

`archives.obm.ohio.gov/Files/Budget_and_Planning/Monthly_Financial_Report/YYYY-MM-mfr.pdf`,
with inconsistent separators — `2022-12_mfr.pdf` and `2022_11-mfr.pdf` both occur. Reports are
enumerable by date, which is exactly what the
[controlling board archive](./obm-controlling-board-manual.md) does not offer.

## Extraction

Feeds via the [`obm`](../../crates/obm/) connector.
