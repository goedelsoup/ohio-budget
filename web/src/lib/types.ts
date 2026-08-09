/**
 * The JSON feed contract, as emitted by `crates/corpus-export`.
 *
 * These are hand-written mirrors of the Rust types rather than generated ones, and the
 * contract version guards the seam: `feed.ts` refuses to load a feed whose
 * `contract_version` this file was not written against. A silently-accepted mismatch would
 * surface as pages that render nothing, which reads as an empty corpus rather than a
 * stale build.
 */

/** The version of `crates/corpus-export`'s output these types describe. */
export const SUPPORTED_CONTRACT = '1';

export type ClaimTag = 'verified' | 'inference' | 'open';

export type TargetKind = 'node' | 'class' | 'catalog' | 'unresolved';

export interface PropertyDef {
  name: string;
  type: string;
  description?: string;
}

export interface EdgeDef {
  relationship: string;
  target: string;
  direction: 'in' | 'out';
  description?: string;
  /**
   * On an inbound edge: whether every instance of this class expects at least one.
   * Absent means no — declaring an edge permits a reciprocal, it does not require one.
   */
  expected?: boolean;
}

export interface ClassView {
  class: string;
  label: string;
  description: string;
  foundational_type: string | null;
  properties: PropertyDef[];
  edges: EdgeDef[];
  instance_count: number;
}

export interface LinkView {
  relationship: string;
  kind: TargetKind;
  slug: string | null;
  class: string | null;
  raw: string;
}

export interface BacklinkView {
  relationship: string;
  slug: string;
  class: string;
  label: string;
}

export interface NodeView {
  slug: string;
  class: string;
  label: string;
  description: string;
  path: string;
  properties: Record<string, string>;
  /**
   * Money in whole cents. A key's absence means the figure is not stated — never zero.
   * Parsed on the Rust side by `lsc::parse_money_to_cents`; nothing here parses money.
   */
  money_cents: Record<string, number>;
  links: LinkView[];
  backlinks: BacklinkView[];
  claims: ClaimTag[];
  is_open_question: boolean;
}

export interface Corpus {
  classes: ClassView[];
  nodes: NodeView[];
}

export interface ManifestCounts {
  classes: number;
  nodes: number;
  catalog: number;
  decisions: number;
  skills: number;
  verified_nodes: number;
  open_nodes: number;
}

export interface Manifest {
  contract_version: string;
  commit: string | null;
  counts: ManifestCounts;
  /** Whether figures from different fiscal periods may be compared. See `derive.ts`. */
  real_dollars: {
    deflator_available: boolean;
    reason: string;
    /**
     * Which index. Carried on every restated figure, because the choice is contestable and a
     * constant-dollar number without it is not interpretable.
     */
    series_name?: string;
    base_period?: string;
    /** Fiscal years the index covers. */
    periods?: string[];
    /** Fiscal years it does not. A period absent from `periods` cannot be restated at all. */
    periods_uncovered?: string[];
  };
}

export interface CatalogEntry {
  slug: string;
  name: string;
  source_type: string;
  location: string;
  publisher: string;
  content_committed: boolean;
  access_constraints?: string;
  feeds: string[];
}

export type CatalogView = { slug: string; path: string } & (
  | ({ state: 'parsed' } & CatalogEntry)
  | { state: 'malformed'; error: string }
);

export interface DecisionRecord {
  id: string;
  summary: string;
  corpus_depth?: number;
  context: string;
  decision: string;
  rationale: string;
}

export interface SkillView {
  slug: string;
  name: string;
  description: string;
  path: string;
}

// ─── findings: what the calculators returned ─────────────────────────────────

export interface Character {
  policy_areas: string[];
  formula_driven: boolean;
  federally_matched: boolean;
  /** The calculator's own reading. Not re-derived here — see `corpus-export`'s module docs. */
  how_to_read: string;
}

export interface GapResult {
  line_item: string;
  period: string;
  appropriated_cents: number;
  spent_cents: number;
  /** Positive when authority exceeded spending. */
  variance_cents: number;
  /** Absent when authority is zero — a share of nothing is undefined, not nought. */
  variance_pct: number | null;
  /** Set where the pair computes and still means something unusual. */
  anomaly?: string | null;
  reversion_cents: number | null;
  basis: string;
  provisional: boolean;
  character: Character;
}

export type GapOutcome =
  | ({ status: 'computed' } & GapResult)
  | { status: 'unavailable'; reason: string }
  | { status: 'refused'; reason: string };

export interface GapCoverage {
  line_item: string;
  period: string;
  outcome: GapOutcome;
}

export type BillStage =
  | 'as-introduced'
  | 'house-substitute'
  | 'house-reported'
  | 'as-passed-house'
  | 'senate-substitute'
  | 'senate-reported'
  | 'as-passed-senate'
  | 'conference-report'
  | 'as-enacted'
  | 'post-veto';

export interface Attribution {
  action: string;
  action_type: string;
  actor: string | null;
  stated_justification: string;
}

export interface Step {
  from: BillStage;
  to: BillStage;
  from_cents: number;
  to_cents: number;
  delta_cents: number;
  /** Non-empty means the delta spans absent stages and is aggregate, not attributable. */
  skipped: BillStage[];
  attributions: Attribution[];
  anomaly: string | null;
  is_attributed: boolean;
}

export interface Decomposition {
  line_item: string;
  period: string;
  steps: Step[];
  missing_stages: BillStage[];
  net_cents: number;
  is_complete: boolean;
}

// ─── constant dollars ────────────────────────────────────────────────────────

/**
 * One period's figure in its own dollars and in the base period's.
 *
 * Both travel together deliberately. Nothing here multiplies a nominal figure by an index —
 * that would be a second deflator beside `crates/real-dollars`, and the two would drift in
 * the direction of whichever was easier to write.
 */
export interface TrendPoint {
  period: string;
  nominal_cents: number;
  real_cents: number;
}

export interface Trend {
  line_item: string;
  series_name: string;
  base_period: string;
  earlier: TrendPoint;
  later: TrendPoint;
  /** The comparable difference: later minus earlier, both in base-period dollars. */
  real_change_cents: number;
  /** What a nominal subtraction would have said, kept so the correction stays visible. */
  nominal_change_cents: number;
}

export type TrendOutcome =
  | ({ status: 'computed' } & Trend)
  | { status: 'unavailable'; reason: string }
  | { status: 'refused'; reason: string };

export interface TrendCoverage {
  line_item: string;
  earlier: string;
  later: string;
  outcome: TrendOutcome;
}

export interface RealPoint {
  period: string;
  /** The appropriation node this came from. */
  slug: string;
  nominal_cents: number;
  real_cents: number;
}

export interface RealSeries {
  line_item: string;
  series_name: string;
  base_period: string;
  points: RealPoint[];
}

export type SeriesOutcome =
  | ({ status: 'restated' } & RealSeries)
  | { status: 'refused'; reason: string };

export interface SeriesCoverage {
  line_item: string;
  outcome: SeriesOutcome;
  /**
   * The same series under a sectorally different price index. Present only where the primary
   * restated. Exists so a reader can see how much of a real-terms figure is the appropriation
   * and how much is the choice of index — which on this corpus is 8 to 14 percentage points.
   */
  alternate?: SeriesOutcome;
}

/** Counts and dollars for one group of line items. Both, because they disagree. */
export interface Tally {
  raised: number;
  cut: number;
  net_cents: number;
  gross_cents: number;
}

/**
 * What one actor did in one bill, split by what its predecessor had done to the same line item.
 *
 * The measurement that dissolved the Speaker alignment. Computed over ~1,400 line items per
 * bill rather than over the corpus's 22, because the biennium is a unit with four observations
 * and always will be — see the `the-unit-of-observation` decision record.
 */
export interface Conditioning {
  actor: string;
  answering: string;
  bill: string;
  general_assembly: string;
  /** Absent for conference, which has no single presiding officer. */
  officer: string | null;
  when_prior_raised: Tally;
  when_prior_cut: Tally;
  /** Separates reversing from restoring — see `revealed.ts`. */
  moved_toward_baseline: number;
  moved_away_from_baseline: number;
}

export interface ConferencePosition {
  bill: string;
  general_assembly: string;
  contested: number;
  at_house: number;
  at_senate: number;
  between: number;
  outside_both: number;
  contested_cents: number;
  cents_at_house: number;
  cents_at_senate: number;
  cents_between: number;
  cents_outside: number;
  /** 0.0 at the Senate's figure, 1.0 at the House's. */
  median_position: number | null;
}

export interface ProcessFindings {
  conditioning: Conditioning[];
  conference: ConferencePosition[];
  /** Agencies treated as reachable by the school funding plan. */
  plan_reaches: string[];
}

export interface Findings {
  gap: GapCoverage[];
  /** Adjacent-period comparisons of the gap, in constant dollars. */
  gap_trend: TrendCoverage[];
  stage_delta: Decomposition[];
  /** Enacted appropriations restated, one series per line item. */
  real_terms: SeriesCoverage[];
  /** Absent when the committed workbooks are not present. */
  process?: ProcessFindings;
}

export interface Feed {
  manifest: Manifest;
  corpus: Corpus;
  catalog: CatalogView[];
  decisions: DecisionRecord[];
  skills: SkillView[];
  findings: Findings;
}
