# agency

The organizational units of state government that hold appropriation authority and spend it.
An agency is a kind, not a role: an entity that ceases to be an agency ceases to exist as
that thing, whereas the roles it plays in the budget process — requester during executive
preparation, spender during execution — vary by phase. That distinction is why
[`actor`](../actor/) is a separate class.

Three agencies are seeded, and two of them exist to demonstrate that agency identity is not
stable across the corpus window:

- **Department of Medicaid** was separated from Job and Family Services partway through the
  window, taking its line items with it.
- **Department of Education and Workforce** replaced a differently-governed predecessor,
  again mid-window.
- **Department of Transportation** is the stable case, and the one whose money comes from a
  different budget bill entirely.

These reorganizations are the reason the approved `agency →[succeeds]→ agency` edge exists.
Without it, a sixteen-year series for any Medicaid or education line item breaks at the
reorganization with no way to express that the funding continued under a new owner.

See the class definition at [`../agency.ont.yml`](../agency.ont.yml).
