# web

Web interface layer for this repository — a static site that renders the corpus, the
catalog, and the calculators' results.

## Why it exists now

The [conventions](../.yidam/.vendor/prelude/guidelines/directories.md#web) say a web layer
is added only when direct programmatic access is insufficient. The trigger recorded here at
genesis was that this domain's questions — *what happened to funding for this program*,
*what changed between the governor's proposal and the enacted bill* — are traversals of the
graph rather than searches over it, and that a web layer earns its place once the corpus
carries enough line item lineage to answer one of them.

That point arrived with the HB 96 extraction. Foundation Funding now carries a figure at
every one of the nine stages between introduction and enactment, which makes stage-by-stage
attribution computable, and FY2024 carries both an enacted appropriation and a closed-book
actual, which makes one gap computable.

## What it renders, and what it does not compute

Every figure on the site is produced by a calculator in [`crates/`](../crates/), exported as
JSON, and rendered. The site formats money and draws figures; it never decides whether two
numbers may be subtracted.

That division is deliberate. The arithmetic is the easy half — the judgments are in the
[skills](../.yidam/skills/): that an `[open]` amount is absence rather than zero, that a
recipient's share may not be measured against a whole line item's authority, that
expenditures on different bases may not be summed, that no cross-period comparison is valid
without a deflator. Reimplementing those in TypeScript would create a second set of rules,
and the two would drift toward whichever was easier to write — always the web copy, because
it is the one under pressure to render something.

So the feed carries verdicts, including refusals, and the site renders a refusal where a
figure would otherwise go. `.yidam/skills/real-dollars.md` names the failure this prevents:
*a chart mixing nominal and real points looks fine*.

## The feed

[`crates/corpus-export`](../crates/corpus-export/) writes `src/data/*.json`:

| File | Contents |
|---|---|
| `manifest.json` | contract version, HEAD commit, counts, whether a deflator is available |
| `corpus.json` | class definitions and every node, with resolved links and computed backlinks |
| `catalog.json` | source registry, including entries that fail to parse |
| `decisions.json` | decision records |
| `skills.json` | the calculators |
| `findings.json` | `gap` coverage and `stage-delta` decompositions |

The feed is **generated and gitignored**. Committing it would put a second copy of the
corpus in the repository that could disagree with the first — the failure
[claims-about-repository-state](../.yidam/decisions/claims-about-repository-state.yml) is
about. `src/lib/feed.ts` fails the build with a pointer to `mise run export` when it is
missing, and refuses a feed whose `contract_version` differs from the one
`src/lib/types.ts` was written against.

## Routes

| Route | Contents |
|---|---|
| `/` | the findings, in prose, with figures inline |
| `/findings/hb96-stage-movement` | the full stage decomposition and what was said about each step |
| `/findings/gap-coverage` | every line item and period pair, and what blocks the ones that are blocked |
| `/wiki` | the corpus — class sidebar, subject landings, open questions, unresolved edges |
| `/wiki/<class>` | a class definition, its declared edges, and its instances |
| `/wiki/<class>/<slug>` | one node: body, properties, edges, backlinks, and a figure where the data supports one |
| `/wiki/subjects/<slug>` | a line item assembled with everything attached to it |
| `/sources` · `/sources/<slug>` | the catalog, and which nodes cite each entry |
| `/methods` | the calculators and the decision records |

## Working on it

```
mise run web-install   # once, per clone
mise run web-dev       # exports the feed, then serves with live reload
mise run web-test      # vitest over the derivation layer
mise run web-check     # astro check — TypeScript across .astro and .ts
mise run web-build     # exports the feed, then builds to web/dist
```

`mise run ci` runs the last three.

Tests cover `src/lib/` — link resolution, body rendering, money formatting, and the
predicates that decide whether a figure may be drawn. The pages are a thin rendering of
that layer, and the facts themselves are already gated by the Rust side's tests.

## Stack

TypeScript in strict mode, Astro with static output, Vitest, and Observable Plot. Plot
renders to SVG at build time through jsdom, so the pages ship no client JavaScript.

Design tokens are imported directly from the vendored
[yidam design system](../.yidam/.vendor/design/) rather than copied, so a change there
reaches the site without a re-copy. That system defines parchment surfaces only and ships no
dark tokens; the site commits to that single look rather than inventing a dark theme it has
not vetted.

Chart colours are drawn from the system's ramps and validated for colour-vision deficiency
rather than chosen by eye: `rigpa-600` against `gold-600` separates at ΔE 26 under protan
simulation, where the obvious green-against-gold pairing separates at 4.9 and was rejected.
