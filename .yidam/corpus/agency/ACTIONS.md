# Actions — agency

## Queries

- All line items whose authority an agency holds, in a given period
- Total appropriated to an agency across funds, and total actually disbursed
- Programs an agency administers, and which line items fund each
- Predecessor and successor chain for an agency across reorganizations

## Transitions

- **Reorganization.** An agency is split from, merged into, or renamed from another. Creates
  a new node with a `succeeds` edge rather than editing the existing one, because the prior
  agency's appropriation history remains attached to it.
- **Code reassignment.** An agency's numeric code changes, which renumbers every line item
  prefixed by it. This is the most common cause of apparent line item discontinuity.

## Calculators

- [`lineage`](../../skills/) — proposes line item succession across reorganizations. Its
  output is a candidate list for human review, never a conclusion.
- [`gap`](../../skills/) — aggregates appropriated against actual at agency level.

## Cautions

- Agency totals are not comparable across a reorganization without resolving lineage first.
  A department that appears to have doubled in size may simply have absorbed another.
- An agency holding a line item is not always the entity that spends the money. Pass-through
  line items are held by a state agency and spent by local recipients.
