# lsc

**Capability type:** connector. **Status:** implemented downstream of PDF extraction.

Parses Legislative Service Commission comparison documents into
[`lsc-comparison-row`](../../.yidam/schemas/extraction/lsc-comparison-row.schema.json) records
— one appropriation figure, for one line item, at one stage, for one fiscal year.

## Feeds

| Class | What it supplies |
|---|---|
| [`appropriation`](../../.yidam/corpus/appropriation.ont.yml) | The amount, per line item, per stage, per period |
| [`bill-version`](../../.yidam/corpus/bill-version.ont.yml) | Document references fixing each stage |
| [`line-item`](../../.yidam/corpus/line-item.ont.yml) | Codes, titles, agency and fund assignment |

## The division of labour

The documents are PDFs. PDF text extraction is **not** done here — it belongs in a Python
package where the ecosystem is mature, and its output is delimited text. Everything downstream
of that is implemented and tested here, because that is where the errors that matter occur.

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
