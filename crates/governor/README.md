# governor

**Capability type:** connector. **Status:** implemented for veto messages.

Reads the Ohio governor's veto messages — the executive's own statement of what it struck from
an appropriation bill and why.

## Feeds

| Class | What it supplies |
|---|---|
| [`budget-action`](../../.yidam/corpus/budget-action.ont.yml) | One action per vetoed item, with the governor's stated reason |
| [`actor`](../../.yidam/corpus/actor.ont.yml) | Evidence of how the item-veto power was actually exercised |
| [`bill-version`](../../.yidam/corpus/bill-version.ont.yml) | What separates the enrolled bill from the enacted one |

## What it produces, and what it never will

One [`VetoItemRow`](../corpus-schema/src/lib.rs) per numbered item: the deletions, the heading,
and the reason. **Never a dollar figure.**

That is not an omission. Across HB 96's 67 items not one deletes an appropriation amount — every
one strikes statutory or temporary-law language. But several move money anyway, by striking a
set-aside, an earmark, or a restriction on who may receive a line item. An item-level diff of the
bill before and after the veto returns nothing while 5% of State Share of Instruction changes
destination. See
[appropriation-is-not-distribution](../../.yidam/decisions/appropriation-is-not-distribution.yml).

## Usage

```
governor-vetoes <pdf|txt> [--item N] [--emit FILE] [--dump-text FILE]
```

On HB 96's message: 67 items, 773 deletion instructions, none unrecognised.

## Why it does not use the geometry from `lsc`

[`lsc`](../lsc/) reconstructs tables from glyph coordinates because a table's structure is
spatial and a PDF only stores positions. A veto message has no tables; its structure is its
reading order, which a text extraction already preserves. Borrowing the geometry here would add
a failure mode and buy nothing.

The two connectors do disagree on one detail, and the disagreement is real rather than
inconsistent. `lsc` keeps every hyphen at a line break because LSC wraps at hyphens already in
the word; this drops some, because the governor's message hyphenates by justification. The rule
here was chosen after reading all 14 breaks in the document, and it is written down in
`join_line` along with the case that would defeat it.
