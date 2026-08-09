# checkbook

**Capability type:** connector. **Status:** stub — **the publisher guards against automation.**

Retrieves state transaction-level transparency reporting.

## Feeds

| Class | What it supplies |
|---|---|
| [`expenditure`](../../.yidam/corpus/expenditure.ont.yml) | Aggregated disbursement detail |
| [`jurisdiction`](../../.yidam/corpus/jurisdiction.ont.yml) | Recipient identification |

## Aggregates, not nodes

Transaction granularity was explicitly ruled out of the corpus during ontology discovery —
individual payments are a data lake, not a knowledge graph. This connector therefore
**aggregates before returning**: its output is a disbursement total per line item, period, and
recipient, never one node per transaction.

That constraint is the whole reason this is a connector rather than a corpus import. Violating
it would flood the graph with nodes that carry no conceptual content.

## Interface sketch

```
fetch_aggregate(line_item: &str, period: FiscalPeriod) -> Result<Vec<RecipientTotal>>
```

## Open questions

- [open] Whether transparency reporting is reconcilable to [`obm`](../obm/) figures. If the two
  disagree, which is expected at period boundaries because of cash versus accrual treatment,
  the corpus needs a stated rule for which is authoritative.

## Why it is still a stub

Scouted 2026-08-09, on the same discipline as
[controlling-board](../controlling-board/README.md): an unexplained stub is worse than a
documented obstacle, and three open questions point here — recipient-level disbursement, the four
[jurisdictions](../../.yidam/corpus/jurisdiction/) nothing is recorded as paid to, and whether an
apparent underspend is money
[encumbered rather than unused](../../.yidam/corpus/expenditure.ont.yml).

What is there: [verified]

    checkbook.ohio.gov                        live, 68 kB of ASP.NET and Tableau
    /Scripts/TableauVisualsDefaults.js        the figures are rendered by Tableau, not served
    google.com/recaptcha/enterprise.js        loaded on the home page
    95 links, none offering download, export, CSV or a dataset
    data.ohio.gov/.../view/ohio-checkbook     HTTP 404 — its own open-data link is dead

**The obstacle is not technical.** A Tableau-backed page can be driven, and a captcha can be
worked around. reCAPTCHA Enterprise on a public transparency site is the publisher stating that
it does not want automated retrieval, and this repository does not scrape sources that say so.
The corpus's rule for reading a document — take what is published, cite it, and record what you
could not get — does not extend to defeating an access control because the data underneath is
public.

**What would unblock it**, in order of preference: a bulk export or API that Ohio publishes and
this scout did not find; a public records request, since the data exists in structured form
behind the visualisations; or OBM's annual financial report, which is already
[catalogued](../../.yidam/catalog/obm-annual-report.md) and carries disbursement detail at lower
granularity without any of this.

[open] The third route is the cheapest and is unattempted. It would not reach recipient level,
which is the one thing only this source has.
