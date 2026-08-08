# fund

The accounts money sits in. A fund is distinct from both the revenue that fills it and the
appropriation that empties it, because all three are governed separately: money can sit in a
fund without being appropriated, and can be appropriated without being available.

Four funds are seeded, chosen to span the range of constraint the corpus has to model:

- **General revenue fund** — the unrestricted pool where most policy argument happens.
- **Budget stabilization fund** — a reserve rather than an operating account. Money moves in
  and out by transfer, not by appropriation, which is why the approved `transfers-to` edge
  exists.
- **Local government fund** — a distribution account that exists to pass money out of state
  government by formula.
- **Highway operating fund** — constitutionally restricted, and therefore the case that
  proves money is not fungible at the fund level.

The distinction between fund *group* and fund *purpose* is worth holding onto. Group is an
accounting classification; purpose is a legal restriction. Two funds in the same group can
have entirely different degrees of freedom.

See the class definition at [`../fund.ont.yml`](../fund.ont.yml).
