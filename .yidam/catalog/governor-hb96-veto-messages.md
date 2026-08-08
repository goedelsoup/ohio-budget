---
slug: governor-hb96-veto-messages
name: Veto messages, Amended Substitute House Bill 96 (136th General Assembly)
source_type: executive-document
location: https://content.govdelivery.com/attachments/OHIOGOVERNOR/2025/07/01/file_attachments/3310799/Veto%20Messages.pdf
publisher: Office of the Governor of Ohio
content_committed: true
feeds:
  - budget-action
  - actor
  - bill-version
---

# Veto messages — HB 96

- **Retrieved:** 2026-08-08
- **Committed at:** [`.yidam/sources/governor/hb96-veto-messages-136th.pdf`](../sources/governor/hb96-veto-messages-136th.pdf)
- **sha256:** `2a985bfecea6dfdb2b4c57ac961480e0c0fc3bb07f9119f8e14697c4f41c27ff`
- 57 pages, dated 30 June 2025

The governor's own statement of what he struck from HB 96 and why, issued under Article II,
Section 16. **67 numbered items**, each giving the deletions by bill page and boxed text,
a title, and a reason ending "Therefore, a veto of this item is in the public interest."

This is the source the corpus had been missing. Three nodes carried a claim about the vetoes'
dollar effect that rested on an inference the appropriation spreadsheet could not support; this
document settles the question directly.

## What it establishes

**No item in the message deletes an appropriation amount.** All 67 strike statutory text or
temporary-law language: eligibility rules, administrative duties, reporting requirements, tax
levy mechanics, set-asides. No rationale describes reducing or eliminating an appropriation,
and the phrase does not occur in the document.

That reinstates, on proper grounds, a claim previously withdrawn for want of them. It is
**not** the same claim as "the vetoes moved no money", and the difference is the whole point —
see below.

## The finding that matters more

At least three vetoes moved money **inside** a line item while leaving its total untouched:

- **Item 6**, State Share of Instruction: struck a set-aside requiring **5% of SSI** to be
  allocated by enrolment in 90-credit-hour degree programmes and College Credit Plus pathways.
  The appropriation is unchanged; 5% of it is redirected.
- **Item 19**, Waterways Improvement: struck three dredging earmarks in the GRF Parks and
  Recreation line item, with the administration directing ODNR to fund the projects from the
  Waterways Safety Fund instead. Same total, different source, different purposes.
- **Item 22**, Youth Homelessness: struck a restriction barring the Department of Health from
  distributing line item 440672 to shelters that "promote or affirm social gender transition".
  The line's amount is unchanged; who may receive it is not.

So an item-level comparison of the bill before and after the vetoes shows zero change, and
substantial sums moved anyway. This is the third independent instance in this corpus of the
same structural fact — see
[appropriation-is-not-distribution](../decisions/appropriation-is-not-distribution.yml), which
was recorded from the House and Senate amendments before this source was read.

## It cross-checks the comparison document

Eight provisions in the [LSC comparison document](./lsc-hb96-comparison.md) carry veto markers,
and each corresponds to an item here — `EDUCD118` to item 27, `BORCD109` to item 5, `PENCD8` to
51, `TAXCD91` to 66, `TAXCD107` to 65, `TAXCD110` to 55, `OBMCD53` to 50.

Item 5 confirms the finest-grained extraction the comparison connector performs. LSC marks
`of up to $10,000` as struck mid-sentence from a research grant provision; the veto message
says the item "would limit awards granted by the Consortium to $10,000" and that the cap would
deter strong proposals. Two independently published documents, agreeing on a four-word strike.

## What it does not settle

Whether LSC's as-enacted spreadsheet column is taken before or after the governor acts is still
unstated by any committed source. It no longer matters for the figures — if no veto changed an
amount, both readings give the same numbers — but it would matter for a bill where one did, and
should not be assumed.

## Extraction

Not implemented. The structure is regular enough to parse — `ITEM NUMBER n`, deletions, title,
rationale — and a `governor` connector reading it into `budget-action` nodes, one per item,
is the obvious next connector. The reading above was done by hand.
