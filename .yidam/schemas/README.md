# schemas

Machine-readable contracts for every file shape in this repository, and for every record a
connector must produce before its output may become corpus nodes.

**These files are generated. Do not hand-edit them.** The source of truth is the Rust types
in [`crates/corpus-schema/src/lib.rs`](../../crates/corpus-schema/src/lib.rs); these JSON
Schemas are emitted from them by `mise run schemas`. A hand-maintained second copy drifts,
and a drifted validation schema is worse than none — it fails closed on correct data and
open on incorrect data. `mise run ci` regenerates and fails on any diff.

## corpus/

Shapes of the files that already exist in this repository.

| Schema | Validates |
|---|---|
| `class-definition` | `.yidam/corpus/<class>.ont.yml` |
| `corpus-instance` | `.yidam/corpus/<class>/<instance>.yml` |
| `decision-record` | `.yidam/decisions/<id>.yml` |
| `catalog-entry` | frontmatter of `.yidam/catalog/<slug>.md` |

A schema can only check shape. It cannot check that a relationship an instance asserts is
one its class actually declares — that requires reading two files together, and it is what
[`corpus-validate`](../../crates/corpus-validate/) exists for. Run both.

## extraction/

Contracts for connector output, written **before** the connectors exist. This ordering is
deliberate: defining the target shape first means an extractor has something to be correct
against on its first run, rather than a shape reverse-engineered from whatever the first
document happened to contain.

| Schema | Produced by |
|---|---|
| `lsc-comparison-row` | [`lsc`](../../crates/lsc/) — one appropriation figure at one stage |
| `obm-expenditure-row` | [`obm`](../../crates/obm/) — one disbursement figure |
| `controlling-board-request` | [`controlling-board`](../../crates/controlling-board/) — one execution-phase action |
| `legislature-bill` | [`legislature`](../../crates/legislature/) — bill metadata and stage history |

Two constraints are enforced by construction rather than by convention:

**Money is integer cents.** Every amount field is `int64`, never a float. Binary floating
point cannot represent most decimal cent values exactly, budget figures are summed across
thousands of line items, and the resulting error is silent. `corpus-validate` additionally
rejects any float-valued corpus property whose name looks monetary.

**Provenance is required, not optional.** Every extraction record must carry a catalog slug,
a document reference, and a retrieval date. A value that cannot say where it came from can
never be promoted from `[inference]` to `[verified]`, so a record without provenance is not
a partial record — it is one that could never do its job. The schemas make it unparseable.

## Fixtures

[`.yidam/fixtures/`](../fixtures/) holds committed sample records conforming to these
schemas, so connectors can be developed and tested offline before any network access, and so
the schemas themselves have worked examples rather than only field lists.
