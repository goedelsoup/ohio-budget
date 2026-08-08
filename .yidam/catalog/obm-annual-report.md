---
slug: obm-annual-report
name: Ohio audited annual financial report and monthly financial reports
source_type: financial-report
publisher: Ohio Office of Budget and Management
location: Office of Budget and Management financial reporting publications
content_committed: false
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

## The open question that constrains the whole design

[open] Whether monthly reporting is available at line item granularity, or only at fund and
agency level. If only the latter, the gap calculation cannot operate at the granularity the
corpus models for open periods, and in-year analysis would be limited to agency totals. This
is the single largest unresolved risk to the repository's central question, and answering it
should be part of the first extraction phase rather than discovered later.

## Extraction

Feeds via the [`obm`](../../crates/obm/) connector.
