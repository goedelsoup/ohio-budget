# jurisdiction

Local entities that receive state money. The class exists because a large share of the state
budget is not spent by the state — it passes through state accounts to counties,
municipalities, school districts, and special districts that the state does not control. An
analysis that stops at the state agency holding the line item stops one step short of where
the money went.

One jurisdiction is seeded at genesis, deliberately. Jurisdiction was included in the
confirmed ontology because the local pass-through framing was in scope, but the
[`population`](../) class that would let it support distributional analysis was deferred, and
so was the `levies` edge that would connect local taxing capacity to state formulas. Seeding
many jurisdictions now would produce exactly the thin, speculative nodes the prelude warns
against. One well-connected instance demonstrates the structure; the rest arrive when the
`district-finance` connector can populate them with real figures.

**This class is a sink.** Its declared edges are all inbound — expenditures land on it and
programs serve it. Its instances therefore carry no outgoing domain edges beyond `instance-of`,
which is correct rather than a defect.

See the class definition at [`../jurisdiction.ont.yml`](../jurisdiction.ont.yml).
