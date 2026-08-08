# program

Policy programs, held separate from the line items that fund them because the two do not
correspond one to one. A single program is commonly funded across several line items, and a
single line item commonly funds several programs. Collapsing the two — the most tempting
simplification in this domain — makes it impossible to answer either "what does this program
cost" or "what does this line item buy" reliably.

Three programs are seeded:

- **Medicaid** — the largest, and the one whose cost is least under legislative control.
- **Fair School Funding Plan** — a formula program, phased in across multiple biennia, which
  makes it the case where the sixteen-year window earns its keep.
- **Highway system preservation** — funded from constitutionally restricted revenue, and the
  instance that populates the approved `program →[draws-from]→ revenue-source` edge.

That last edge is the one that keeps the corpus honest about fungibility. Most programs
compete for general revenue; earmarked ones do not. Without the edge, the graph would imply a
competition that does not exist.

See the class definition at [`../program.ont.yml`](../program.ont.yml).
