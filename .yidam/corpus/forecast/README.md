# forecast

Revenue estimates and caseload projections. A budget is built on projections rather than on
receipts, and the distance between a forecast and what actually arrived is the evidence base
for any claim about structural balance.

The class earns its place through a specific asymmetry. Every
[appropriation](../appropriation/) in the corpus was sized against a revenue estimate, so an
error in the estimate propagates into every figure downstream. A corpus that recorded only the
appropriations would show the consequences of forecast error as unexplained mid-year
adjustments, which is precisely how it appears in most public accounts of a budget.

One forecast is seeded — a revenue estimate. The obvious gap is a **caseload projection**, which
is what would populate the approved `budget-action →[triggered-by]→ forecast` edge: Medicaid
adjustments follow enrollment revisions, not revenue revisions, and the corpus currently has the
action but not the trigger. Seeding one is the single highest-value addition to this class.

**On variance.** The `variance_from_actual` property is only meaningful once a period has
closed, and it is the property that makes forecasts retrospectively informative rather than
merely historical. A forecast whose variance is never recorded is a document; one whose variance
is recorded is evidence about the forecaster.

See the class definition at [`../forecast.ont.yml`](../forecast.ont.yml).
