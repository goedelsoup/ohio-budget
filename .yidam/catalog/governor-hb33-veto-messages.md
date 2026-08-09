---
slug: governor-hb33-veto-messages
name: Veto messages, Amended Substitute House Bill 33 (135th General Assembly)
source_type: executive-document
location: https://ohiocapitaljournal.com/wp-content/uploads/2025/05/VetoMessageHB33_Final.pdf
publisher: Office of the Governor of Ohio
content_committed: true
feeds:
  - budget-action
  - actor
  - bill-version
---

# Veto messages — HB 33

- **Retrieved:** 2026-08-09
- **Committed at:** [`.yidam/sources/governor/hb33-veto-messages-135th.pdf`](../sources/governor/hb33-veto-messages-135th.pdf)
- **sha256:** `94c8dbad9abd9eb4a5b24f3c10eab96e3cab03cc9fdb857b0113f9b5f7a1d77a`
- 29 pages, dated 3 July 2023, **44 items**

## A provenance caveat that belongs at the top

**The host is not the issuer.** This copy was retrieved from the Ohio Capital Journal, a news
outlet, because no copy served by the governor's office or the General Assembly could be
located. The [HB 96 message](./governor-hb96-veto-messages.md) came directly from the governor's
own distribution.

What can be checked has been: the document opens with the same formula as HB 96's, cites the
same constitutional authority, is dated the day the bill was signed, and contains 44 numbered
items — matching the count in contemporary reporting. Nothing suggests it is anything other
than the official message.

What cannot be checked is whether this file is byte-identical to what the governor issued.
[open] Claims resting on it should be read with that in mind, and it should be replaced if an
official copy is found.

## What it establishes

The same result as HB 96, in the previous biennium: **no item reduces or eliminates an
appropriation.** Those phrases do not occur in the document. All 44 strike statutory or
temporary-law language. [verified]

Item 15 is the closest thing to an exception and proves the rule — it deletes a cap reading
"not exceed $600,000,000" on transfers of Budget Stabilization Fund interest. A limit written in
language, not a line item's amount.

## What it did to the connector

`governor-vetoes`, unchanged, read 44 items and 340 instructions on the first run and left 12
unrecognised. Four defects in one document, none of which HB 96 exhibits:

- **Ranges that end positionally** — `and continuing to the bottom of the page`, used 21 times
  here and never in HB 96. Modelled as a new extent, `RangeToPageEnd`, rather than as a `Range`
  with invented closing words: the endpoint depends on where the enrolled bill's page breaks.
- **Two instructions on one line**, `…"5747.025," On page 2701, delete…`, which handed the
  second one's page number and quoted text to the first.
- **Justified spacing** — `On  page  2701,  delete` with doubled spaces. Every phrase the parser
  matches on is written single-spaced, so this made an instruction invisible while looking
  entirely ordinary to a reader.
- **A quotation that closes without a sentence period** — `delete the following boxed text,
  "5747.025,"`. The instruction never looked finished, absorbed the item's heading, and left
  item 13 reporting no stated reason at all.

After the fixes: **44 items, 341 instructions, none unrecognised, every item giving a reason.**
HB 96 re-reads identically at 67 items and 773 instructions, so nothing was traded away.

That is the argument for a second document. Each of these four would have gone on being an
unknown-unknown for as long as the connector was only ever pointed at the file it was written
against.
