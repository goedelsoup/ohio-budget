# Actions — fund

## Queries

- Which revenue instruments deposit into a fund, and in what proportions
- All line items chargeable against a fund
- Total appropriated from a fund in a period, against total deposited into it
- Transfers into and out of a fund, which are invisible to appropriation-based queries

## Transitions

- **Transfer.** Money moves between funds without an appropriation. Recorded as a
  `transfers-to` edge and, where a controlling board or executive action authorized it, as a
  [`budget-action`](../budget-action/).
- **Reversion.** Unspent authority lapses back at period end, restoring fund balance without
  any expenditure having occurred.

## Calculators

- [`structural-balance`](../../skills/) — reads fund balances to distinguish recurring
  capacity from accumulated one-time money.

## Cautions

- Fund balance is not spendable capacity. A fund can hold a large balance that is
  restricted, encumbered, or committed to a future obligation.
- Appropriating from a fund does not require money to be in it. Appropriation is authority,
  not cash, and the two diverge routinely.
- Do not sum across fund groups without saying so. Adding general revenue to federal funds
  produces an all-funds figure that is correct but answers a different question than most
  readers think they asked.
