---
slug: ga-bill-record
name: Ohio General Assembly bill record
source_type: legislative-document
publisher: Ohio General Assembly
location: https://www.legislature.ohio.gov/legislation/136/hb96/status
content_committed: true
feeds:
  - bill
  - bill-version
  - budget-action
---

# General Assembly bill record

Bill numbers, sponsors, introduction and enactment dates, and the sequence of versions each
appropriation bill passed through.

- **Retrieved:** 2026-08-08
- **Committed at:** [`.yidam/sources/legislature/hb96-status-136th.html`](../sources/legislature/hb96-status-136th.html) (HB 96 only)
- **sha256:** `f7d89ac1eba4fe1c783edb99720e295dd0074b1ae04f11522512933047cdd3e0`

## It records the vetoes being overridden

The reason this entry stopped being the least interesting source. Its action table carries two
entries no other source in this corpus has:

    7-21-2025   House    Item passed notwithstanding objections of the Governor
    10-1-2025   Senate   Item passed notwithstanding objections of the Governor

The [veto message](./governor-hb96-veto-messages.md) and the
[comparison document](./lsc-hb96-comparison.md) are both dated to enactment and neither can
know this. Both describe the bill as of 30 June 2025, and one of the governor's 67 items did
not survive that state — see
[the override action](../corpus/budget-action/hb96-veto-override-item-66.yml).

That is a general caution about "as enacted" documents rather than a defect in these two. An
enacted budget keeps changing, and a source published at enactment is a snapshot of a moment
that its own title invites you to read as an end state.

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
