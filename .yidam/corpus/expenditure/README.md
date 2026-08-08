# expenditure

Money that actually left the treasury. An event, not a state, and irreducibly distinct from the
[appropriation](../appropriation/) that authorized it.

Separating these two classes is the modeling decision this repository is organized around.
Authority granted and money spent routinely differ, and the gap between them is the answer to
the question the repository exists to ask. Had they been folded into one class with a `measure`
discriminator, that gap would be a filter predicate a reader has to know to apply. As two
classes, it is a traversal that is visible in the graph itself.

One expenditure is seeded, and it is deliberately the one that completes a chain: from the
[personal income tax](../revenue-source/personal-income-tax.yml), into the
[general revenue fund](../fund/general-revenue-fund.yml), through
[foundation funding](../line-item/foundation-funding.yml) under the
[Fair School Funding Plan](../program/fair-school-funding-plan.yml), landing at
[Columbus City School District](../jurisdiction/columbus-city-school-district.yml) in a closed
fiscal period, where it can be set against
[what was appropriated](../appropriation/foundation-funding-fy2024-25-as-enacted.yml).

That one complete chain was worth more at genesis than several disconnected disbursements.

**On basis.** The `basis` property distinguishes in-year `disbursed` figures from
`actual-closed` ones. A gap computed against a provisional figure is provisional, and the
distinction is not cosmetic — in-year reporting and closed books routinely disagree.

See the class definition at [`../expenditure.ont.yml`](../expenditure.ont.yml).
