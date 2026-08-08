# appropriation

Legal authority to spend a stated amount, from a stated fund, for a stated line item, in a
stated period, as fixed by one version of one bill.

This is the corpus's only **relator**. It is not an edge between a line item and a fund — it is
a node in its own right, because the connection carries properties that belong to neither
endpoint: the amount, the stage at which that amount stood, and the record of every action that
moved it. Modeling it as an edge would force the amount onto the line item, which would then
need one amount per period per version, which is a node in all but name.

The practical consequence is that a single line item in a single period has *several*
appropriation nodes — one per bill version — and they disagree with each other by design.
Disagreement between them is the signal, not noise.

Four are seeded. Three are for [foundation funding](../line-item/foundation-funding.yml): two
stages within FY2026 and one in the prior biennium, so that both stage movement and
cross-biennium comparison are demonstrable on a single line item. The fourth covers
[Medicaid](../line-item/medicaid-health-care-services.yml) as enacted.

**All amounts are `[open]`.** Every figure here would come from an LSC comparison document that
is not yet catalogued. Rather than assert plausible numbers, the nodes carry the structure with
the amounts left explicitly unfilled. A corpus that states unsourced dollar figures at this
granularity is worse than one that states none — the figures are precise, citable, and wrong in
ways nothing downstream can detect.

See the class definition at [`../appropriation.ont.yml`](../appropriation.ont.yml).
