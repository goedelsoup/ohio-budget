# Actions — bill-version

## Queries

- Every appropriation figure stated in a version
- The same line item across all versions of a bill, in stage order
- Which actions were applied to a version, and by which actor
- Which version is currently operative

## Transitions

- **Supersession.** A later version replaces an earlier one as operative. The earlier node is
  never edited — it remains the record of what the figures were at that stage.

## Calculators

- [`stage-delta`](../../skills/) — differences a line item across consecutive versions and
  attributes each change to an action.

## Cautions

- A missing intermediate version does not mean nothing happened between two stages. Where the
  chain is incomplete, `stage-delta` reports aggregate movement and cannot attribute it.
- The as-enacted version already reflects line-item vetoes. Comparing as-passed to as-enacted
  therefore conflates legislative and executive change unless the veto actions are read
  separately.
