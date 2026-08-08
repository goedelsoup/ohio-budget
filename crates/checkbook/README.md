# checkbook

**Capability type:** connector. **Status:** stub — not implemented.

Retrieves state transaction-level transparency reporting.

## Feeds

| Class | What it supplies |
|---|---|
| [`expenditure`](../../.yidam/corpus/expenditure.ont.yml) | Aggregated disbursement detail |
| [`jurisdiction`](../../.yidam/corpus/jurisdiction.ont.yml) | Recipient identification |

## Aggregates, not nodes

Transaction granularity was explicitly ruled out of the corpus during ontology discovery —
individual payments are a data lake, not a knowledge graph. This connector therefore
**aggregates before returning**: its output is a disbursement total per line item, period, and
recipient, never one node per transaction.

That constraint is the whole reason this is a connector rather than a corpus import. Violating
it would flood the graph with nodes that carry no conceptual content.

## Interface sketch

```
fetch_aggregate(line_item: &str, period: FiscalPeriod) -> Result<Vec<RecipientTotal>>
```

## Open questions

- [open] Whether transparency reporting is reconcilable to [`obm`](../obm/) figures. If the two
  disagree, which is expected at period boundaries because of cash versus accrual treatment,
  the corpus needs a stated rule for which is authoritative.
