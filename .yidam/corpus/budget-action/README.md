# budget-action

The discrete events that change a number. Amendments, line-item vetoes, controlling board
transfers, supplementals, and executive reductions — modeled as their own nodes so that *who
moved this, when, and why they said they moved it* survives in the graph rather than appearing
as an unexplained difference between two versions.

The class carries a `phase` property distinguishing legislative from executive from execution,
and that distinction is the point. Three of the seeded instances fall in three different
phases, which is deliberate: the common mental model of a budget as a thing the legislature
decides and the governor signs accounts for only the first two, while a substantial amount of
money moves in the third.

Three are seeded:

- A **House amendment**, in the legislative phase.
- A **line-item veto**, in the executive phase — the mechanism that makes the enacted bill
  nobody's product in particular.
- A **controlling board transfer**, in the execution phase, after the budget is supposedly
  settled.

**On stated justification.** The `stated_justification` property records what the actor said,
not why they did it. Those are different claims with different evidence, and the corpus keeps
them apart: a stated reason is a fact about the record, an inferred motive is an
`[inference]` tag in the node body.

See the class definition at [`../budget-action.ont.yml`](../budget-action.ont.yml).
