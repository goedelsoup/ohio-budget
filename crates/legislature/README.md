# legislature

**Capability type:** connector. **Status:** stub — not implemented.

Retrieves bill metadata, text, and stage history from the Ohio General Assembly's published
record.

## Feeds

| Class | What it supplies |
|---|---|
| [`bill`](../../.yidam/corpus/bill.ont.yml) | Bill number, assembly, introduction and enactment dates |
| [`bill-version`](../../.yidam/corpus/bill-version.ont.yml) | Stage, date, chamber, document reference |
| [`budget-action`](../../.yidam/corpus/budget-action.ont.yml) | Amendment events and their dates |

## Why this one is built first

It is the structured, stable source among the seven, and it produces the skeleton the other
connectors hang their figures on. Nothing in [`lsc`](../lsc/) can be attached to a version that
does not yet exist as a node.

## Interface sketch

```
fetch_bill(assembly: u16, number: &str) -> Result<Bill>
fetch_versions(assembly: u16, number: &str) -> Result<Vec<BillVersion>>
```

Offline mode falls back to committed fixtures, per the connector conventions in
[directory guidelines](../../.yidam/.vendor/prelude/guidelines/directories.md#crates).

## Open questions

- [open] Whether stage history is exposed in a stable machine-readable form, or must be derived
  from document listings.
