# docs

This repository documents the Ohio state budget as a connected system — how revenue
instruments feed statutory funds, how funds are committed through appropriation line
items, how those line items are moved by legislative and executive action, and how the
money finally lands with state agencies and local jurisdictions. It covers the full
appropriation surface (main operating, transportation, capital, workers' compensation
and industrial commission budgets, plus execution-level changes) across FY2010 to the
present. Written for researchers, journalists, agents, and contributors working in the
corpus.

This directory holds documentation about this repository: its purpose, scope,
domain conventions, and any decisions that shaped its structure.

It is distinct from the corpus (which holds knowledge claims) and the prelude
(which holds yidam's own model). Documentation here describes the *repository*,
not the domain.

## Reading order

1. [`.yidam/decisions/ontology.yml`](../.yidam/decisions/ontology.yml) — why the corpus
   has the fourteen classes it has, what was considered and discarded, and why the classes
   are aligned to UFO rather than BFO.
2. [`.yidam/corpus/`](../.yidam/corpus/) — the class definitions (`*.ont.yml`) and the
   instances that populate them.
3. [`.yidam/catalog/`](../.yidam/catalog/) — provenance anchors for every claim that
   carries a `[verified]` tag.

## A note on the five numbers

The single most common error in reading state budget data is treating one line item as
having one number. It has at least five: *appropriated* (what the enacted bill grants),
*allocated* (what the agency is cleared to plan against), *encumbered* (what is
contractually committed), *disbursed* (what has actually left the treasury), and
*actual* (what the closed books report). This corpus models the first and the fourth as
separate classes — [`appropriation`](../.yidam/corpus/appropriation.ont.yml) and
[`expenditure`](../.yidam/corpus/expenditure.ont.yml) — precisely so the gap between
them is a traversal rather than a footnote.
