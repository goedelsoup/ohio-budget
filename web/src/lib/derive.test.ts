import { describe, expect, it } from 'vitest';
import {
  enactedByPeriod,
  headlineGap,
  indexSensitivity,
  landingSubjects,
  periodKind,
  periodStart,
  realSeriesFor,
  seriesChange,
  stagePoints,
} from './derive.ts';
import type {
  Character,
  Corpus,
  Decomposition,
  Feed,
  GapCoverage,
  Manifest,
  NodeView,
  SeriesCoverage,
  Step,
} from './types.ts';

// ─── fixtures ────────────────────────────────────────────────────────────────

function node(partial: Partial<NodeView> & Pick<NodeView, 'slug' | 'class' | 'label'>): NodeView {
  return {
    description: '',
    path: `.yidam/corpus/${partial.class}/${partial.slug}.yml`,
    properties: {},
    money_cents: {},
    links: [],
    backlinks: [],
    claims: [],
    is_open_question: false,
    ...partial,
  };
}

function step(partial: Partial<Step> & Pick<Step, 'from' | 'to'>): Step {
  return {
    from_cents: 0,
    to_cents: 0,
    delta_cents: 0,
    skipped: [],
    attributions: [],
    anomaly: null,
    is_attributed: false,
    ...partial,
  };
}

function manifest(deflator: boolean): Manifest {
  return {
    contract_version: '1',
    commit: 'abc1234',
    counts: {
      classes: 1,
      nodes: 1,
      catalog: 0,
      decisions: 0,
      skills: 0,
      verified_nodes: 0,
      open_nodes: 0,
    },
    real_dollars: {
      deflator_available: deflator,
      reason: 'No deflator is supplied.',
    },
  };
}

const CHARACTER: Character = {
  policy_areas: [],
  formula_driven: false,
  federally_matched: false,
  how_to_read: 'a reading the calculator supplied',
};

/** A computed gap outcome as `crates/gap` emits one. */
function computedGap(
  line_item: string,
  period: string,
  appropriated_cents: number,
  spent_cents: number,
): GapCoverage {
  const variance_cents = appropriated_cents - spent_cents;
  return {
    line_item,
    period,
    outcome: {
      status: 'computed',
      line_item,
      period,
      appropriated_cents,
      spent_cents,
      variance_cents,
      variance_pct: appropriated_cents === 0 ? null : (variance_cents / appropriated_cents) * 100,
      reversion_cents: null,
      basis: 'actual-closed',
      provisional: false,
      character: CHARACTER,
    },
  };
}

function feed(
  corpus: Corpus,
  opts: {
    deflator?: boolean;
    gap?: GapCoverage[];
    stageDelta?: Decomposition[];
    realTerms?: SeriesCoverage[];
  } = {},
): Feed {
  return {
    manifest: manifest(opts.deflator ?? false),
    corpus,
    catalog: [],
    decisions: [],
    skills: [],
    findings: {
      gap: opts.gap ?? [],
      gap_summary: {
        computed: 0,
        appropriated_cents: 0,
        spent_cents: 0,
        overspent: 0,
        by_character: [],
      },
      gap_trend: [],
      stage_delta: opts.stageDelta ?? [],
      real_terms: opts.realTerms ?? [],
    },
  };
}

/** A restated series as `crates/corpus-export` emits one. */
function restated(line_item: string, points: [string, number, number][]): SeriesCoverage {
  return {
    line_item,
    outcome: {
      status: 'restated',
      line_item,
      series_name: 'test index',
      base_period: 'FY2025',
      points: points.map(([period, nominal_cents, real_cents], i) => ({
        period,
        slug: `n${i}`,
        nominal_cents,
        real_cents,
      })),
    },
  };
}

// ─── periods ─────────────────────────────────────────────────────────────────

describe('periodKind', () => {
  it('reads a single fiscal year as annual', () => {
    expect(periodKind('FY2026')).toBe('annual');
  });

  it('reads a biennium as biennial', () => {
    expect(periodKind('FY2024-25')).toBe('biennial');
    expect(periodKind('FY2024-2025')).toBe('biennial');
  });

  it('does not guess at an unrecognized label', () => {
    expect(periodKind('the 2026 budget')).toBe('unknown');
  });
});

describe('periodStart', () => {
  it('sorts on the first year named', () => {
    expect(periodStart('FY2024-25')).toBe(2024);
    expect(periodStart('FY2026')).toBe(2026);
  });
});

// ─── stage movement ──────────────────────────────────────────────────────────

describe('stagePoints', () => {
  const decomposition: Decomposition = {
    line_item: 'foundation-funding',
    period: 'FY2026',
    net_cents: 3_261_179_800,
    missing_stages: [],
    is_complete: true,
    steps: [
      step({
        from: 'as-introduced',
        to: 'house-substitute',
        from_cents: 842_498_697_400,
        to_cents: 851_723_697_400,
        delta_cents: 9_225_000_000,
        is_attributed: true,
        attributions: [
          {
            action: 'hb96-house-amendment',
            action_type: 'amendment',
            actor: 'house-finance-committee',
            stated_justification: 'Provision EDUCD26',
          },
        ],
      }),
      step({
        from: 'house-substitute',
        to: 'house-reported',
        from_cents: 851_723_697_400,
        to_cents: 851_873_697_400,
        delta_cents: 150_000_000,
      }),
    ],
  };

  it('reconstructs one point per stage, including the opening figure', () => {
    const out = stagePoints(decomposition);
    expect(out.ok).toBe(true);
    if (!out.ok) return;
    expect(out.data.map((p) => p.stage)).toEqual([
      'as-introduced',
      'house-substitute',
      'house-reported',
    ]);
    expect(out.data[0]?.cents).toBe(842_498_697_400);
    expect(out.data[2]?.cents).toBe(851_873_697_400);
  });

  it('leaves the opening point without a delta', () => {
    const out = stagePoints(decomposition);
    if (!out.ok) throw new Error('expected points');
    expect(out.data[0]?.delta).toBeNull();
    expect(out.data[1]?.delta).toBe(9_225_000_000);
  });

  it('carries the actor only where the calculator attributed the step', () => {
    const out = stagePoints(decomposition);
    if (!out.ok) throw new Error('expected points');
    expect(out.data[1]?.attributedTo).toBe('house-finance-committee');
    // An unattributed movement must not borrow the previous step's actor.
    expect(out.data[2]?.attributedTo).toBeNull();
  });

  it('refuses a decomposition with no steps rather than plotting one point', () => {
    const empty: Decomposition = { ...decomposition, steps: [] };
    const out = stagePoints(empty);
    expect(out.ok).toBe(false);
    if (out.ok) return;
    expect(out.reason).toContain('fewer than two');
  });
});

// ─── enacted series ──────────────────────────────────────────────────────────

describe('enactedByPeriod', () => {
  const corpus: Corpus = {
    classes: [],
    nodes: [
      node({
        slug: 'foundation-funding',
        class: 'line-item',
        label: 'Foundation Funding',
        backlinks: [
          {
            relationship: 'grants-authority-for',
            slug: 'ff-fy2026',
            class: 'appropriation',
            label: 'FY2026',
          },
          {
            relationship: 'grants-authority-for',
            slug: 'ff-fy2024',
            class: 'appropriation',
            label: 'FY2024',
          },
          {
            relationship: 'grants-authority-for',
            slug: 'ff-fy2026-house',
            class: 'appropriation',
            label: 'FY2026 house',
          },
          {
            relationship: 'grants-authority-for',
            slug: 'ff-fy2020',
            class: 'appropriation',
            label: 'FY2020',
          },
        ],
      }),
      node({
        slug: 'ff-fy2026',
        class: 'appropriation',
        label: 'FY2026',
        properties: { stage: 'as-enacted', period_label: 'FY2026' },
        money_cents: { amount: 845_759_877_200 },
      }),
      node({
        slug: 'ff-fy2024',
        class: 'appropriation',
        label: 'FY2024',
        properties: { stage: 'as-enacted', period_label: 'FY2024' },
        money_cents: { amount: 796_725_000_000 },
      }),
      // A non-enacted stage: part of the bill's history, not of the enacted series.
      node({
        slug: 'ff-fy2026-house',
        class: 'appropriation',
        label: 'FY2026 house',
        properties: { stage: 'as-passed-house', period_label: 'FY2026' },
        money_cents: { amount: 851_873_697_400 },
      }),
      // Enacted but with no figure: [open] is absence, so it contributes no point.
      node({
        slug: 'ff-fy2020',
        class: 'appropriation',
        label: 'FY2020',
        properties: { stage: 'as-enacted', period_label: 'FY2020-21' },
      }),
    ],
  };

  it('takes only enacted appropriations that carry a figure', () => {
    const points = enactedByPeriod(corpus, 'foundation-funding');
    expect(points.map((p) => p.slug)).toEqual(['ff-fy2024', 'ff-fy2026']);
  });

  it('orders by the period start', () => {
    const points = enactedByPeriod(corpus, 'foundation-funding');
    expect(points.map((p) => p.period)).toEqual(['FY2024', 'FY2026']);
  });

  it('returns nothing for a line item the corpus does not hold', () => {
    expect(enactedByPeriod(corpus, 'no-such-line')).toEqual([]);
  });
});

describe('realSeriesFor', () => {
  const empty: Corpus = { classes: [], nodes: [] };

  it('refuses when the feed carries no series for the line item', () => {
    const out = realSeriesFor(feed(empty), 'foundation-funding');
    expect(out.ok).toBe(false);
    if (out.ok) return;
    expect(out.reason).toContain('no enacted appropriation');
  });

  it('passes through the calculator refusal rather than restating it', () => {
    // The reason is written where the decision was taken. Rewording it here would be a second
    // account of why the corpus declined, and the two would drift.
    const out = realSeriesFor(
      feed(empty, {
        realTerms: [
          {
            line_item: 'local-government-fund-distribution',
            outcome: { status: 'refused', reason: 'index does not cover ["FY2010-11"]' },
          },
        ],
      }),
      'local-government-fund-distribution',
    );
    expect(out.ok).toBe(false);
    if (out.ok) return;
    expect(out.reason).toContain('FY2010-11');
  });

  it('refuses a single point, which is a figure rather than a series', () => {
    const out = realSeriesFor(
      feed(empty, { realTerms: [restated('ff', [['FY2024', 100, 102]])] }),
      'ff',
    );
    expect(out.ok).toBe(false);
    if (out.ok) return;
    expect(out.reason).toContain('Fewer than two');
  });

  it('returns the restated series with both dollars on every point', () => {
    const out = realSeriesFor(
      feed(empty, {
        realTerms: [
          restated('ff', [
            ['FY2020', 694_288_084_500, 853_906_238_300],
            ['FY2026', 845_759_877_200, 812_792_001_700],
          ]),
        ],
      }),
      'ff',
    );
    expect(out.ok).toBe(true);
    if (!out.ok) return;
    expect(out.data.base_period).toBe('FY2025');
    expect(out.data.points[0]?.real_cents).toBe(853_906_238_300);
  });
});

describe('seriesChange', () => {
  it('reports a nominal rise that is a real fall as a sign reversal', () => {
    // Foundation funding's actual FY2020-FY2026 figures: +21.8% as written into law,
    // -4.8% against what state and local government pays for what it buys.
    const out = realSeriesFor(
      feed(
        { classes: [], nodes: [] },
        {
          realTerms: [
            restated('ff', [
              ['FY2020', 694_288_084_500, 853_906_238_300],
              ['FY2026', 845_759_877_200, 812_792_001_700],
            ]),
          ],
        },
      ),
      'ff',
    );
    if (!out.ok) throw new Error('expected a series');
    const change = seriesChange(out.data);
    expect(change?.nominalPct).toBeGreaterThan(0);
    expect(change?.realPct).toBeLessThan(0);
    expect(change?.reversesSign).toBe(true);
  });

  it('does not call it a reversal when both agree', () => {
    const out = realSeriesFor(
      feed(
        { classes: [], nodes: [] },
        { realTerms: [restated('m', [['FY2020', 100, 120], ['FY2026', 200, 190]])] },
      ),
      'm',
    );
    if (!out.ok) throw new Error('expected a series');
    expect(seriesChange(out.data)?.reversesSign).toBe(false);
  });
});

// ─── the headline gap ────────────────────────────────────────────────────────

describe('headlineGap', () => {
  const corpus: Corpus = {
    classes: [],
    nodes: [
      node({
        slug: 'medicaid-health-care-services-federal',
        class: 'line-item',
        label: 'Medicaid Health Care Services (Federal Share)',
      }),
      node({ slug: 'community-schools-funding', class: 'line-item', label: 'Community Schools Funding' }),
    ],
  };

  // Feed order is alphabetical by line item, so the small line comes first. Selecting on
  // position rather than on size is what put a $2.2M line under prose about a $7B one.
  const small = computedGap('community-schools-funding', 'FY2012', 220_000_000, 168_324_841);
  const large = computedGap(
    'medicaid-health-care-services-federal',
    'FY2023',
    866_158_538_300,
    1_049_693_074_163,
  );

  it('leads with the widest divergence rather than the first in feed order', () => {
    const out = headlineGap(feed(corpus, { gap: [small, large] }));
    expect(out?.gap.line_item).toBe('medicaid-health-care-services-federal');
    expect(out?.gap.period).toBe('FY2023');
  });

  it('reads the direction off the sign, not off the page', () => {
    const over = headlineGap(feed(corpus, { gap: [large] }));
    expect(over?.overspent).toBe(true);
    expect(over?.gap.variance_cents).toBeLessThan(0);

    const under = headlineGap(feed(corpus, { gap: [small] }));
    expect(under?.overspent).toBe(false);
    expect(under?.gap.variance_cents).toBeGreaterThan(0);
  });

  it('names the line item as the corpus labels it', () => {
    const out = headlineGap(feed(corpus, { gap: [large] }));
    expect(out?.label).toBe('Medicaid Health Care Services (Federal Share)');
  });

  it('falls back to the slug when the corpus holds no node for the line item', () => {
    const out = headlineGap(feed({ classes: [], nodes: [] }, { gap: [large] }));
    expect(out?.label).toBe('Medicaid health care services federal');
  });

  it('considers only outcomes that computed', () => {
    const blocked: GapCoverage = {
      line_item: 'pupil-transportation',
      period: 'FY2011',
      outcome: { status: 'unavailable', reason: 'no expenditure committed' },
    };
    const out = headlineGap(feed(corpus, { gap: [blocked, small] }));
    expect(out?.gap.line_item).toBe('community-schools-funding');
  });

  it('has no headline when nothing computes', () => {
    expect(headlineGap(feed(corpus, { gap: [] }))).toBeUndefined();
  });

  it('breaks a tie on period and line item, so the build is reproducible', () => {
    const a = computedGap('b-line', 'FY2020', 1_000, 2_000);
    const b = computedGap('a-line', 'FY2020', 5_000, 4_000);
    expect(headlineGap(feed(corpus, { gap: [a, b] }))?.gap.line_item).toBe('a-line');
    expect(headlineGap(feed(corpus, { gap: [b, a] }))?.gap.line_item).toBe('a-line');
  });
});

// ─── subjects ────────────────────────────────────────────────────────────────

describe('landingSubjects', () => {
  const shallow = node({
    slug: 'thin-line',
    class: 'line-item',
    label: 'Thin line',
    backlinks: [
      { relationship: 'grants-authority-for', slug: 'x', class: 'appropriation', label: 'x' },
    ],
  });
  const deep = node({
    slug: 'foundation-funding',
    class: 'line-item',
    label: 'Foundation Funding',
    backlinks: Array.from({ length: 6 }, (_, i) => ({
      relationship: 'grants-authority-for',
      slug: `a${i}`,
      class: 'appropriation',
      label: `a${i}`,
    })),
  });

  it('promotes a line item with enough around it', () => {
    const f = feed({ classes: [], nodes: [shallow, deep] });
    expect(landingSubjects(f).map((s) => s.slug)).toEqual(['foundation-funding']);
  });

  it('promotes a thin line item that nonetheless has a stage series', () => {
    // Depth is one route to a page; having something computable is another.
    const f = feed(
      { classes: [], nodes: [shallow] },
      {
        stageDelta: [
          {
            line_item: 'thin-line',
            period: 'FY2026',
            steps: [],
            missing_stages: [],
            net_cents: 0,
            is_complete: false,
          },
        ],
      },
    );
    expect(landingSubjects(f).map((s) => s.slug)).toEqual(['thin-line']);
  });

  it('leaves a line item with nothing around it off the list', () => {
    const f = feed({ classes: [], nodes: [shallow] });
    expect(landingSubjects(f)).toEqual([]);
  });
});

describe('indexSensitivity', () => {
  const empty: Corpus = { classes: [], nodes: [] };

  /** A series plus the same series restated under a second index. */
  function withAlternate(primary: [string, number, number][], alt: [string, number, number][]) {
    const a = restated('m', primary);
    const b = restated('m', alt);
    if (b.outcome.status !== 'restated') throw new Error('fixture');
    b.outcome.series_name = 'health care prices';
    return feed(empty, { realTerms: [{ ...a, alternate: b.outcome }] });
  }

  it('reports both answers and the spread between them', () => {
    // Medicaid's real figures: +4.3% under government purchases, +15.6% under health prices.
    const out = indexSensitivity(
      withAlternate(
        [['FY2014', 100, 1000], ['FY2026', 147, 1043]],
        [['FY2014', 100, 1000], ['FY2026', 147, 1156]],
      ),
      'm',
    );
    expect(out?.primaryPct).toBeCloseTo(4.3, 1);
    expect(out?.alternatePct).toBeCloseTo(15.6, 1);
    expect(out?.spreadPoints).toBeCloseTo(11.3, 1);
    expect(out?.alternateName).toBe('health care prices');
  });

  it('flags when the two indices disagree about direction', () => {
    // Does not occur anywhere in this corpus today, which is the finding — so the case that
    // would overturn it has to be detectable rather than assumed away.
    const out = indexSensitivity(
      withAlternate(
        [['FY2014', 100, 1000], ['FY2026', 147, 900]],
        [['FY2014', 100, 1000], ['FY2026', 147, 1100]],
      ),
      'm',
    );
    expect(out?.sameDirection).toBe(false);
  });

  it('is absent when no alternate was emitted', () => {
    const out = indexSensitivity(
      feed(empty, { realTerms: [restated('m', [['FY2014', 100, 1000], ['FY2026', 147, 1043]])] }),
      'm',
    );
    expect(out).toBeUndefined();
  });
});
