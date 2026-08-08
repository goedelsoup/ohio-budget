# revenue-source

The instruments that produce money for the state. A revenue source is held separate from the
fund its proceeds land in because the two are governed by different law and change on
different schedules: a rate change is a tax policy decision, while a change in how proceeds
are split across funds is a distribution decision, and the same statute rarely does both.

Three instruments are seeded, chosen because each demonstrates a different constraint the
corpus has to represent:

- **Personal income tax** — the instrument that changed most across the sixteen-year window,
  moving from a graduated bracket structure toward a flat rate. Any long-series revenue claim
  has to account for it.
- **Sales and use tax** — the stable high-volume counterweight, and the instrument whose
  local piggyback rates make the state and local pictures diverge.
- **Motor fuel tax** — constitutionally restricted, which breaks the assumption that revenue
  is fungible once it reaches a fund.

Instruments are distinguished from the programs they earmark to. Where a program draws
directly on an instrument rather than through general appropriation, that is an explicit
`draws-from` edge rather than an implicit fact about the fund.

See the class definition at [`../revenue-source.ont.yml`](../revenue-source.ont.yml).
