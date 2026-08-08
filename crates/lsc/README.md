# lsc

**Capability type:** connector. **Status:** implemented end to end, offline.

Parses Legislative Service Commission comparison documents into
[`lsc-comparison-row`](../../.yidam/schemas/extraction/lsc-comparison-row.schema.json) records
— one appropriation figure, for one line item, at one stage, for one fiscal year.

## Feeds

| Class | What it supplies |
|---|---|
| [`appropriation`](../../.yidam/corpus/appropriation.ont.yml) | The amount, per line item, per stage, per period |
| [`bill-version`](../../.yidam/corpus/bill-version.ont.yml) | Document references fixing each stage |
| [`line-item`](../../.yidam/corpus/line-item.ont.yml) | Codes, titles, agency and fund assignment |

## From PDF to typed record

The whole path is here. `pdf::extract_pages` reads positioned glyphs out of a document;
`geometry::to_table` reconstructs the table; the rest of the crate turns that into typed
records. A PDF and a hand-written delimited file both produce a `RawTable`, so they are
indistinguishable to everything downstream.

### Columns are found by whitespace, not by alignment

A PDF stores characters at coordinates, not cells, so columns have to be recovered. The
obvious approach — cluster cells by where they start — fails on precisely the columns that
matter here: **currency in budget tables is right-aligned**, so `$12.00` and
`$812,345,678.00` in the same column begin far apart and end together. Clustering on start
splits that column; clustering on end breaks the text columns instead.

So columns are found as vertical **rivers**: bands of x where no glyph appears on any row.
That looks for the gap *between* columns rather than the edge of any one, and handles left,
right, and centre alignment without being told which is which. `right_aligned_currency_stays_one_column`
is the regression test for it.

### The `pdf` feature

Reading PDF bytes pulls the font and CMap stack, so it sits behind a feature that is on by
default. `--no-default-features` still builds and tests the geometry, which is the part with
the interesting failure modes and needs no document to exercise.

## Money

`parse_money_to_cents` is the most safety-critical function in this repository. Budget figures
are summed across thousands of line items, so a parser that silently rounds produces totals
wrong by amounts nothing downstream can detect.

It accepts the published forms — `$1,234,567.89`, `(1,234.56)` and `-1,234.56` for negatives —
and refuses three things rather than approximating:

- **More than two decimal places.** Cents cannot represent the value exactly, and rounding here
  would be invisible.
- **Empty cells.** An absent figure is not zero. Callers get an explicit blank outcome.
- **Anything non-numeric.** `n/a` is reported, never coerced.

## Nothing is dropped

`normalize` returns one outcome per cell that should have held a figure: a parsed row, a
recorded blank, or a recorded parse failure with its reason. An extraction run can therefore
report its own coverage, rather than returning fewer records than the document contained and
leaving nobody to notice.

A row whose cell count disagrees with the header is an error, not a truncation — a misaligned
row in a budget table puts figures in the wrong fiscal year.

## Open

- [open] Whether table structure is stable enough across biennia for one column map, or whether
  each year's documents need their own.
