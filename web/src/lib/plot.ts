/**
 * Renders Observable Plot figures to static SVG at build time.
 *
 * Plot needs a DOM, so one jsdom document is created for the whole build and reused. The
 * output is serialized markup, which means the pages ship no client JavaScript: a reader
 * with scripting off, or a slow connection, still sees the figures.
 *
 * # The palette
 *
 * Drawn from the vendored design system's ramps and validated for colour-vision deficiency
 * rather than chosen by eye. `rigpa-600` against `gold-600` separates at ΔE 26 under protan
 * simulation; the obvious green-against-gold pairing separates at 4.9 and was rejected. Ink
 * is the deliberate neutral for absence — it is a near-gray on purpose, and every chart using
 * it also carries labels and counts so nothing is encoded by colour alone.
 */

import * as Plot from '@observablehq/plot';
import { JSDOM } from 'jsdom';

const { document } = new JSDOM('').window;

export const PALETTE = {
  /** Money added. Gold is the system's accent for earned value. */
  increase: '#9c7800',
  /** Money removed. */
  decrease: '#2254a4',
  /** No movement, or a figure the corpus does not hold. Near-gray by intent. */
  neutral: '#b9b0a1',
  /** Single-series magnitude, where the bars carry no identity of their own. */
  single: '#5a5449',
  ink: '#413d36',
  grid: '#d5cfc4',
  text: '#5a5449',
  surface: '#faf8f4',
} as const;

/** Applied to every figure, so the charts read as one family. */
const BASE: Plot.PlotOptions = {
  style: {
    background: 'transparent',
    color: PALETTE.text,
    fontFamily: 'DM Sans, system-ui, sans-serif',
    fontSize: '12px',
    overflow: 'visible',
  },
  marginLeft: 64,
  marginBottom: 40,
};

/**
 * Renders a plot to an SVG string.
 *
 * Plot returns a bare `<svg>` or, once a legend or caption is involved, a `<figure>`
 * wrapping one. Both serialize the same way.
 */
export function renderPlot(options: Plot.PlotOptions): string {
  const node = Plot.plot({ ...BASE, ...options, document });
  return (node as unknown as Element).outerHTML;
}

/** A horizontal rule at zero, so a diverging bar's baseline is explicit. */
export function zeroRule(y = false) {
  return y
    ? Plot.ruleY([0], { stroke: PALETTE.ink, strokeWidth: 1 })
    : Plot.ruleX([0], { stroke: PALETTE.ink, strokeWidth: 1 });
}
