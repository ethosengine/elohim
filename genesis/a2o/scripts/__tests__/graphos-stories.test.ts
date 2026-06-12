import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  componentPrefix,
  familyOf,
  groupByComponent,
  listStories,
  matchesComponent,
  sheetHtml,
  storiesForSheet,
  suggestComponents,
  type StorybookIndex,
  type StoryEntry,
} from '../lib/graphos-stories.js';

const COMPUTE_TILE = 'elohim-compute-tile';
const COMPUTE_TILE_PREFIX = 'designed-core-elohim-compute-tile';
const COMPUTE_TILE_DARK = 'designed-core-elohim-compute-tile--dark';
const COMPUTE_TILE_TITLE = 'Designed/Core/elohim-compute-tile';

function entry(
  id: string,
  title: string,
  name: string,
  type: 'story' | 'docs' = 'story'
): StoryEntry {
  return { id, title, name, type };
}

function index(): StorybookIndex {
  return {
    v: 5,
    entries: {
      'default-core-elohim-compute-tile--minimal': entry(
        'default-core-elohim-compute-tile--minimal',
        'Default/Core/elohim-compute-tile',
        'Minimal'
      ),
      'designed-core-elohim-compute-tile--standard': entry(
        'designed-core-elohim-compute-tile--standard',
        COMPUTE_TILE_TITLE,
        'Standard'
      ),
      [COMPUTE_TILE_DARK]: entry(COMPUTE_TILE_DARK, COMPUTE_TILE_TITLE, 'Dark'),
      'designed-foundations-compute-capacity-tokens--docs': entry(
        'designed-foundations-compute-capacity-tokens--docs',
        'Designed/Foundations/Compute Capacity Tokens',
        'Docs',
        'docs'
      ),
      'designed-core-elohim-presence-badge--standard': entry(
        'designed-core-elohim-presence-badge--standard',
        'Designed/Core/elohim-presence-badge',
        'Standard'
      ),
    },
  };
}

void describe('componentPrefix', () => {
  void it('returns the id prefix before --', () => {
    assert.equal(
      componentPrefix('designed-core-elohim-compute-tile--standard'),
      COMPUTE_TILE_PREFIX
    );
  });
  void it('returns the whole id when there is no --', () => {
    assert.equal(componentPrefix('foundations-colors'), 'foundations-colors');
  });
});

void describe('matchesComponent (segment-aligned)', () => {
  void it('matches when prefix ends with -<component>', () => {
    assert.ok(matchesComponent(COMPUTE_TILE_DARK, COMPUTE_TILE));
  });
  void it('matches exact prefix', () => {
    assert.ok(matchesComponent('elohim-compute-tile--dark', COMPUTE_TILE));
  });
  void it('does NOT match a bare substring tail (tile)', () => {
    assert.equal(matchesComponent(COMPUTE_TILE_DARK, 'tile'), false);
  });
});

void describe('familyOf', () => {
  void it('lowercases the first title segment', () => {
    assert.equal(familyOf(entry('x--y', COMPUTE_TILE_TITLE, 'Y')), 'designed');
  });
});

void describe('listStories', () => {
  void it('returns all entries without a filter', () => {
    assert.equal(listStories(index()).length, 5);
  });
  void it('filters by id substring, case-insensitive', () => {
    const got = listStories(index(), 'COMPUTE-TILE');
    assert.equal(got.length, 3);
  });
  void it('filters by title substring too', () => {
    const got = listStories(index(), 'capacity tokens');
    assert.equal(got.length, 1);
    assert.equal(got[0].type, 'docs');
  });
});

void describe('groupByComponent', () => {
  void it('groups by component prefix preserving order', () => {
    const groups = groupByComponent(listStories(index()));
    assert.deepEqual(
      [...groups.keys()],
      [
        'default-core-elohim-compute-tile',
        COMPUTE_TILE_PREFIX,
        'designed-foundations-compute-capacity-tokens',
        'designed-core-elohim-presence-badge',
      ]
    );
    assert.equal(groups.get(COMPUTE_TILE_PREFIX)?.length, 2);
  });
});

void describe('suggestComponents', () => {
  void it('suggests component prefixes containing the name, deduped', () => {
    const got = suggestComponents(index(), 'compute');
    assert.ok(got.includes(COMPUTE_TILE_PREFIX));
    assert.ok(got.includes('default-core-elohim-compute-tile'));
    assert.equal(new Set(got).size, got.length);
  });
  void it('respects the limit', () => {
    assert.ok(suggestComponents(index(), 'e', 2).length <= 2);
  });
});

void describe('storiesForSheet', () => {
  void it('selects story-type entries for the component across families', () => {
    const got = storiesForSheet(index(), COMPUTE_TILE);
    assert.equal(got.length, 3);
    assert.ok(got.every(e => e.type === 'story'));
  });
  void it('narrows by family', () => {
    const got = storiesForSheet(index(), COMPUTE_TILE, 'designed');
    assert.equal(got.length, 2);
  });
  void it('excludes docs entries', () => {
    const got = storiesForSheet(index(), 'compute-capacity-tokens');
    assert.equal(got.length, 0);
  });
});

void describe('sheetHtml', () => {
  void it('renders one labeled iframe per story, grouped by family, with grid cols', () => {
    const entries = storiesForSheet(index(), COMPUTE_TILE);
    const html = sheetHtml({
      component: COMPUTE_TILE,
      base: 'https://storybook.elohim.host',
      entries,
      cell: { width: 420, height: 320 },
      cols: 3,
    });
    assert.ok(
      html.includes('iframe.html?id=designed-core-elohim-compute-tile--standard&amp;viewMode=story')
    );
    assert.equal((html.match(/<iframe /g) ?? []).length, 3);
    assert.ok(html.includes('<h2>default</h2>'));
    assert.ok(html.includes('<h2>designed</h2>'));
    assert.ok(html.includes('repeat(3, 420px)'));
    assert.ok(html.includes('<figcaption>Standard</figcaption>'));
  });
  void it('escapes HTML in names', () => {
    const html = sheetHtml({
      component: 'x',
      base: 'https://s',
      entries: [entry('x--a', 'Default/x', '<b>&')],
      cell: { width: 100, height: 100 },
      cols: 1,
    });
    assert.ok(html.includes('&lt;b&gt;&amp;'));
  });
});

// ---------------------------------------------------------------------------
// sheetHtml — viewport bands (viewport archetypes as a matrix dimension,
// sibling of the default/designed family sections)
// ---------------------------------------------------------------------------

describe('sheetHtml viewport bands', () => {
  const entries: StoryEntry[] = [
    {
      id: 'default-core-elohim-hypercard-panel--standard',
      title: 'Default/Core/elohim-hypercard-panel',
      name: 'Standard',
      type: 'story',
    },
  ];
  const baseOpts = {
    component: 'elohim-hypercard-panel',
    base: 'https://storybook.example',
    entries,
    cell: { width: 420, height: 320 },
    cols: 3,
  };

  it('renders one band per viewport with iframes at the TRUE archetype size', () => {
    const html = sheetHtml({
      ...baseOpts,
      viewports: [
        { name: 'phone', width: 390, height: 844 },
        { name: 'desktop', width: 1280, height: 800 },
      ],
    });
    // Band headings name the archetype and its true size.
    assert.ok(html.includes('phone (390×844)'), 'phone band heading');
    assert.ok(html.includes('desktop (1280×800)'), 'desktop band heading');
    // Iframes render at the archetype's true pixel size so vw-relative CSS
    // and breakpoints resolve as on a real device...
    assert.ok(html.includes('width="390"'), 'phone-true-size iframe');
    assert.ok(html.includes('width="1280"'), 'desktop-true-size iframe');
    // ...and oversized archetypes are scaled down to fit the cell.
    assert.ok(/scale\(0\.3\d+\)/.test(html), 'desktop iframe scaled to cell width');
  });

  it('keeps the single-grid shape when viewports are absent', () => {
    const html = sheetHtml(baseOpts);
    assert.ok(!html.includes('class="band"'), 'no viewport bands');
    assert.ok(html.includes(`width="420"`), 'iframe at cell width');
  });
});
