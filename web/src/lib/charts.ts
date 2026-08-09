/**
 * The figures. Each returns serialized SVG; none of them decides whether it should exist —
 * that judgment lives in `derive.ts`, and a page asks there first.
 *
 * Conventions held across all of them, from the design system and the dataviz checks:
 *
 * - **One hue for one series.** A value-ramp across nominal categories would burn the only
 *   free channel re-encoding bar length as colour.
 * - **Two hues only where direction is the subject**, with a neutral zero — gold for money
 *   added, rigpa blue for money removed, validated at ΔE 26 under protan simulation.
 * - **Labels are selective.** The extremes and the endpoint carry values; the rest are read
 *   from the axis, the native SVG `<title>` on hover, or the table every figure ships with.
 * - **Hairline solid grid**, never dashed — a dashed rule reads as a threshold.
 */

import * as Plot from '@observablehq/plot';
import type { StagePoint } from './derive.ts';
import { stageLabel } from './derive.ts';
import { compact, delta as fmtDelta, dollars } from './money.ts';
import { PALETTE, renderPlot } from './plot.ts';
import type { GapCoverage, GapResult, ManifestCounts } from './types.ts';

const AXIS = { stroke: PALETTE.grid, strokeOpacity: 1 } as const;

/**
 * Where a figure's money is measured from.
 *
 * A level chart over stages cannot start at zero — the whole series sits within about one
 * percent of itself, and a zero baseline would render it as a flat line. Non-zero is
 * legitimate for a line, which encodes position rather than area, but it must be said out
 * loud, so every caller of `stageLevels` states it in the caption.
 */
export function stageLevels(points: StagePoint[]): string {
  const data = points.map((p, i) => ({
    order: i,
    stage: stageLabel(p.stage),
    cents: p.cents,
    title: `${stageLabel(p.stage)} — ${dollars(p.cents)}`,
  }));
  const last = data[data.length - 1];
  const first = data[0];

  return renderPlot({
    width: 760,
    height: 300,
    marginLeft: 72,
    marginBottom: 78,
    marginRight: 24,
    x: {
      type: 'point',
      domain: data.map((d) => d.stage),
      label: null,
      tickRotate: -35,
      line: false,
      ...AXIS,
    },
    y: {
      label: null,
      grid: true,
      ticks: 5,
      tickFormat: (d: number) => compact(d),
      ...AXIS,
    },
    marks: [
      Plot.line(data, {
        x: 'stage',
        y: 'cents',
        stroke: PALETTE.single,
        strokeWidth: 2,
        curve: 'linear',
      }),
      Plot.dot(data, {
        x: 'stage',
        y: 'cents',
        fill: PALETTE.single,
        stroke: PALETTE.surface,
        strokeWidth: 2,
        r: 4.5,
        title: 'title',
      }),
      // Selective labels: where the series opens and where it lands. The eight figures
      // between them are read from the axis, the hover title, or the table.
      ...(first
        ? [
            Plot.text([first], {
              x: 'stage',
              y: 'cents',
              text: (d: { cents: number }) => compact(d.cents),
              dy: -14,
              textAnchor: 'start',
              fill: PALETTE.text,
              fontSize: 11,
            }),
          ]
        : []),
      ...(last
        ? [
            Plot.text([last], {
              x: 'stage',
              y: 'cents',
              text: (d: { cents: number }) => compact(d.cents),
              dy: -14,
              textAnchor: 'end',
              fill: PALETTE.text,
              fontSize: 11,
            }),
          ]
        : []),
    ],
  });
}

/**
 * Per-transition movement as a diverging bar.
 *
 * This is the figure that answers the question the level chart only implies: not what the
 * number was, but who changed it and by how much.
 */
export function stageDeltas(points: StagePoint[]): string {
  const data = points
    .filter((p) => p.delta !== null)
    .map((p) => ({
      stage: stageLabel(p.stage),
      delta: p.delta as number,
      actor: p.attributedTo,
      title: p.attributedTo
        ? `${stageLabel(p.stage)} — ${fmtDelta(p.delta as number)} (${p.attributedTo})`
        : `${stageLabel(p.stage)} — ${fmtDelta(p.delta as number)}, unattributed`,
    }));

  // Label the two largest movements and nothing else; the rest are in the table below.
  // Split by direction so each label sits outside its bar's end — a single mark cannot,
  // since Plot takes `textAnchor` and `dx` as constants rather than per-datum channels.
  const ranked = [...data].sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta));
  const labelled = new Set(ranked.slice(0, 2).map((d) => d.stage));
  const labelledData = data.filter((d) => labelled.has(d.stage));
  const positives = labelledData.filter((d) => d.delta > 0);
  const negatives = labelledData.filter((d) => d.delta < 0);

  return renderPlot({
    width: 760,
    height: Math.max(220, data.length * 34 + 70),
    marginLeft: 148,
    marginRight: 72,
    marginBottom: 42,
    x: {
      label: null,
      grid: true,
      tickFormat: (d: number) => compact(d),
      ...AXIS,
    },
    y: {
      label: null,
      domain: data.map((d) => d.stage),
      ...AXIS,
    },
    marks: [
      Plot.barX(data, {
        y: 'stage',
        x: 'delta',
        fill: (d: { delta: number }) =>
          d.delta > 0 ? PALETTE.increase : d.delta < 0 ? PALETTE.decrease : PALETTE.neutral,
        rx: 3,
        // A 2px surface gap between adjacent fills, not a stroke around each bar.
        insetTop: 1,
        insetBottom: 1,
        title: 'title',
      }),
      Plot.ruleX([0], { stroke: PALETTE.ink, strokeWidth: 1 }),
      Plot.text(positives, {
        y: 'stage',
        x: 'delta',
        text: (d: { delta: number }) => fmtDelta(d.delta),
        textAnchor: 'start',
        dx: 8,
        fill: PALETTE.text,
        fontSize: 11,
      }),
      Plot.text(negatives, {
        y: 'stage',
        x: 'delta',
        text: (d: { delta: number }) => fmtDelta(d.delta),
        textAnchor: 'end',
        dx: -8,
        fill: PALETTE.text,
        fontSize: 11,
      }),
    ],
  });
}

/**
 * Spending against authority, as a meter.
 *
 * Two bars would be the obvious form and the wrong one: at this scale the pair is visually
 * identical, and a reader would take "no visible difference" for "no difference". A meter
 * puts the authority in the track and the spending in the fill, so overshoot is the one
 * thing the figure can show.
 */
export function gapMeter(gap: GapResult): string {
  const authority = gap.appropriated_cents;
  const spent = gap.spent_cents;
  const ceiling = Math.max(authority, spent);

  return renderPlot({
    width: 760,
    height: 148,
    marginLeft: 96,
    marginRight: 40,
    marginTop: 24,
    marginBottom: 44,
    x: {
      domain: [0, ceiling * 1.02],
      label: null,
      grid: true,
      tickFormat: (d: number) => compact(d),
      ...AXIS,
    },
    y: { axis: null, domain: ['spent'], range: [40, 88] },
    marks: [
      // The track: authority granted.
      Plot.barX([{ y: 'spent', x: authority }], {
        y: 'y',
        x: 'x',
        fill: '#e8e4dc',
        rx: 3,
        title: `Authority granted — ${dollars(authority)}`,
      }),
      // The fill: money actually out the door.
      Plot.barX([{ y: 'spent', x: spent }], {
        y: 'y',
        x: 'x',
        fill: spent > authority ? PALETTE.increase : PALETTE.decrease,
        rx: 3,
        insetTop: 8,
        insetBottom: 8,
        title: `Disbursed (${gap.basis}) — ${dollars(spent)}`,
      }),
      // The limit, marked so the crossing is visible rather than inferred.
      Plot.ruleX([authority], { stroke: PALETTE.ink, strokeWidth: 1.5 }),
      Plot.text([{ x: authority, label: `authority ${compact(authority)}` }], {
        x: 'x',
        text: 'label',
        frameAnchor: 'top',
        dy: -6,
        textAnchor: 'end',
        dx: -4,
        fill: PALETTE.text,
        fontSize: 11,
      }),
    ],
  });
}

const COVERAGE_ORDER = ['computed', 'refused', 'unavailable'] as const;

const COVERAGE_FILL: Record<(typeof COVERAGE_ORDER)[number], string> = {
  // A figure exists.
  computed: PALETTE.decrease,
  // Both sides are present and the calculator declined the subtraction anyway.
  refused: PALETTE.increase,
  // The corpus does not hold the figures. Neutral by intent — absence is not a finding.
  unavailable: PALETTE.neutral,
};

export { COVERAGE_FILL, COVERAGE_ORDER };

/**
 * How much of the central question is currently answerable, as part-to-whole.
 *
 * Three states, each also named in a legend and a table, so nothing rests on colour — the
 * neutral slot is a near-gray on purpose and would not survive a categorical check alone.
 */
export function coverageBar(coverage: GapCoverage[]): string {
  const counts = COVERAGE_ORDER.map((status) => ({
    status,
    n: coverage.filter((c) => c.outcome.status === status).length,
  })).filter((d) => d.n > 0);

  return renderPlot({
    width: 760,
    height: 108,
    marginLeft: 8,
    marginRight: 8,
    marginTop: 30,
    marginBottom: 30,
    x: { axis: null },
    y: { axis: null },
    color: { domain: COVERAGE_ORDER, range: COVERAGE_ORDER.map((s) => COVERAGE_FILL[s]) },
    marks: [
      Plot.barX(counts, {
        x: 'n',
        fill: 'status',
        rx: 3,
        // 2px of surface between segments rather than a stroke around each.
        insetLeft: 1,
        insetRight: 1,
        title: (d: { status: string; n: number }) => `${d.n} ${d.status}`,
      }),
      Plot.text(counts, {
        x: 'n',
        text: (d: { n: number; status: string }) => `${d.n} ${d.status}`,
        // Only label a segment wide enough to hold the text; the rest are in the legend.
        filter: (d: { n: number }) => d.n / coverage.length > 0.18,
        fill: '#ffffff',
        fontSize: 11,
        textAnchor: 'middle',
      }),
    ],
  });
}

/**
 * Nodes carrying each claim marker.
 *
 * Not part-to-whole: a node routinely carries a verified figure and an open question about
 * it at once, so the three counts overlap and a stack would sum to more than the corpus.
 * Three independent bars in one hue, with the axis carrying identity.
 */
export function claimStates(counts: ManifestCounts, total: number): string {
  const data = [
    { state: 'Carries a verified claim', n: counts.verified_nodes },
    { state: 'Carries an open claim', n: counts.open_nodes },
    { state: 'Nodes in the corpus', n: total },
  ];

  return renderPlot({
    width: 640,
    height: 176,
    marginLeft: 178,
    marginRight: 64,
    marginBottom: 36,
    x: { label: null, grid: true, ...AXIS },
    y: { label: null, domain: data.map((d) => d.state), ...AXIS },
    marks: [
      Plot.barX(data, {
        y: 'state',
        x: 'n',
        fill: PALETTE.single,
        rx: 3,
        insetTop: 3,
        insetBottom: 3,
        title: (d: { state: string; n: number }) => `${d.state}: ${d.n}`,
      }),
      Plot.text(data, {
        y: 'state',
        x: 'n',
        text: (d: { n: number }) => String(d.n),
        textAnchor: 'start',
        dx: 8,
        fill: PALETTE.text,
        fontSize: 11,
      }),
      Plot.ruleX([0], { stroke: PALETTE.ink, strokeWidth: 1 }),
    ],
  });
}
