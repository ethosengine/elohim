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

function entry(id: string, title: string, name: string, type: 'story' | 'docs' = 'story'): StoryEntry {
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
        'Designed/Core/elohim-compute-tile',
        'Standard'
      ),
      'designed-core-elohim-compute-tile--dark': entry(
        'designed-core-elohim-compute-tile--dark',
        'Designed/Core/elohim-compute-tile',
        'Dark'
      ),
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

describe('componentPrefix', () => {
  it('returns the id prefix before --', () => {
    assert.equal(
      componentPrefix('designed-core-elohim-compute-tile--standard'),
      'designed-core-elohim-compute-tile'
    );
  });
  it('returns the whole id when there is no --', () => {
    assert.equal(componentPrefix('foundations-colors'), 'foundations-colors');
  });
});

describe('matchesComponent (segment-aligned)', () => {
  it('matches when prefix ends with -<component>', () => {
    assert.ok(
      matchesComponent('designed-core-elohim-compute-tile--dark', 'elohim-compute-tile')
    );
  });
  it('matches exact prefix', () => {
    assert.ok(matchesComponent('elohim-compute-tile--dark', 'elohim-compute-tile'));
  });
  it('does NOT match a bare substring tail (tile)', () => {
    assert.equal(matchesComponent('designed-core-elohim-compute-tile--dark', 'tile'), false);
  });
});

describe('familyOf', () => {
  it('lowercases the first title segment', () => {
    assert.equal(familyOf(entry('x--y', 'Designed/Core/elohim-compute-tile', 'Y')), 'designed');
  });
});

describe('listStories', () => {
  it('returns all entries without a filter', () => {
    assert.equal(listStories(index()).length, 5);
  });
  it('filters by id substring, case-insensitive', () => {
    const got = listStories(index(), 'COMPUTE-TILE');
    assert.equal(got.length, 3);
  });
  it('filters by title substring too', () => {
    const got = listStories(index(), 'capacity tokens');
    assert.equal(got.length, 1);
    assert.equal(got[0].type, 'docs');
  });
});

describe('groupByComponent', () => {
  it('groups by component prefix preserving order', () => {
    const groups = groupByComponent(listStories(index()));
    assert.deepEqual(
      [...groups.keys()],
      [
        'default-core-elohim-compute-tile',
        'designed-core-elohim-compute-tile',
        'designed-foundations-compute-capacity-tokens',
        'designed-core-elohim-presence-badge',
      ]
    );
    assert.equal(groups.get('designed-core-elohim-compute-tile')?.length, 2);
  });
});

describe('suggestComponents', () => {
  it('suggests component prefixes containing the name, deduped', () => {
    const got = suggestComponents(index(), 'compute');
    assert.ok(got.includes('designed-core-elohim-compute-tile'));
    assert.ok(got.includes('default-core-elohim-compute-tile'));
    assert.equal(new Set(got).size, got.length);
  });
  it('respects the limit', () => {
    assert.ok(suggestComponents(index(), 'e', 2).length <= 2);
  });
});

describe('storiesForSheet', () => {
  it('selects story-type entries for the component across families', () => {
    const got = storiesForSheet(index(), 'elohim-compute-tile');
    assert.equal(got.length, 3);
    assert.ok(got.every(e => e.type === 'story'));
  });
  it('narrows by family', () => {
    const got = storiesForSheet(index(), 'elohim-compute-tile', 'designed');
    assert.equal(got.length, 2);
  });
  it('excludes docs entries', () => {
    const got = storiesForSheet(index(), 'compute-capacity-tokens');
    assert.equal(got.length, 0);
  });
});

describe('sheetHtml', () => {
  it('renders one labeled iframe per story, grouped by family, with grid cols', () => {
    const entries = storiesForSheet(index(), 'elohim-compute-tile');
    const html = sheetHtml({
      component: 'elohim-compute-tile',
      base: 'https://storybook.elohim.host',
      entries,
      cell: { width: 420, height: 320 },
      cols: 3,
    });
    assert.ok(
      html.includes(
        'iframe.html?id=designed-core-elohim-compute-tile--standard&viewMode=story'
      )
    );
    assert.equal((html.match(/<iframe /g) ?? []).length, 3);
    assert.ok(html.includes('<h2>default</h2>'));
    assert.ok(html.includes('<h2>designed</h2>'));
    assert.ok(html.includes('repeat(3, 420px)'));
    assert.ok(html.includes('<figcaption>Standard</figcaption>'));
  });
  it('escapes HTML in names', () => {
    const html = sheetHtml({
      component: 'x',
      base: 'https://s',
      entries: [entry('x--a', 'Default/x', '<b>&'), ],
      cell: { width: 100, height: 100 },
      cols: 1,
    });
    assert.ok(html.includes('&lt;b&gt;&amp;'));
  });
});
