# Actions — bill

## Queries

- All versions of a bill in stage order
- Every appropriation made by a bill, by line item and fund
- Which bills amended a given bill, and what each changed
- Line-item vetoes exercised against a bill, and their net dollar effect

## Transitions

- **Stage progression.** A new version is produced. Creates a
  [`bill-version`](../bill-version/) node with a `supersedes` edge; never edits the prior one.
- **Amendment by a later bill.** A supplemental or mid-biennium act changes appropriations
  made by an earlier bill. Recorded as an `amends` edge between bills, not as an edit.

## Calculators

- [`stage-delta`](../../skills/) — computes how each amount moved across the bill's versions.

## Cautions

- Bill numbers recur across General Assemblies. Always qualify a bill number with its
  assembly.
- The enacted version is not the final word on the money. Supplementals, controlling board
  action, and executive reductions all follow it.
- An operating budget carries substantial non-appropriation policy language. A bill's
  significance is not bounded by its dollar figures.
