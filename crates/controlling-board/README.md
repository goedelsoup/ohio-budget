# controlling-board

**Capability type:** connector. **Status:** stub — **blocked on enumeration, not on effort.**

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

## Why it is still a stub

Scouted 2026-08-09, because two findings name this connector as the thing that would settle their
remaining question and an unexplained stub is worse than a documented obstacle.

**The specification is reachable and is committed**: the Office of Budget and Management's
[Controlling Board manual](../../.yidam/catalog/obm-controlling-board-manual.md), 42 pages, which
establishes that transfers of appropriation authority between line items are made under R.C.
127.14 and 127.15 and lists what is exempt from board review. That is enough to say what a
controlling board action *is*.

**The data is not.** [verified]

    obm.ohio.gov/.../controlling-board/agendas-and-minutes        HTTP 404
    obm.ohio.gov/.../controlling-board/search-for-prior-records   HTTP 404
    archives.obm.ohio.gov/Files/.../<known file>.pdf              a PDF
    archives.obm.ohio.gov/Files/.../<anything else>               18.6 kB of HTML, **HTTP 200**

**Probe the archive by content, not by status.** It answers 200 to everything, including
`this-cannot-possibly-exist-9137.pdf`, and returns a soft-404 page rather than an error. Six
guessed agenda paths all "succeeded" and all returned the same HTML; the manual returned 1.3 MB
of PDF. A scout that trusted the status code would have reported the agendas as reachable and
written a parser against an error page.

Individual files serve fine once their URL is known. Nothing offers a way to learn the URLs:
agendas sit behind a search form, and the obvious path conventions — `Agendas/2025/`,
`Meeting_Agendas/2025/`, date-stamped filenames in three formats — are not it. A connector could
parse a request perfectly and have nothing to point it at.

**What would unblock it**, in order of likelihood: a naming convention for agenda PDFs inferred
from a handful of known ones — attempted and unsuccessful, six paths tried; a public records request for the approved-requests dataset, which
OBM maintains as a searchable database and therefore holds in structured form; or driving the
search form, which this repository does not do.

[open] The corpus's one controlling board node asserts the pattern rather than a verified event
and stays that way. Its `AdjustedAppropriation` figures come from LSC's workbooks rather than
from the board's own record, and reach only the first weeks of a fiscal year — see
[what the gap actually says](../../.yidam/decisions/what-the-gap-actually-says.yml).
