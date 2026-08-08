# controlling-board

**Capability type:** connector. **Status:** stub — not implemented.

Retrieves Controlling Board agendas, requests, and minutes — the record of appropriation
changes made after enactment.

## Feeds

| Class | What it supplies |
|---|---|
| [`budget-action`](../../.yidam/corpus/budget-action.ont.yml) | Execution-phase transfers and adjustments, with dates, amounts, and agency justifications |

## Why it matters more than its size suggests

This is the only source for the execution phase. An analysis that stops at the enacted bill
treats the budget as a decision when it is closer to a decision plus two years of adjustments,
and this connector supplies the adjustments. The seeded
[controlling board transfer](../../.yidam/corpus/budget-action/controlling-board-medicaid-transfer.yml)
asserts the pattern rather than a verified event, and stays that way until this runs.

## Interface sketch

```
fetch_agenda(date: NaiveDate) -> Result<Vec<ControllingBoardRequest>>
fetch_requests(period: FiscalPeriod) -> Result<Vec<ControllingBoardRequest>>
```

## Open questions

- [open] The aggregate dollar volume passing through controlling board action in a typical
  biennium. This is the first number the connector should produce, because it settles whether
  the execution phase is marginal or material — and the corpus currently assumes material
  without evidence.
