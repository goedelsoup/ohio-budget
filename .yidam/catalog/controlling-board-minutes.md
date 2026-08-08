---
slug: controlling-board-minutes
name: Ohio Controlling Board agendas, requests, and minutes
source_type: minutes
publisher: Ohio Controlling Board
location: Controlling Board meeting agendas and minutes
content_committed: false
feeds:
  - budget-action
---

# Controlling Board minutes

Agendas, agency requests, and minutes recording what the Controlling Board approved, denied,
tabled, or saw withdrawn after a budget was enacted.

## The only window onto the execution phase

This is the sole source for a whole phase of the budget cycle. The corpus models three phases
— legislative, executive, execution — and the third is invisible without these documents. An
analysis that stops at the enacted bill treats the budget as a decision, when it is closer to
a decision followed by two years of adjustments.

## Disposition is not optional

Requests that were denied or withdrawn moved no money but are still part of the record, and
still evidence about what an agency asked for and when. Extraction must carry disposition
alongside the amount; a parser keying only on the dollar figure would overstate
execution-phase movement, which is why
[`controlling-board-request`](../schemas/extraction/controlling-board-request.schema.json)
makes `disposition` required and the fixtures include a withdrawn case.

## The number that settles a corpus assumption

[open] The aggregate dollar volume passing through Controlling Board action in a typical
biennium is unknown. The corpus currently assumes this mechanism is material — that
assumption is stated in [`controlling-board`](../corpus/actor/controlling-board.yml) and is
untested. This source settles it, and should be asked that question first.

## Extraction

Feeds via the [`controlling-board`](../../crates/controlling-board/) connector.
