import { describe, expect, it } from 'vitest';
import {
  enactedByPeriod,
  landingSubjects,
  periodKind,
  periodStart,
  stagePoints,
  trendable,
} from './derive.ts';
import type { PeriodPoint } from './derive.ts';
import type { Corpus, Decomposition, Feed, Manifest, NodeView, Step } from './types.ts';

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

function feed(corpus: Corpus, opts: { deflator?: boolean; stageDelta?: Decomposition[] } = {}): Feed {
  return {
    manifest: manifest(opts.deflator ?? false),
    corpus,
    catalog: [],
    decisions: [],
    skills: [],
    findings: { gap: [], stage_delta: opts.stageDelta ?? [] },
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

describe('trendable', () => {
  const annual: PeriodPoint[] = [
    { period: 'FY2024', cents: 796_725_000_000, slug: 'a' },
    { period: 'FY2026', cents: 845_759_877_200, slug: 'b' },
  ];

  it('refuses a single point', () => {
    const out = trendable(feed({ classes: [], nodes: [] }), [annual[0]!]);
    expect(out.ok).toBe(false);
    if (out.ok) return;
    expect(out.reason).toContain('Fewer than two');
  });

  it('refuses a nominal series when no deflator is available', () => {
    const out = trendable(feed({ classes: [], nodes: [] }, { deflator: false }), annual);
    expect(out.ok).toBe(false);
    if (out.ok) return;
    expect(out.reason).toContain('No deflator');
  });

  it('refuses a series mixing annual and biennial periods before it reaches the deflator check', () => {
    // Two independent reasons; the mixed-period one is the more specific and must win,
    // because supplying a deflator would not make the comparison valid.
    const mixed: PeriodPoint[] = [
      { period: 'FY2024', cents: 1, slug: 'a' },
      { period: 'FY2024-25', cents: 2, slug: 'b' },
    ];
    const out = trendable(feed({ classes: [], nodes: [] }, { deflator: true }), mixed);
    expect(out.ok).toBe(false);
    if (out.ok) return;
    expect(out.reason).toContain('annual and biennial');
  });

  it('permits a consistent series once a deflator exists', () => {
    const out = trendable(feed({ classes: [], nodes: [] }, { deflator: true }), annual);
    expect(out.ok).toBe(true);
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
