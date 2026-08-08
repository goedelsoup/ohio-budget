# legislature

**Capability type:** connector. **Status:** implemented, offline. Network retrieval pending.

Retrieves bill metadata, text references, and stage history from the Ohio General Assembly's
published record.

## Feeds

| Class | What it supplies |
|---|---|
| [`bill`](../../.yidam/corpus/bill.ont.yml) | Bill number, assembly, introduction and enactment dates |
| [`bill-version`](../../.yidam/corpus/bill-version.ont.yml) | Stage, date, chamber, document reference |
| [`budget-action`](../../.yidam/corpus/budget-action.ont.yml) | Amendment events and their dates |

## What is implemented

Everything except the network call. `BillRef` keys on the (assembly, number) pair, because bill
numbers recur across General Assemblies for unrelated legislation and `HB 96` alone identifies
nothing. `parse_stage` normalizes the published record's several spellings of each stage onto
the corpus's single vocabulary, and returns `None` rather than guessing when it meets one it
does not know — a stage guessed wrong puts figures on the wrong node.

`version_node_slug` and `missing_version_slugs` reconcile a fetched record against the corpus
and report which `bill-version` nodes are missing. They return findings; they do not write.

## HttpSource fails rather than falling back

`HttpSource` returns an error naming the URL it would have fetched. It deliberately does not
fall back to fixtures. A connector that silently serves synthetic data when the network is
unavailable would put fabricated figures into a run that believed it fetched them — and the
resulting corpus would be indistinguishable from one built on real data.

Implementing it is a thin wrapper over `document_url`, which is pure and tested.

## Open

- [open] Whether stage history is exposed in a stable machine-readable form, or must be derived
  from document listings. This decides whether the HTTP source is a client or a scraper.
