---
slug: obm-controlling-board-manual
name: A Guide to the Ohio Controlling Board (Office of Budget and Management)
source_type: executive-document
location: https://archives.obm.ohio.gov/Files/Controlling_Board/Resources/Controlling_Board_Manual/
publisher: Ohio Office of Budget and Management
content_committed: true
feeds:
  - budget-action
---

# A Guide to the Ohio Controlling Board

| File | sha256 | Contents |
|---|---|---|
| [`controlling-board-manual-2023-05-04.pdf`](../sources/obm/controlling-board-manual-2023-05-04.pdf) | `7fbad1e0…` | 42 pages: request types, thresholds, exemptions, and the statutory authority for each |

- **Retrieved:** 2026-08-09, dated 4 May 2023

## It is the specification, not the data

Committed because two of this corpus's findings name the controlling board as the thing that
would settle their remaining question — [what the gap actually
says](../decisions/what-the-gap-actually-says.yml) and the appropriation-adjustment test inside
it. Before writing a connector it is worth knowing what the connector would read.

**The manual describes the mechanism and is reachable. The agendas, minutes and approved requests
are not.** Both `obm.ohio.gov/wps/portal/gov/obm2/controlling-board/agendas-and-minutes` and the
approved-requests search return HTTP 404 to a plain fetch, the archive host serves individual
files but refuses directory listing with 403, and the requests themselves sit behind a search
form rather than at predictable URLs. [verified]

So the connector is not blocked on effort. It is blocked on enumeration: there is no way from
here to discover which requests exist without driving a form.

## What it establishes

That transfers of appropriation authority between line items are made under R.C. 127.14 and
127.15, that they are the board's own statutory power rather than an agency's, and that a
substantial list of purchases is exempt from board review regardless of amount. [verified]

That is enough to state what a controlling board action *is* with a citation. It is not enough to
say which ones happened, and the corpus's single
[controlling board transfer](../corpus/budget-action/controlling-board-medicaid-transfer.yml)
stays an assertion of the pattern rather than a verified event.
