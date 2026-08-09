/**
 * Renders a corpus node body as HTML.
 *
 * Node bodies and decision records are markdown-ish: paragraphs, lists, emphasis, code
 * spans, inline links written as repository-relative file paths, and claim markers. Two of
 * those need real handling.
 *
 * **Links are file paths, not URLs.** `../program/fair-school-funding-plan.yml` is meaningful
 * relative to the file it appears in, and has to be resolved against that file's directory
 * before it can become a route. Getting this wrong silently produces links that 404 — which
 * is why `resolveHref` is tested against every shape the corpus actually uses.
 *
 * **Claim markers are content, not decoration.** `[verified]`, `[inference]` and `[open]` are
 * the corpus's epistemic vocabulary. They are marked up so a reader can see at a glance which
 * sentences rest on a committed source, and they are never stripped.
 */

import type { ClaimTag } from './types.ts';

const CLAIM_TAGS: readonly ClaimTag[] = ['verified', 'inference', 'open'];

/** `[text](href)` — the closing bracket and opening paren must be adjacent. */
const LINK = /\[([^\]]+)\]\(([^)\s]+)\)/g;
/** A bare marker: `[open]` with no `(` behind it, so it is not the start of a link. */
const MARKER = /\[(verified|inference|open)\](?!\()/g;
/** A code span's contents are literal — no link, marker or emphasis inside it is markup. */
const CODE = /`([^`\n]+)`/g;
const BOLD = /\*\*([^*\n]+)\*\*/g;
const BULLET = /^\s*[-*]\s+/;
const NUMBERED = /^\s*\d+\.\s+/;

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/** Collapses `.` and `..` lexically. Mirrors `corpus_validate::normalize_join`. */
export function normalizeJoin(baseDir: string, rel: string): string {
  const out = baseDir.split('/').filter((p) => p.length > 0 && p !== '.');
  for (const part of rel.split('/')) {
    if (part === '' || part === '.') continue;
    if (part === '..') out.pop();
    else out.push(part);
  }
  return out.join('/');
}

/**
 * Maps a corpus-relative file path onto a route, or returns `null` where the target is a
 * real file with no page of its own — a crate, a fixture, a source document.
 *
 * `fromPath` is the repository-relative path of the file the link appears in.
 */
export function resolveHref(fromPath: string, href: string): string | null {
  if (/^(https?:|mailto:|#|\/)/.test(href)) return href;

  const baseDir = fromPath.split('/').slice(0, -1).join('/');
  const target = normalizeJoin(baseDir, href);

  const ont = target.match(/^\.yidam\/corpus\/([^/]+)\.ont\.yml$/);
  if (ont) return `/wiki/${ont[1]}`;

  const node = target.match(/^\.yidam\/corpus\/([^/]+)\/([^/]+)\.yml$/);
  if (node) return `/wiki/${node[1]}/${node[2]}`;

  const catalog = target.match(/^\.yidam\/catalog\/([^/]+)\.md$/);
  if (catalog && catalog[1] !== 'README') return `/sources/${catalog[1]}`;

  const decision = target.match(/^\.yidam\/decisions\/([^/]+)\.yml$/);
  if (decision) return `/methods#decision-${decision[1]}`;

  const skill = target.match(/^\.yidam\/skills\/([^/]+)\.md$/);
  if (skill && skill[1] !== 'README') return `/methods#skill-${skill[1]}`;

  return null;
}

/** Everything except code spans: links, emphasis, and claim markers. */
function renderMarkup(escaped: string, fromPath: string): string {
  return escaped
    .replace(LINK, (_whole, label: string, href: string) => {
      const to = resolveHref(fromPath, href);
      if (to === null) {
        // A path that exists in the repository but has no page. Showing the label alone
        // would quietly drop the reference; showing the path keeps it checkable.
        return `${label} <code class="path">${href}</code>`;
      }
      const external = /^https?:/.test(to);
      const attrs = external ? ' rel="noopener noreferrer" target="_blank"' : '';
      return `<a href="${to}"${attrs}>${label}</a>`;
    })
    .replace(BOLD, (_whole, inner: string) => `<strong>${inner}</strong>`)
    .replace(MARKER, (_whole, tag: string) => `<span class="claim claim--${tag}">[${tag}]</span>`);
}

/**
 * Inline rendering. The text is split on code spans and only the segments between them are
 * treated as markup.
 *
 * Splitting rather than substituting placeholder tokens: a placeholder can collide with real
 * content, and the decision records in this corpus discuss `[open]` and file paths *inside*
 * backticks, where turning them into a marker or a link would be wrong.
 */
function renderInline(text: string, fromPath: string): string {
  const escaped = escapeHtml(text);
  let out = '';
  let last = 0;
  for (const m of escaped.matchAll(CODE)) {
    out += renderMarkup(escaped.slice(last, m.index), fromPath);
    out += `<code>${m[1]}</code>`;
    last = m.index + m[0].length;
  }
  return out + renderMarkup(escaped.slice(last), fromPath);
}

/** Joins a block's wrapped lines, honouring list-item boundaries. */
function listItems(block: string, marker: RegExp): string[] {
  const items: string[] = [];
  for (const line of block.split('\n')) {
    if (line.trim().length === 0) continue;
    if (marker.test(line)) items.push(line.replace(marker, '').trim());
    else if (items.length > 0) items[items.length - 1] += ` ${line.trim()}`;
  }
  return items.filter((i) => i.length > 0);
}

/**
 * Body text to HTML.
 *
 * Blank lines separate blocks. Inside a block, single newlines are YAML block-scalar
 * wrapping and collapse to spaces — except where the lines are list items, which the corpus
 * uses in decision records and the longer node bodies.
 */
export function renderBody(text: string, fromPath: string): string {
  return text
    .split(/\n\s*\n/)
    .filter((block) => block.trim().length > 0)
    .map((block) => {
      const lines = block.split('\n').filter((l) => l.trim().length > 0);
      const first = lines[0] ?? '';

      if (BULLET.test(first)) {
        const items = listItems(block, BULLET);
        return `<ul>${items.map((i) => `<li>${renderInline(i, fromPath)}</li>`).join('')}</ul>`;
      }
      if (NUMBERED.test(first)) {
        const items = listItems(block, NUMBERED);
        return `<ol>${items.map((i) => `<li>${renderInline(i, fromPath)}</li>`).join('')}</ol>`;
      }

      const heading = first.match(/^#{1,4}\s+(.*)$/);
      if (heading) return `<h4>${renderInline(heading[1] ?? '', fromPath)}</h4>`;

      return `<p>${renderInline(block.trim().replace(/\s*\n\s*/g, ' '), fromPath)}</p>`;
    })
    .join('\n');
}

/** Single-paragraph rendering, for property values and table cells. */
export function renderInlineText(text: string, fromPath: string): string {
  return renderInline(text.trim().replace(/\s*\n\s*/g, ' '), fromPath);
}

/** Body text with markup removed, clipped to `max` characters on a word boundary. */
export function excerpt(text: string, max = 220): string {
  const flat = text
    .replace(LINK, '$1')
    .replace(CODE, '$1')
    .replace(BOLD, '$1')
    .replace(MARKER, '')
    .replace(/\s+/g, ' ')
    .trim();
  if (flat.length <= max) return flat;
  const cut = flat.slice(0, max);
  const lastSpace = cut.lastIndexOf(' ');
  return `${cut.slice(0, lastSpace > 0 ? lastSpace : max).trimEnd()}…`;
}

/** True when the text carries the given marker. */
export function hasClaim(text: string, tag: ClaimTag): boolean {
  return text.includes(`[${tag}]`);
}

export { CLAIM_TAGS };
