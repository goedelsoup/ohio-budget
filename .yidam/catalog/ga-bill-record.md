---
slug: ga-bill-record
name: Ohio General Assembly bill record
source_type: legislative-document
publisher: Ohio General Assembly
location: General Assembly published bill status, text, and version history
content_committed: false
feeds:
  - bill
  - bill-version
  - budget-action
---

# General Assembly bill record

Bill numbers, sponsors, introduction and enactment dates, and the sequence of versions each
appropriation bill passed through.

## Built first, despite mattering least

This is the least analytically interesting of the five sources and the first that should be
extracted. It is structured and stable where the others are PDF tables and prose, and it
produces the skeleton the others hang figures on: an
[`lsc-comparison-row`](../schemas/extraction/lsc-comparison-row.schema.json) has nowhere to
attach until the [`bill-version`](../corpus/bill-version.ont.yml) it belongs to exists as a
node.

Ordering connector work by difficulty against dependency rather than by importance is
recorded as a decision in [`proposals.yml`](../decisions/proposals.yml).

## Bill numbers are not unique

They recur across General Assemblies for entirely unrelated legislation. Every reference in
this corpus qualifies a bill number with its assembly, and extraction must key on the pair —
`HB 96` alone identifies nothing.

## Open

[open] Whether stage history is exposed in a stable machine-readable form, or must be derived
from document listings. This determines whether the connector is a straightforward client or
a scraper with the fragility that implies.

## Extraction

Feeds via the [`legislature`](../../crates/legislature/) connector.
