# lsc

**Capability type:** connector. **Status:** stub — not implemented.

Retrieves Legislative Service Commission budget analyses and stage-comparison documents — the
authoritative record of what a line item stood at when a body acted.

## Feeds

| Class | What it supplies |
|---|---|
| [`appropriation`](../../.yidam/corpus/appropriation.ont.yml) | The amount, per line item, per stage, per period |
| [`bill-version`](../../.yidam/corpus/bill-version.ont.yml) | Document references fixing each stage's figures |
| [`line-item`](../../.yidam/corpus/line-item.ont.yml) | Codes, titles, agency and fund assignment |

## The load-bearing connector

Every `amount` field in the seed corpus is `[open]` pending this connector. It is the single
largest determinant of whether this repository becomes useful, and it is deliberately **not**
built first: its content is locked in PDF tables, and standing up
[`legislature`](../legislature/) first means there is somewhere to put the figures when
extraction works.

Table extraction from PDF may justify a Python package rather than a Rust crate — see
[`packages/`](../../packages/).

## Interface sketch

```
fetch_comparison(assembly: u16, bill: &str, stage: Stage) -> Result<Vec<AppropriationRow>>
fetch_agency_analysis(assembly: u16, agency_code: &str) -> Result<AgencyAnalysis>
```

## Open questions

- [open] Whether table structure is stable enough across years for one extractor, or whether
  each biennium's documents need their own handling.
- [open] Whether line item codes appear in the documents in a form that survives extraction, or
  must be reconciled against another source.
