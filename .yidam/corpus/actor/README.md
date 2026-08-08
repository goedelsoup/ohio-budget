# actor

The roles with authority to propose, amend, enact, veto, or transfer appropriations. Actor is
a UFO role, not a kind: it is held contingently, and the same entity holds different ones at
different points in the cycle. The governor is a proposer in February and a vetoer in June.
An agency is a requester during executive preparation and a spender during execution. Keeping
this separate from [`agency`](../agency/) is what lets the corpus say *who did this* without
implying that doing it is part of what they essentially are.

Four roles are seeded, spanning the phases of the cycle:

- **Governor** — proposes the executive budget and holds the line-item veto.
- **House Finance Committee** — the first legislative body to rewrite the executive proposal.
- **Controlling Board** — acts after enactment, which is the phase most analyses ignore.
- **Office of Budget and Management** — prepares the executive budget and issues the revenue
  estimates the whole structure rests on.

**Known gap at genesis.** The `agency →[participates-as]→ actor` edge has no instance. The
agencies that play these roles — Budget and Management above all — are not themselves seeded
as [`agency`](../agency/) nodes, since the three agencies chosen were selected to demonstrate
reorganization rather than role-playing. The edge is declared and correct; it is simply
unpopulated until an agency that plays a named role is seeded.

See the class definition at [`../actor.ont.yml`](../actor.ont.yml).
