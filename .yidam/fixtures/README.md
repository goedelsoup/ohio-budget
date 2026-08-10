# fixtures

Committed sample records conforming to the [extraction schemas](../schemas/extraction/), so
connectors can be built and tested offline before any network access exists, and so each
schema has a worked example rather than only a field list.

## These values are synthetic

**Nothing here is real Ohio budget data.** Every record carries
`provenance.catalog_slug: synthetic-fixture` and `provenance.retrieved: "1970-01-01"` —
both deliberately unmistakable.

Fixtures exist to exercise extractor code paths: does the parser handle a multi-stage
comparison table, a monthly report with a reversion column, a withdrawn controlling board
request. They do **not** exist to seed the corpus. A fixture value that reached a corpus
node would be a fabricated fact wearing the costume of an extracted one, which is precisely
the failure this repository's `[open]`-over-guessing policy is built to prevent.

That boundary is enforced, not merely stated: `corpus-validate` fails with an error if the
string `synthetic-fixture` appears anywhere in a corpus instance.

## Layout

```
lsc/                 appropriation figures by line item and stage
obm/                 disbursement figures, disbursed and actual-closed
obm/dasf/            year-end budgetary rows: original, final, and actual together
controlling-board/   execution-phase requests and their disposition
legislature/         bill metadata and stage history
```

Every file is a YAML list of records of one type. `cargo test -p corpus-schema` parses all
of them against their Rust types, so a fixture cannot drift from the schema it claims to
demonstrate — if the type changes and a fixture is not updated, the test fails.

## What each fixture demonstrates

| File | Exercises |
|---|---|
| `lsc/hb96-foundation-funding.yml` | One line item across three stages, so `stage-delta` has something to difference |
| `obm/fy2024-25-foundation-funding.yml` | Both bases (`disbursed`, `actual-closed`) and a reversion, so `gap` can be tested against a closed period |
| `controlling-board/fy2026-medicaid.yml` | Approved and withdrawn dispositions, so extraction does not assume every request took effect |
| `legislature/hb96.yml` | A bill with a partial stage list, so downstream code handles missing intermediate versions |
