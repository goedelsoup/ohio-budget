# line-item

The atomic unit of legislative spending authority, and the class where this corpus's most
consequential modeling decision lives.

A line item node is a **durable identity**, not a code. Appropriation line item codes are
renumbered and retired across biennia, and they do so at exactly the moments that carry the
most analytical weight — when a formula is restructured, when an agency is reorganized, when a
program is split or consolidated. Treating the code as the identity is correct for most lines
most of the time and fails silently precisely where it matters. So the code is a property that
changes over time, and continuity across a renumbering is asserted explicitly through
`succeeds` and `superseded-by` edges.

Those edges are assertions, not facts. The [`lineage`](../../skills/) calculator proposes
candidate pairs and scores them, but its output must never be written to these edges without a
contributor confirming the pair. A wrong lineage assertion silently fuses two unrelated funding
histories into one series, and the resulting chart looks entirely reasonable.

Four line items are seeded, one per major policy area, chosen so that each demonstrates a
different property of the class: formula distribution, entitlement-driven cost, statutory
pass-through, and restricted-fund capital work.

**On codes.** Only [foundation funding](./foundation-funding.yml) carries a code stated with
any confidence, and it is tagged `[inference]`. The others are marked `[open]` rather than
guessed. A fabricated appropriation line item code is worse than an acknowledged gap: it is
citable, it looks authoritative, and nothing downstream will flag it.

See the class definition at [`../line-item.ont.yml`](../line-item.ont.yml).
