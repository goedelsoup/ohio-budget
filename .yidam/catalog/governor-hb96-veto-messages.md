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

Implemented in [`crates/governor`](../../crates/governor/). Read with
`governor-vetoes <pdf> [--item N] [--emit FILE]`.

Coverage on this document: **67 items, 773 deletion instructions, none unrecognised.** Every
item is numbered without a gap, deletes something, and closes with the standard formula. Extents
break down as 420 quoted passages, 211 ranges, and 142 whole-page deletions.

### What the document does to a parser

Four defects in the source, all recorded rather than smoothed over:

- **Three missing quotation marks.** `and ending with the effective date of this section.”`
  opens nothing; `and ending with “at large.` closes nothing. The words are unambiguous from
  the surrounding grammar, so they are recovered and each record is marked `quotes_repaired`.
- **Typographical variants for the same act** — `begging with`, `stating with`, `boxed test`,
  `or ending with`, and bare `delete “X”`. The connector classifies by structure (how many
  quoted segments, and whether a range connector appears) rather than by phrasing, so a
  typesetter's slip cannot drop a real deletion.
- **Quoted text ending in an ellipsis.** `“…means a patient...”` reads as a finished sentence
  to any "ends with a period" test, which then takes the instruction's own continuation to be
  the item's heading. Two items lost their headings this way before the check was fixed.
- **One heading wrapped across two lines** (item 45). Headings are set in bold and so wrap
  earlier than the body around them; the rejoin is flagged on the record.

### A note on line breaks

`pdf-extract` and `pdftotext` do not agree on this document. Poppler silently rejoins words
hyphenated by justification; `pdf-extract` preserves the visual break, leaving `end-` and `ing`
on separate lines. The connector rejoins them under a rule derived from all 14 breaks in the
document, which keeps `90-credit-hour`, `DeWine-Tressel`, `ADD-ON`, and `2026-2027` while
mending `func-tions`, `end-ing`, and `other-wise`.

That rule is the **opposite** of the one [`lsc`](./lsc-hb96-comparison.md) uses, which keeps
every hyphen. Neither is wrong: LSC wraps at hyphens already in the word, the governor's office
hyphenates by justification. It is worth stating because "how to rejoin a wrapped line" looks
like a settled question and is actually a property of the publisher.
