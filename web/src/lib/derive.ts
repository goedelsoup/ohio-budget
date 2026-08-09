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

/**
 * How far the real-terms answer moves under a different price index.
 *
 * Reported rather than resolved. The corpus deflates by what state and local government buys,
 * which is defensible for a budget and is not the only defensible choice; a health-care index
 * puts Medicaid's fourteen-year change at +15.6% where the default puts it at +4.3%. Neither is
 * wrong, and a reader shown one figure alone cannot tell that the other exists.
 *
 * `sameDirection` is the part that travels: across every series in this corpus the two indices
 * agree on sign and disagree on size, so a claim about direction survives the choice and a
 * claim about magnitude does not.
 */
export interface IndexSensitivity {
  primaryPct: number;
  alternatePct: number;
  alternateName: string;
  spreadPoints: number;
  sameDirection: boolean;
}

export function indexSensitivity(feed: Feed, lineItemSlug: string): IndexSensitivity | undefined {
  const found = feed.findings.real_terms.find((s) => s.line_item === lineItemSlug);
  if (!found || found.outcome.status !== 'restated') return undefined;
  const alt = found.alternate;
  if (!alt || alt.status !== 'restated') return undefined;

  const pct = (pts: { real_cents: number }[]) => {
    const a = pts[0];
    const b = pts[pts.length - 1];
    if (!a || !b || a === b || a.real_cents === 0) return undefined;
    return (b.real_cents / a.real_cents - 1) * 100;
  };
  const primaryPct = pct(found.outcome.points);
  const alternatePct = pct(alt.points);
  if (primaryPct === undefined || alternatePct === undefined) return undefined;

  return {
    primaryPct,
    alternatePct,
    alternateName: alt.series_name,
    spreadPoints: Math.abs(alternatePct - primaryPct),
    sameDirection: Math.sign(primaryPct) === Math.sign(alternatePct),
  };
}

// ─── how each actor answers the one before it ────────────────────────────────

/**
 * One hand-off, with the shares already taken.
 *
 * Percentages are computed here rather than emitted because they are a ratio of two integers
 * the feed already carries — unlike money, where a second implementation would be a second set
 * of rules. A share of nothing stays `null`: `0%` would read as "everything was cut".
 */
export interface HandOff {
  actor: string;
  answering: string;
  bill: string;
  generalAssembly: string;
  officer: string | null;
  raisedWhenPriorRaised: number | null;
  raisedWhenPriorCut: number | null;
  /** Percentage points by which this actor favoured what its predecessor cut. */
  gap: number | null;
  movedN: number;
  /** Share of moved lines landing closer to where the predecessor started. */
  towardBaseline: number | null;
}

const shareOf = (t: { raised: number; cut: number }): number | null => {
  const n = t.raised + t.cut;
  return n > 0 ? (t.raised / n) * 100 : null;
};

export function handOffs(feed: Feed): HandOff[] {
  const p = feed.findings.process;
  if (!p) return [];
  return p.conditioning.map((c) => {
    const a = shareOf(c.when_prior_raised);
    const b = shareOf(c.when_prior_cut);
    const toward = c.moved_toward_baseline + c.moved_away_from_baseline;
    return {
      actor: c.actor,
      answering: c.answering,
      bill: c.bill,
      generalAssembly: c.general_assembly,
      officer: c.officer,
      raisedWhenPriorRaised: a,
      raisedWhenPriorCut: b,
      gap: a !== null && b !== null ? b - a : null,
      movedN: c.when_prior_raised.raised + c.when_prior_raised.cut +
        c.when_prior_cut.raised + c.when_prior_cut.cut,
      towardBaseline: toward > 0 ? (c.moved_toward_baseline / toward) * 100 : null,
    };
  });
}

/**
 * Where conference landed, by line item and by contested dollar.
 *
 * Both, and never one alone. On this corpus the two answer differently: 6.2% of contested lines
 * carry 51.2% of the contested money, so "conference rarely splits" and "conference splits most
 * of the money" are both true and only one of them is usually quoted.
 */
export interface ConferenceShares {
  bill: string;
  generalAssembly: string;
  contested: number;
  byLine: { atHouse: number; atSenate: number; between: number; outside: number };
  byDollar: { atHouse: number; atSenate: number; between: number; outside: number };
  medianPosition: number | null;
}

export function conferenceShares(feed: Feed): ConferenceShares[] {
  const p = feed.findings.process;
  if (!p) return [];
  return p.conference.map((c) => {
    const l = (v: number) => (c.contested > 0 ? (v / c.contested) * 100 : 0);
    const d = (v: number) => (c.contested_cents > 0 ? (v / c.contested_cents) * 100 : 0);
    return {
      bill: c.bill,
      generalAssembly: c.general_assembly,
      contested: c.contested,
      byLine: {
        atHouse: l(c.at_house), atSenate: l(c.at_senate),
        between: l(c.between), outside: l(c.outside_both),
      },
      byDollar: {
        atHouse: d(c.cents_at_house), atSenate: d(c.cents_at_senate),
        between: d(c.cents_between), outside: d(c.cents_outside),
      },
      medianPosition: c.median_position,
    };
  });
}

/**
 * State money to local government, in constant dollars, with the pass-through separated out.
 *
 * Years the price index cannot reach are dropped rather than shown nominal beside real ones —
 * the rule `real-dollars` exists to enforce, and the reason FY2027 does not appear.
 */
export interface LocalFinancePoint {
  period: string;
  ownSource: number;
  shared: number;
  reimbursement: number;
  stateMoney: number;
}

export function localFinance(feed: Feed): LocalFinancePoint[] {
  return (feed.findings.local_finance?.years ?? []).flatMap((y) =>
    y.real
      ? [{
          period: y.fiscal_year,
          ownSource: y.real.own_source_cents,
          shared: y.real.shared_cents,
          reimbursement: y.real.reimbursement_cents,
          stateMoney: y.real.shared_cents + y.real.reimbursement_cents,
        }]
      : [],
  );
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
