import { describe, expect, it } from 'vitest';
import { excerpt, normalizeJoin, renderBody, resolveHref } from './prose.ts';

const LINE_ITEM = '.yidam/corpus/line-item/foundation-funding.yml';
const APPROPRIATION = '.yidam/corpus/appropriation/foundation-funding-fy2026-as-enacted.yml';

describe('normalizeJoin', () => {
  it('collapses .. lexically', () => {
    expect(normalizeJoin('.yidam/corpus/line-item', '../program/fsfp.yml')).toBe(
      '.yidam/corpus/program/fsfp.yml',
    );
  });

  it('collapses . as a no-op', () => {
    expect(normalizeJoin('.yidam/corpus/appropriation', './other.yml')).toBe(
      '.yidam/corpus/appropriation/other.yml',
    );
  });

  it('walks up more than one level', () => {
    expect(normalizeJoin('.yidam/corpus/appropriation', '../../catalog/lsc.md')).toBe(
      '.yidam/catalog/lsc.md',
    );
  });
});

describe('resolveHref', () => {
  it('routes a sibling-class node', () => {
    expect(resolveHref(LINE_ITEM, '../program/fair-school-funding-plan.yml')).toBe(
      '/wiki/program/fair-school-funding-plan',
    );
  });

  it('routes a same-directory node', () => {
    expect(resolveHref(APPROPRIATION, './foundation-funding-fy2026-conference-report.yml')).toBe(
      '/wiki/appropriation/foundation-funding-fy2026-conference-report',
    );
  });

  it('routes a class definition to its class page, not a node page', () => {
    expect(resolveHref(APPROPRIATION, '../appropriation.ont.yml')).toBe('/wiki/appropriation');
  });

  it('routes a catalog entry to the sources section', () => {
    expect(resolveHref(APPROPRIATION, '../../catalog/lsc-hb96-comparison.md')).toBe(
      '/sources/lsc-hb96-comparison',
    );
  });

  it('routes a decision record to its anchor under methods', () => {
    expect(resolveHref(LINE_ITEM, '../../decisions/ontology.yml')).toBe(
      '/methods#decision-ontology',
    );
  });

  it('routes a skill to its anchor under methods', () => {
    expect(resolveHref(LINE_ITEM, '../../skills/gap.md')).toBe('/methods#skill-gap');
  });

  it('returns null for a repository path with no page', () => {
    // A crate is a real target with nothing to navigate to. The renderer shows the path
    // rather than dropping the reference.
    expect(resolveHref(LINE_ITEM, '../../../crates/lsc/')).toBeNull();
  });

  it('passes external and absolute links through untouched', () => {
    expect(resolveHref(LINE_ITEM, 'https://lsc.ohio.gov/')).toBe('https://lsc.ohio.gov/');
    expect(resolveHref(LINE_ITEM, '/wiki')).toBe('/wiki');
  });
});

describe('renderBody', () => {
  it('marks up claim tags rather than stripping them', () => {
    const html = renderBody('Its code is 200550. [verified]', LINE_ITEM);
    expect(html).toContain('claim--verified');
    expect(html).toContain('[verified]');
  });

  it('distinguishes a bare marker from a markdown link', () => {
    // `[open]` is a marker; `[open questions](...)` is a link. The regexes must not collide.
    const html = renderBody(
      '[open] See [the plan](../program/fair-school-funding-plan.yml).',
      LINE_ITEM,
    );
    expect(html).toContain('<span class="claim claim--open">[open]</span>');
    expect(html).toContain('<a href="/wiki/program/fair-school-funding-plan">the plan</a>');
  });

  it('does not treat a marker followed by a parenthetical as a link', () => {
    const html = renderBody('[open] (pending the obm connector)', LINE_ITEM);
    expect(html).toContain('claim--open');
    expect(html).toContain('(pending the obm connector)');
  });

  it('splits on blank lines and unwraps YAML block wrapping inside a paragraph', () => {
    const html = renderBody('One line\nwrapped here.\n\nSecond paragraph.', LINE_ITEM);
    expect(html).toBe('<p>One line wrapped here.</p>\n<p>Second paragraph.</p>');
  });

  it('escapes HTML in body text', () => {
    expect(renderBody('a < b & c', LINE_ITEM)).toBe('<p>a &lt; b &amp; c</p>');
  });

  it('keeps an unroutable path visible next to its label', () => {
    const html = renderBody('See [the connector](../../../crates/lsc/).', LINE_ITEM);
    expect(html).toContain('the connector');
    expect(html).toContain('crates/lsc/');
    expect(html).not.toContain('<a ');
  });

  it('renders code spans', () => {
    expect(renderBody('The code is `200550`.', LINE_ITEM)).toBe(
      '<p>The code is <code>200550</code>.</p>',
    );
  });

  it('treats a code span as literal, not as markup', () => {
    // Decision records discuss the markers and the paths rather than referencing them.
    // Turning `[open]` inside backticks into a marker would misread the sentence.
    const html = renderBody(
      'A value tagged `[open]` at `../program/p.yml` is absent, not zero.',
      LINE_ITEM,
    );
    expect(html).toContain('<code>[open]</code>');
    expect(html).toContain('<code>../program/p.yml</code>');
    expect(html).not.toContain('claim--open');
    expect(html).not.toContain('<a ');
  });

  it('still renders markup outside a code span in the same paragraph', () => {
    const html = renderBody('`amount` is [open] here.', LINE_ITEM);
    expect(html).toContain('<code>amount</code>');
    expect(html).toContain('claim--open');
  });

  it('does not mangle bare numbers surrounded by spaces', () => {
    // A placeholder scheme keyed on spaced digits would eat "in 2024 the" here.
    const html = renderBody('`x` held in 2024 the value 5 and `y` did not.', LINE_ITEM);
    expect(html).toContain('in 2024 the value 5 and');
    expect(html).toContain('<code>x</code>');
    expect(html).toContain('<code>y</code>');
  });

  it('renders bold', () => {
    expect(renderBody('**Never** mix bases.', LINE_ITEM)).toBe(
      '<p><strong>Never</strong> mix bases.</p>',
    );
  });

  it('renders a bullet list as a list, not as one run-on paragraph', () => {
    const html = renderBody('- first item\n- second item', LINE_ITEM);
    expect(html).toBe('<ul><li>first item</li><li>second item</li></ul>');
  });

  it('renders a numbered list', () => {
    const html = renderBody('1. first\n2. second', LINE_ITEM);
    expect(html).toBe('<ol><li>first</li><li>second</li></ol>');
  });

  it('folds a wrapped continuation line into the item above it', () => {
    const html = renderBody('- an item that\n  wraps across lines\n- a second', LINE_ITEM);
    expect(html).toBe('<ul><li>an item that wraps across lines</li><li>a second</li></ul>');
  });

  it('keeps markup working inside a list item', () => {
    const html = renderBody('- see [the plan](../program/p.yml) [verified]', LINE_ITEM);
    expect(html).toContain('<a href="/wiki/program/p">the plan</a>');
    expect(html).toContain('claim--verified');
  });
});

describe('excerpt', () => {
  it('strips markup and markers', () => {
    expect(excerpt('The [plan](../program/p.yml) is set. [verified]')).toBe('The plan is set.');
  });

  it('clips on a word boundary', () => {
    const out = excerpt('alpha beta gamma delta epsilon', 14);
    expect(out).toBe('alpha beta…');
  });

  it('leaves short text alone', () => {
    expect(excerpt('Short.')).toBe('Short.');
  });
});
