/**
 * Loads the JSON feed and refuses to proceed on anything ambiguous.
 *
 * Two failures are worth failing the build over rather than degrading around:
 *
 * - **A missing feed.** `src/data/*.json` is generated and gitignored, so a fresh clone has
 *   none. Rendering an empty site would be indistinguishable from an empty corpus.
 * - **A contract mismatch.** A feed written by a newer `corpus-export` may have moved a
 *   field this code reads. The pages would render, with holes.
 */

import { SUPPORTED_CONTRACT, type Feed } from './types.ts';
import type {
  CatalogView,
  Corpus,
  DecisionRecord,
  Findings,
  Manifest,
  NodeView,
  SkillView,
} from './types.ts';

const REGENERATE = 'Run `mise run export` (or `cargo run --bin corpus-export -- .`) first.';

async function section<T>(name: string): Promise<T> {
  try {
    // Vite resolves this at build time; the JSON is inlined into the page bundle.
    const mod = await import(`../data/${name}.json`);
    return mod.default as T;
  } catch {
    throw new Error(`web/src/data/${name}.json is missing. ${REGENERATE}`);
  }
}

let cached: Feed | undefined;

/** Reads the whole feed once per build. */
export async function loadFeed(): Promise<Feed> {
  if (cached) return cached;

  const manifest = await section<Manifest>('manifest');
  if (manifest.contract_version !== SUPPORTED_CONTRACT) {
    throw new Error(
      `feed contract ${manifest.contract_version} but this site was written against ` +
        `${SUPPORTED_CONTRACT}. Reconcile web/src/lib/types.ts with ` +
        `crates/corpus-export/src/lib.rs before building.`,
    );
  }

  cached = {
    manifest,
    corpus: await section<Corpus>('corpus'),
    catalog: await section<CatalogView[]>('catalog'),
    decisions: await section<DecisionRecord[]>('decisions'),
    skills: await section<SkillView[]>('skills'),
    findings: await section<Findings>('findings'),
  };
  return cached;
}

/** Every node of one class, in the feed's order (class then slug). */
export function nodesOfClass(corpus: Corpus, className: string): NodeView[] {
  return corpus.nodes.filter((n) => n.class === className);
}

export function findNode(corpus: Corpus, className: string, slug: string): NodeView | undefined {
  return corpus.nodes.find((n) => n.class === className && n.slug === slug);
}

/**
 * Slugs are unique within a class but not across the corpus, so a bare-slug lookup is only
 * safe where the caller knows the class. This exists for link resolution, where the feed
 * carries both.
 */
export function nodeBySlug(corpus: Corpus, slug: string): NodeView | undefined {
  return corpus.nodes.find((n) => n.slug === slug);
}
