---
slug: lsc-hb96-comparison
name: Legislative Service Commission comparison document, HB 96 (136th General Assembly)
source_type: legislative-document
location: Legislative Service Commission budget publications for the 136th General Assembly
publisher: Ohio Legislative Service Commission
content_committed: false
feeds:
  - appropriation
  - bill-version
  - line-item
---

# LSC comparison document — HB 96

The Legislative Service Commission publishes, for each stage of an appropriation bill, a
document setting out every line item's amount as it stood at that stage. It is the
authoritative answer to the question this corpus asks most often: what did this line stand
at when this body acted.

## Why this entry is first

Every `amount` field in the corpus is currently `[open]`, and nearly all of them are waiting
on this one source. It is the highest-leverage catalog entry in the repository.

## Content is not committed

`content_committed: false`. This entry registers the source; it does not contain it.

That distinction is load-bearing and easy to lose. Registering a source does **not** license
promoting a claim from `[inference]` to `[verified]` — verification requires the source
content to be committed and citable, so that a later reader can check the figure rather than
check that a document was named. Until extraction runs and its output is committed, a node
citing this entry is saying *this is where the answer will come from*, not *this is
established*.

## Extraction

Feeds [`lsc-comparison-row`](../schemas/extraction/lsc-comparison-row.schema.json) records
via the [`lsc`](../../crates/lsc/) connector. Published as PDF with tabular figures, so
extraction is the hard part and table structure may vary across biennia.
