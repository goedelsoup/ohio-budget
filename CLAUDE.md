# Working in this repository

This is a yidam-derived knowledge repository, not an application. The corpus of nodes lives in
`.yidam/corpus/`, the calculators that read it live in `crates/`, and the site that renders their
output lives in `web/`. Before adding or editing nodes, read
[`.yidam/.vendor/prelude/guidelines/agent-conduct.md`](.yidam/.vendor/prelude/guidelines/agent-conduct.md) —
in particular the claim-confidence tags (`[verified]`, `[inference]`, `[open]`), which are load-bearing
here rather than decorative.

`.yidam/.vendor/` is vendored template infrastructure, replaced wholesale the next time
`yidam overlay` runs. Do not edit anything under it; changes there are lost, not merged.
[`.gitattributes`](.gitattributes) marks it, and `.yidam/sources/`, as vendored: neither counts
toward the repository's language statistics, and both arrive collapsed in a pull request diff.
That is the intent — but it also means an accidental edit under `.vendor/` is folded shut in the
one place anyone would see it, so the rule against editing there is worth more than it looks.

## Before opening a pull request

Run the gate and get it green:

```
mise run yidam-build   # once, per clone — `mise run ci` calls the `yidam` binary by name
mise run ci
```

This is not advice to run CI's equivalent. [`.github/workflows/ci.yml`](.github/workflows/ci.yml)
installs the toolchain and then runs `mise run ci`, the same command from the same definition in
[`mise.toml`](mise.toml). A failure the workflow reports is a failure that was already visible
locally, and the round trip through a pushed branch buys nothing.

Adding a check means editing `[tasks.ci]` in `mise.toml` and nothing in the workflow. Do not add a
step to the workflow that restates a check: two descriptions of one gate drifting apart while both
look right in isolation is the failure this repository keeps having, and it is the reason the
workflow is as thin as it is.

There is no commit-time hook. Nothing runs on your behalf between editing and pushing, so running
the gate is a step you have to take rather than one you can rely on being taken for you.

## While iterating

`mise run ci` compiles the workspace, runs every test, and builds the site. That is the right cost
once per pull request and the wrong cost per edit. While working, run the narrower task that covers
what you touched:

| Touched | Run |
|---|---|
| `.yidam/corpus/`, `.yidam/catalog/`, `.yidam/decisions/`, `crates/corpus-*` | `mise run validate` |
| Node relationships, links between nodes | `yidam graph-check` |
| `crates/` | `mise run test` |
| `web/` | `mise run web-test`, `mise run web-check` |

`mise run validate` is the one to reach for by default when writing prose into a node. It checks
the corpus rules alone — no compile, no tests — and catches the failure this repository keeps
having: a node asserting something about the repository that stopped being true. A source described
as uncatalogued after it was catalogued; an amount described as unfilled after it was filled. Both
halves of such a contradiction look correct in isolation, and the stale half is usually written in
the same commit that falsifies it, which is why reading the diff does not catch it. See
[claims-about-repository-state](.yidam/decisions/claims-about-repository-state.yml) for the
convention that makes such claims checkable — a claim that a source is not catalogued names the
slug it means.

## Generated files

`web/src/data/*.json` is emitted by `mise run export` and gitignored. It is a second copy of the
corpus that can disagree with the corpus, so it is never committed. `.yidam/schemas/` is emitted by
`mise run schemas` and *is* committed; the gate regenerates it and fails on a diff, so regenerate
and commit it in the same change as the Rust types it comes from.
