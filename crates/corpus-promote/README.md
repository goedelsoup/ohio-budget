# corpus-promote

**Capability type:** calculator (reconciliation). **Status:** implemented, dry-run only.

Matches connector extraction records against corpus nodes and reports what each record would
change. **Writes nothing.**

## Why proposals rather than writes

Every promotion attaches a factual claim about Ohio's budget to a node. The failure mode of an
automatic writer is a corpus that fills itself with plausible figures nobody read — and once
written, an extracted figure and a fabricated one are indistinguishable.

## How a record finds its node

A row carries a line item code, a fiscal year, and a stage. Matching walks:

1. code → the [`line-item`](../../.yidam/corpus/line-item.ont.yml) node whose `current_code`
   matches, or failing that whose `historical_codes` contain it as a **whole token** — so `200`
   never matches inside `200550`;
2. line item → the [`appropriation`](../../.yidam/corpus/appropriation.ont.yml) node that grants
   authority for it with the same `period_label` and `stage`.

Anything that does not match is reported with the reason, never dropped.

## Three refusals

| Guard | Why |
|---|---|
| Synthetic provenance is refused outright | Fixture values are fabricated; one reaching a node would be indistinguishable from a real figure |
| A historical-code match is flagged for review | The promotion crosses a renumbering and asserts continuity — a human judgment, per the [lineage](../../.yidam/skills/lineage.md) rule |
| An uncommitted source forbids `[verified]` | Registering a catalog entry is not committing its content; such a figure may be filled but its claim stays `[inference]` |

That last guard reads `content_committed` from the catalog entry, which is why the field exists.

## Running it

```
mise run promote
```

Against the committed fixtures this refuses all five rows. That is the first guard working, not
a failure — and it is worth running once to see it.
