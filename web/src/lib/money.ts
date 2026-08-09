/**
 * Formats money. It never parses money.
 *
 * Every figure arrives from the feed already in whole cents, parsed on the Rust side by
 * `lsc::parse_money_to_cents`. A second parser here would be a second set of rules about
 * what `$8,457,598,772.00` and `[open] pending the lsc connector` mean, and the two would
 * disagree at exactly the inputs that matter.
 */

const DOLLARS = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 0,
});

const EXACT = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

/** `$8,457,598,772` — the reading figure. */
export function dollars(cents: number): string {
  return DOLLARS.format(cents / 100);
}

/** `$8,457,598,772.00` — for anywhere the cents are themselves the point. */
export function exact(cents: number): string {
  return EXACT.format(cents / 100);
}

/** `+$92.25M` / `−$72.89M`. Signed, because a delta's direction is half of what it says. */
export function delta(cents: number): string {
  if (cents === 0) return '$0';
  // U+2212 minus, not a hyphen: these sit next to figures, and a hyphen reads as a dash.
  return `${cents > 0 ? '+' : '−'}${compact(Math.abs(cents))}`;
}

/** `$8.46B` — for axis ticks and dense tables, where full figures crowd out the shape. */
export function compact(cents: number): string {
  const d = Math.abs(cents) / 100;
  const sign = cents < 0 ? '−' : '';
  if (d >= 1e9) return `${sign}$${(d / 1e9).toFixed(2)}B`;
  if (d >= 1e6) return `${sign}$${(d / 1e6).toFixed(1)}M`;
  if (d >= 1e3) return `${sign}$${(d / 1e3).toFixed(0)}K`;
  return `${sign}$${d.toFixed(0)}`;
}

/** `0.10%`, signed. Percentages here are small and the sign carries the meaning. */
export function percent(pct: number, digits = 2): string {
  const sign = pct > 0 ? '+' : pct < 0 ? '−' : '';
  return `${sign}${Math.abs(pct).toFixed(digits)}%`;
}
