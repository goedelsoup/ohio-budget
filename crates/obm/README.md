# obm

**Capability type:** connector. **Status:** built and reading nothing — **the source is right and
the text layer cannot read it.** See [The document does not decode](#the-document-does-not-decode).

Retrieves Office of Budget and Management monthly financial reports and the state's audited
annual financial report.

## Feeds

| Class | What it supplies |
|---|---|
| [`expenditure`](../../.yidam/corpus/expenditure.ont.yml) | Disbursed and actual-closed figures by line item and period |
| [`fund`](../../.yidam/corpus/fund.ont.yml) | Balances and transfers |
| [`forecast`](../../.yidam/corpus/forecast.ont.yml) | Revenue estimates and their revisions |

## Why it is ordered third

It is the only authoritative source for actual disbursement and therefore a hard prerequisite
for [`gap`](../../.yidam/skills/gap.md), the repository's central calculation. It is ordered
after [`lsc`](../lsc/) only because a gap needs both sides — actuals with nothing to compare
against answer no question.

## Basis handling

The connector must preserve the distinction between in-year `disbursed` figures and
`actual-closed` figures rather than normalizing them. They routinely disagree, and a gap
computed across mixed bases is not a gap.

## Interface sketch

```
fetch_monthly(period: FiscalPeriod, month: u8) -> Result<Vec<ExpenditureRow>>
fetch_annual(period: FiscalPeriod) -> Result<AnnualReport>
fetch_revenue_estimate(period: FiscalPeriod) -> Result<Vec<Forecast>>
```

## Open questions

- [open] Whether monthly reporting is available at line item granularity or only at fund and
  agency level. If the latter, the gap calculator cannot operate at the granularity the corpus
  models, and that is a material constraint on the whole design.

## Scouted 2026-08-09

Unlike the [controlling board](../controlling-board/README.md), which cannot be enumerated, and
the [checkbook](../checkbook/README.md), which guards against automation, this source is fully
reachable:

    archives.obm.ohio.gov/Files/Budget_and_Planning/Monthly_Financial_Report/YYYY-MM-mfr.pdf

Reports are enumerable by date, roughly 1.5 MB each, and the text extracts cleanly. Separators
are inconsistent — `2022-12_mfr.pdf` and `2022_11-mfr.pdf` both occur — so a fetcher needs to try
both.

**What it cannot do is the thing the corpus wanted.** Disbursements are reported by spending
category and by fund, never by appropriation line item: the January 2026 report contains zero
occurrences of any ALI this corpus models. [verified] The catalog entry called that question
["the single largest unresolved risk to the repository's central
question"](../../.yidam/catalog/obm-annual-report.md) and it is answered against.

So a connector here would feed `fund` and `forecast` usefully and would not feed `expenditure`
at the granularity `gap` operates on. That is worth writing and is not what it was queued for.

[open] The audited **annual** report is a different document and was not scouted. If it carries
line-item detail where the monthly reports do not, it would close FY2025 two years before LSC
does — which is the one thing that would change what this corpus can answer about an open year.

## Scouted again 2026-08-09 — the line item detail exists, in a third document

The open question above asked whether some other OBM publication carries what the monthly
reports do not. It does, and it is neither the monthly report nor the audited annual report:

    archives.obm.ohio.gov/Files/State_Accounting/Financial_Reporting/
      Detailed_Appropriation_Summary_by_Fund/DASF_FY-YYYY.pdf

**Detailed Appropriation Summary by Fund**, subtitled *per the Ohio Administrative Knowledge
System (OAKS)*, as of 30 June. One row per **(agency, fund, appropriation line item)**, with four
money columns: [verified]

    BUDGET ORIGINAL   BUDGET FINAL   ACTUAL   VARIANCE WITH FINAL BUDGET

**All sixteen fiscal years resolve, FY2010 through FY2025** — the corpus's entire declared
window, including the FY2010-11 biennium nothing else here reaches. [verified] Probe by
content-type, not status: this host soft-404s with `200`, exactly as the
[controlling board](../controlling-board/README.md) scout found. Naming is inconsistent —
`DASF_FY-2021.pdf`, `DASF%20_FY-22.pdf`, and `DASF%20_FY-2023.pdf` all occur.

**It carries `BUDGET FINAL` for a closed year, which the corpus has said no committed source
does.** [verified] That figure is the appropriation as it stood at year end, after transfers and
reductions, and its absence is what
[what the gap actually says](../../.yidam/decisions/what-the-gap-actually-says.yml) records as
making the encumbrance subtraction "defined and not computable". It also reaches the
execution-phase question from the other side: the closed-year adjusted figure is the controlling
board's work totalled up, so some of what that blocked connector was wanted for arrives without
enumerating a single agenda.

### One line item agrees to the cent, and three do not

Checked against the corpus for FY2021, the year whose actuals this repository has just finished
correcting: [verified]

| ALI | corpus appropriated / actual | DASF original / final / actual |
|---|---|---|
| 110969 local government | 424,900,000.00 / 451,474,951.44 | 424,900,000.00 / 451,559,619.78 / **451,474,951.44** |
| 200550 foundation funding | 6,774,618,845.00 / 6,703,103,300.93 | 6,804,218,890.74 / 6,829,055,557.74 / 6,760,597,235.56 |
| 501321 institutional operations | 1,167,132,362.00 / 1,105,832,410.92 | 1,184,170,422.91 / 1,184,170,422.91 / 1,167,527,704.52 |
| 651525 medicaid | 15,886,271,485.00 / 16,897,026,748.47 | 15,902,819,670.27 / 16,933,063,212.27 / 16,913,417,988.72 |

Local government matches **exactly on both sides** — an independent publisher, a different
system of record, and the same cents. That is the strongest confirmation the corpus's LSC
extraction has ever had.

The other three do not, by tens of millions — far past rounding. Both plausible explanations are
[open] and the connector's first job is to decide between them: DASF splits by **fund** and shows
one row per fund, so a corpus figure spanning funds would not equal any single row; and `BUDGET
ORIGINAL` is what OAKS was loaded with, which need not be the enacted figure if reappropriations
or carried encumbrances are included.

**One near-coincidence is a trap worth naming.** Institutional operations' corpus *appropriation*
(1,167,132,362.00) sits within $400k of DASF's *actual* (1,167,527,704.52). Nothing follows from
that, and a reconciliation that matched on proximity rather than on column would read it as
agreement.

### What this changes

It makes the whole of [`gap`](../../.yidam/skills/gap.md) checkable against a second publisher at
line item granularity for every year the corpus covers, and it is the natural way to test the
[inference] that LSC reports on a budgetary basis — compare an LSC actual against this one for
the same ALI and year, which the table above starts and does not finish.

## The document does not decode

The crate is built, tested, and returns nothing from the real file. `pdf-extract` — the stack
[`lsc`](../lsc/) reads PDFs with, and this connector borrows — decodes these documents wrongly,
and wrongly in the worst available way. Measured on page 3 of the FY2021 report against
poppler's `pdftotext` on the same page: [verified]

| | poppler | pdf-extract |
|---|---|---|
| `0` `1` `2` `3` | 570 · 209 · 194 · 147 | 568 · 208 · 191 · 147 |
| `4` `5` `7` `8` `9` | 195 · 186 · 134 · 145 · 159 | **0 · 0 · 0 · 0 · 0** |
| `6` | 148 | 1 |
| `.` | 204 | **0** |
| `,` | 316 | 315 |

**Digits 0 to 3 and the thousands separator survive; 4 to 9 and the decimal point do not.** So
`4,000.00` arrives as `,00000` — still comma-separated, still digits, still shaped exactly like
money. Nothing downstream could detect it. A connector that trusted the text would have written
figures wrong by orders of magnitude into a corpus whose entire discipline is that a figure is
either sourced or marked `[open]`.

So the extractor refuses instead. `dasf::verify_decode` rejects any page missing a decimal point
or a whole digit before the parser sees it, on the argument that a table of money cannot be
missing one; against the real FY2021 file it refuses all 54 pages and reads nothing. [verified]
That is the same rule `lsc` applies to a misaligned row, moved one step earlier: **an error, not
a number.**

The cause is not the geometry. Column reconstruction was never reached, and is untested against
this document for that reason — with two fifths of the glyphs missing, the whitespace rivers the
algorithm looks for are not the document's.

### What would unblock it, in order of preference

1. **A text layer that decodes these fonts.** poppler reads the file perfectly, so nothing about
   the document is unreadable in principle. Shelling out to `pdftotext` would make a Rust-only,
   self-contained repository depend on a system binary being installed, which is a real cost and
   not this crate's decision to make alone.
2. **A structured publication.** [open] LSC publishes its appropriation spreadsheets as xlsx as
   well as PDF, and [the xlsx is the better source](../lsc/) for exactly this reason. Whether OBM
   does the same is unknown: guessed `.xlsx`, `.xls`, and `.csv` paths all soft-404, and the two
   OBM portal index pages return 404 to a plain fetch — the same wall the
   [controlling board](../controlling-board/README.md) scout hit. A records request would settle
   it, and OBM holds this in OAKS in structured form by definition.
3. **A different PDF crate.** The font dictionaries sit inside object streams, so the failure is
   in font decoding rather than in the page content. Not diagnosed further.

Until one of those lands, this connector is a specification with a test suite: the row parsing,
the cents arithmetic, and the reconciliation check are exercised against synthetic tables in
[`dasf.rs`](src/dasf.rs) and against [`.yidam/fixtures/obm/dasf/`](../../.yidam/fixtures/obm/dasf/),
so the day the text layer works there is nothing left to write.

## On depending on lsc

`obm` depends on `lsc` for the PDF-to-table step alone, and that is an odd-looking edge: one
connector should not need another. The algorithm it borrows —
[`lsc::geometry`](../lsc/src/geometry.rs) — is not about LSC. It recovers a table from positioned
glyphs, which is a PDF problem, and it lives where it does because LSC was the only source that
needed it when it was written.

The alternative was copying five hundred tested lines to avoid the appearance of a bad
dependency, which trades a real property for a cosmetic one. [open] If a third source needs it,
the module should move to its own crate and both connectors should depend on that instead.
