# bill-version

One stage of an appropriation bill, as a distinct artifact carrying its own set of figures.

The class exists to answer, without replaying a chain of amendments, questions of the form
*what did this line stand at when the House passed it*. Those questions are asked constantly
in this domain, and the alternative design — storing only the deltas — makes every one of them
a reconstruction. It also lets the corpus hold figures for versions that predate this
repository's git history, which matters given a window reaching back to FY2010.

Three versions of [HB 96](../bill/hb96-136th.yml) are seeded, chosen to bracket the two
largest sources of change:

- **As introduced** — the executive proposal, before any legislative rewriting.
- **As passed by the House** — after the largest single set of legislative changes.
  [inference]
- **As enacted** — after the Senate, the conference committee, and the governor's line-item
  vetoes.

The gap between the second and third is deliberately left as one hop rather than three. Filling
it in with the Senate-passed and conference versions is the most obvious corpus expansion, and
the one that would let [`stage-delta`](../../skills/) attribute changes precisely rather than
in aggregate.

See the class definition at [`../bill-version.ont.yml`](../bill-version.ont.yml).
