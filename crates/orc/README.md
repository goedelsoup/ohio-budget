# orc

**Capability type:** connector. **Status:** stub — not implemented.

Retrieves Ohio Revised Code sections governing funds, revenue instruments, and distribution
formulas.

## Feeds

| Class | What it supplies |
|---|---|
| [`fund`](../../.yidam/corpus/fund.ont.yml) | Statutory basis, purpose, and use restrictions |
| [`revenue-source`](../../.yidam/corpus/revenue-source.ont.yml) | Authorizing chapter, rate basis, disposition percentages |
| [`program`](../../.yidam/corpus/program.ont.yml) | Authorizing statute and formula basis |

## Character

Slow-changing. This is a one-time seed plus occasional refresh rather than a live connector,
and it is the cheapest way to convert a large number of `[inference]` tags in the seed corpus
into `[verified]` ones — every statutory citation currently asserted from memory is a candidate.

It also supplies the historical percentages that make the
[local government fund](../../.yidam/corpus/fund/local-government-fund.yml) story checkable
rather than remembered.

## Interface sketch

```
fetch_section(citation: &str) -> Result<StatuteSection>
fetch_chapter(chapter: &str) -> Result<Vec<StatuteSection>>
```

## Open questions

- [open] Whether superseded versions are retrievable. The corpus needs the percentage as it
  stood in each biennium, not only as it stands now, and current-text-only access would make
  the historical series unverifiable from this source.
