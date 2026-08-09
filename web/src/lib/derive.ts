/**
 * Turns the feed into the shapes the pages plot, and decides when there is nothing to plot.
 *
 * The deciding is the point. A chart drawn from two figures that should never have been put
 * on the same axis is worse than no chart: it is legible, it looks considered, and nothing
 * about it signals that the comparison was invalid. `.yidam/skills/real-dollars.md` names
 * this failure directly — "a chart mixing nominal and real points looks fine" — so the
 * predicates here return a stated refusal rather than an empty array, and the pages render
 * the refusal.
 *
 * Nothing here re-derives a calculator's judgment. Whether a gap may be subtracted, whether a
 * stage delta is attributable, how a variance should be read — those arrive already decided
 * in `findings.json`.
 */

import type {
  BillStage,
  Corpus,
  Decomposition,
  Feed,
  GapCoverage,
  NodeView,
  RealSeries,
  TrendCoverage,
} from './types.ts';

/** A refusal carries its reason, because the reason is what the reader needs. */
export type Chartable<T> = { ok: true; data: T } | { ok: false; reason: string };

export function refuse<T>(reason: string): Chartable<T> {
  return { ok: false, reason };
}

// ─── fiscal periods ──────────────────────────────────────────────────────────

export type PeriodKind = 'annual' | 'biennial' | 'unknown';

/**
 * `FY2026` covers one year; `FY2024-25` covers two. Summing or plotting them together
 * compares a year against a biennium, which is wrong by roughly a factor of two and looks
 * entirely plausible on an axis.
 */
export function periodKind(label: string): PeriodKind {
  if (/^FY\d{4}-\d{2,4}$/.test(label.trim())) return 'biennial';
  if (/^FY\d{4}$/.test(label.trim())) return 'annual';
  return 'unknown';
}

/** Sort key: the first year named. `FY2024-25` → 2024. */
export function periodStart(label: string): number {
  const m = label.match(/(\d{4})/);
  return m?.[1] ? Number.parseInt(m[1], 10) : Number.NaN;
}

// ─── stage movement, within one bill and one period ──────────────────────────

export interface StagePoint {
  stage: BillStage;
  cents: number;
  /** The step that arrived at this stage; absent on the first point. */
  delta: number | null;
  attributedTo: string | null;
  justification: string | null;
  anomaly: string | null;
}

/**
 * Reconstructs the stage-by-stage figures from a decomposition's steps.
 *
 * Safe to plot without a deflator: every point stands in the same fiscal period, so no
 * cross-period comparison is involved. This is the one chart the corpus fully supports.
 */
export function stagePoints(d: Decomposition): Chartable<StagePoint[]> {
  const first = d.steps[0];
  if (!first) {
    return refuse(
      `${d.line_item} ${d.period}: fewer than two staged figures, so there is no movement to show.`,
    );
  }

  const points: StagePoint[] = [
    {
      stage: first.from,
      cents: first.from_cents,
      delta: null,
      attributedTo: null,
      justification: null,
      anomaly: null,
    },
  ];
  for (const s of d.steps) {
    const actor = s.is_attributed ? (s.attributions[0]?.actor ?? null) : null;
    points.push({
      stage: s.to,
      cents: s.to_cents,
      delta: s.delta_cents,
      attributedTo: actor,
      justification: s.is_attributed ? (s.attributions[0]?.stated_justification ?? null) : null,
      anomaly: s.anomaly,
    });
  }
  return { ok: true, data: points };
}

export function decompositionFor(
  feed: Feed,
  lineItem: string,
  period?: string,
): Decomposition | undefined {
  return feed.findings.stage_delta.find(
    (d) => d.line_item === lineItem && (period === undefined || d.period === period),
  );
}

// ─── enacted figures across periods ──────────────────────────────────────────

export interface PeriodPoint {
  period: string;
  cents: number;
  slug: string;
}

/**
 * Enacted appropriations for one line item, one point per period.
 *
 * Refused as a chart whenever it would span periods, which — absent a deflator — it always
 * does once there is more than one point. The figures are still returned to the caller as a
 * table: a reader can compare two nominal figures deliberately, which is not the same as an
 * axis inviting them to read a slope.
 */
export function enactedByPeriod(corpus: Corpus, lineItemSlug: string): PeriodPoint[] {
  const li = corpus.nodes.find((n) => n.class === 'line-item' && n.slug === lineItemSlug);
  if (!li) return [];

  const points: PeriodPoint[] = [];
  for (const b of li.backlinks) {
    if (b.class !== 'appropriation') continue;
    const node = corpus.nodes.find((n) => n.class === 'appropriation' && n.slug === b.slug);
    if (!node) continue;
    if (node.properties['stage']?.trim() !== 'as-enacted') continue;
    const cents = node.money_cents['amount'];
    const period = node.properties['period_label']?.trim();
    if (cents === undefined || !period) continue;
    points.push({ period, cents, slug: node.slug });
  }
  points.sort((a, b) => periodStart(a.period) - periodStart(b.period));
  return points;
}

/**
 * The enacted series restated in constant dollars, or the reason there is none.
 *
 * # Why this reads the feed rather than deciding anything
 *
 * An earlier version of this function decided for itself: it counted the points, checked that
 * the periods did not mix annual with biennial, checked `deflator_available`, and — if all
 * three passed — returned the **nominal** points for plotting. Every guard was real and the
 * conclusion was still wrong, because passing a deflator check is not the same as having
 * deflated anything. The day a deflator was committed, that function went from refusing to
 * returning a nominal series with the caveat removed, which is the exact failure
 * `.yidam/skills/real-dollars.md` names.
 *
 * So the restatement now happens in `crates/real-dollars` and arrives here already done. This
 * reads the answer; it does not reconstruct it.
 */
export function realSeriesFor(feed: Feed, lineItemSlug: string): Chartable<RealSeries> {
  const found = feed.findings.real_terms.find((s) => s.line_item === lineItemSlug);
  if (!found) {
    return refuse('The corpus holds no enacted appropriation carrying an amount for this line item.');
  }
  if (found.outcome.status === 'refused') return refuse(found.outcome.reason);
  if (found.outcome.points.length < 2) {
    return refuse('Fewer than two enacted figures are known for this line item.');
  }
  return { ok: true, data: found.outcome };
}

/** Adjacent-period gap comparisons touching one line item, whatever their status. */
export function trendsForLineItem(feed: Feed, lineItemSlug: string): TrendCoverage[] {
  return feed.findings.gap_trend.filter((t) => t.line_item === lineItemSlug);
}

/**
 * How much of a nominal change was price level rather than money.
 *
 * Returned as a pair rather than a verdict: a series can grow in both, grow in one, or — as
 * four of this corpus's five multi-year series do — grow nominally while falling in real
 * terms. Naming which of those happened is the page's job, not this function's.
 */
export interface SeriesChange {
  from: string;
  to: string;
  nominalPct: number;
  realPct: number;
  /** True when the two disagree about direction. */
  reversesSign: boolean;
}

export function seriesChange(series: RealSeries): SeriesChange | undefined {
  const a = series.points[0];
  const b = series.points[series.points.length - 1];
  if (!a || !b || a === b || a.nominal_cents === 0 || a.real_cents === 0) return undefined;
  const nominalPct = (b.nominal_cents / a.nominal_cents - 1) * 100;
  const realPct = (b.real_cents / a.real_cents - 1) * 100;
  return {
    from: a.period,
    to: b.period,
    nominalPct,
    realPct,
    reversesSign: Math.sign(nominalPct) !== Math.sign(realPct),
  };
}

// ─── gap coverage ────────────────────────────────────────────────────────────

export function computedGaps(feed: Feed): GapCoverage[] {
  return feed.findings.gap.filter((c) => c.outcome.status === 'computed');
}

export function blockedGaps(feed: Feed): GapCoverage[] {
  return feed.findings.gap.filter((c) => c.outcome.status !== 'computed');
}

/** Gap outcomes touching one line item, whatever their status. */
export function gapsForLineItem(feed: Feed, lineItemSlug: string): GapCoverage[] {
  return feed.findings.gap.filter((c) => c.line_item === lineItemSlug);
}

// ─── subjects ────────────────────────────────────────────────────────────────

/**
 * A line item with enough around it to be worth a page of its own.
 *
 * Derived rather than listed, so a line item that gains depth through extraction gets a
 * landing page without anyone remembering to add it — and one that never does, does not get
 * an empty page that implies the corpus knows more than it does.
 */
export interface Subject {
  slug: string;
  label: string;
  node: NodeView;
  /** Nodes pointing at this line item. */
  relatedCount: number;
  /** Appropriations and expenditures on it that carry a figure. */
  figureCount: number;
  hasStageSeries: boolean;
  hasComputedGap: boolean;
}

/** Below this, a landing page would be a heading and a list of open questions. */
export const SUBJECT_THRESHOLD = 4;

export function subjects(feed: Feed): Subject[] {
  const { corpus } = feed;
  const out: Subject[] = [];

  for (const node of corpus.nodes) {
    if (node.class !== 'line-item') continue;

    let figureCount = 0;
    for (const b of node.backlinks) {
      const related = corpus.nodes.find((n) => n.class === b.class && n.slug === b.slug);
      if (related && Object.keys(related.money_cents).length > 0) figureCount += 1;
    }

    out.push({
      slug: node.slug,
      label: node.label,
      node,
      relatedCount: node.backlinks.length,
      figureCount,
      hasStageSeries: decompositionFor(feed, node.slug) !== undefined,
      hasComputedGap: gapsForLineItem(feed, node.slug).some(
        (c) => c.outcome.status === 'computed',
      ),
    });
  }

  // Depth first, then name, so the ordering is stable across builds.
  out.sort(
    (a, b) =>
      b.figureCount - a.figureCount ||
      b.relatedCount - a.relatedCount ||
      a.slug.localeCompare(b.slug),
  );
  return out;
}

/** The subjects that earn a landing page. */
export function landingSubjects(feed: Feed): Subject[] {
  return subjects(feed).filter(
    (s) => s.hasStageSeries || s.hasComputedGap || s.relatedCount >= SUBJECT_THRESHOLD,
  );
}

// ─── presentation helpers ────────────────────────────────────────────────────

const STAGE_LABELS: Record<BillStage, string> = {
  'as-introduced': 'As introduced',
  'house-substitute': 'House substitute',
  'house-reported': 'House reported',
  'as-passed-house': 'Passed House',
  'senate-substitute': 'Senate substitute',
  'senate-reported': 'Senate reported',
  'as-passed-senate': 'Passed Senate',
  'conference-report': 'Conference report',
  'as-enacted': 'As enacted',
  'post-veto': 'Post-veto',
};

export function stageLabel(stage: BillStage): string {
  return STAGE_LABELS[stage] ?? stage;
}

/** Sentence case from a corpus slug: `foundation-funding` → `Foundation funding`. */
export function humanize(slug: string): string {
  const spaced = slug.replace(/-/g, ' ');
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}
