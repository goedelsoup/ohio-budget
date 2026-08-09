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
| [`controlling-board-glossary.pdf`](../sources/obm/controlling-board-glossary.pdf) | `7bb239b5…` | 6 pages defining the board's terms, including **adjusted appropriation** |

- **Retrieved:** 2026-08-09; the manual is dated 4 May 2023

## The glossary settles what an adjusted appropriation is

> **Adjusted Appropriation.** The amount of the original appropriation minus any executive order
> reductions plus net transfers. **For prior budget fiscal years, the adjusted appropriation
> always equals the sum of disbursements and outstanding encumbrances.**

[verified] Two things follow, and the second is a trap the corpus walked past rather than around.

**Two mechanisms, not one.** The corpus's extractor describes the adjusted figure as authority
after "controlling board transfers and the like". The glossary names the other component:
executive order reductions, which are the governor's and not the board's.

**A closed-year adjusted appropriation cannot test whether adjustment explains the gap.** For a
prior year it is *defined* as disbursements plus encumbrances, so asking whether the outturn
tracks it would return "almost exactly" by construction and mean nothing at all. See
[what the gap actually says](../decisions/what-the-gap-actually-says.yml), whose test survives
only because all three adjusted figures available happen to be current-year — taken two to three
months into the fiscal year they describe. That was luck.

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
