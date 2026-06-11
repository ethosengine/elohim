# graphos-look Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `pnpm graphos {list,story,sheet}` in genesis/a2o — enumerate and render graphos (component library + design guide) stories from a Storybook index, so visual cues are a first-class input to frontend work.

**Architecture:** New pure-function library `scripts/lib/graphos-stories.ts` (index parsing, segment-aligned component matching, grouping, suggestions, sheet HTML generation) + thin CLI `scripts/graphos.ts` that fetches `index.json` and delegates rendering to the exported `runLook()` from `scripts/look.ts`. Sheets are a generated `sheet.html` iframe grid rendered via `file://` in one full-page screenshot. Zero new dependencies.

**Tech Stack:** TypeScript (tsx), node:test + node:assert/strict, Playwright via existing `runLook()`/`PlaywrightDevice`. Spec: `genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md`.

**Worker context (read first):**
- All commands run from `/projects/elohim/genesis/a2o` unless stated otherwise.
- The pnpm warning `The "pnpm" field in package.json is no longer read` is benign noise — ignore it.
- SHARED WORKTREE: other sessions co-commit here. NEVER `git add -A` / `git add .` — stage ONLY the files this plan names. Commit; never push.
- `index.json` v5 entry shape (deployed `https://storybook.elohim.host/index.json`, 483 entries):
  `{ id: "designed-core-elohim-compute-tile--standard", title: "Designed/Core/elohim-compute-tile", name: "Standard", type: "story"|"docs", importPath, exportName, tags }`.
  Story id = `<family>-<group>-<component>--<cell-slug>`; family = first `title` segment lowercased (`default`, `designed`, `i`…`iv` narrative, `foundations`).
- `runLook(opts)` (exported from `scripts/look.ts`) accepts `{ url, as?, doorway?, waitTestid?, out?, viewport? }`, writes `reports/look/<out|latest>/{shot.png,capture.json}`, returns `LookResult` with `ok`, `shotPath`, `capturePath`. It navigates with `waitUntil: 'networkidle'` (spans child frames) and records 4xx/5xx subframe responses into `httpErrors`.

---

### Task 1: Pure library `scripts/lib/graphos-stories.ts` (TDD)

**Files:**
- Test: `genesis/a2o/scripts/__tests__/graphos-stories.test.ts`
- Create: `genesis/a2o/scripts/lib/graphos-stories.ts`

- [ ] **Step 1: Write the failing test**

Create `genesis/a2o/scripts/__tests__/graphos-stories.test.ts`:

```typescript
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
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/graphos-stories.test.ts`
Expected: FAIL — `Cannot find module '../lib/graphos-stories.js'`.

- [ ] **Step 3: Write the implementation**

Create `genesis/a2o/scripts/lib/graphos-stories.ts`:

```typescript
/**
 * Pure functions over a Storybook v5 index — story-id conventions, component
 * matching, grouping, and composite-sheet HTML generation for `graphos`.
 *
 * Story ids follow `<family>-<group>-<component>--<cell>`; family is the
 * first `title` segment lowercased (default | designed | i..iv | foundations).
 * Spec: genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md
 */

export interface StoryEntry {
  id: string;
  title: string;
  name: string;
  type: 'story' | 'docs';
}

export interface StorybookIndex {
  v: number;
  entries: Record<string, StoryEntry>;
}

/** Id prefix before `--`, e.g. `designed-core-elohim-compute-tile`. */
export function componentPrefix(id: string): string {
  const i = id.indexOf('--');
  return i === -1 ? id : id.slice(0, i);
}

/** Segment-aligned: prefix === component or prefix ends with `-<component>`. */
export function matchesComponent(id: string, component: string): boolean {
  const prefix = componentPrefix(id);
  return prefix === component || prefix.endsWith(`-${component}`);
}

export function familyOf(entry: StoryEntry): string {
  return (entry.title.split('/')[0] ?? '').toLowerCase();
}

export function listStories(index: StorybookIndex, filter?: string): StoryEntry[] {
  const all = Object.values(index.entries);
  if (!filter) return all;
  const f = filter.toLowerCase();
  return all.filter(e => e.id.includes(f) || e.title.toLowerCase().includes(f));
}

export function groupByComponent(entries: StoryEntry[]): Map<string, StoryEntry[]> {
  const groups = new Map<string, StoryEntry[]>();
  for (const e of entries) {
    const key = componentPrefix(e.id);
    const list = groups.get(key);
    if (list) list.push(e);
    else groups.set(key, [e]);
  }
  return groups;
}

export function suggestComponents(index: StorybookIndex, name: string, limit = 5): string[] {
  const n = name.toLowerCase();
  const seen = new Set<string>();
  for (const e of Object.values(index.entries)) {
    const prefix = componentPrefix(e.id);
    if (prefix.toLowerCase().includes(n)) seen.add(prefix);
    if (seen.size >= limit) break;
  }
  return [...seen];
}

export function storiesForSheet(
  index: StorybookIndex,
  component: string,
  family?: string
): StoryEntry[] {
  return Object.values(index.entries).filter(
    e =>
      e.type === 'story' &&
      matchesComponent(e.id, component) &&
      (!family || familyOf(e) === family)
  );
}

export interface SheetOptions {
  component: string;
  base: string;
  entries: StoryEntry[];
  cell: { width: number; height: number };
  cols: number;
}

/**
 * Self-contained iframe-grid page; rendered via file:// in one full-page
 * screenshot. A file:// parent loading http(s) iframes is permitted in
 * Chromium (mixed-content rules block the reverse case).
 */
export function sheetHtml(opts: SheetOptions): string {
  const { component, base, entries, cell, cols } = opts;
  const byFamily = new Map<string, StoryEntry[]>();
  for (const e of entries) {
    const fam = familyOf(e);
    const list = byFamily.get(fam);
    if (list) list.push(e);
    else byFamily.set(fam, [e]);
  }
  const sections = [...byFamily.entries()]
    .map(([family, stories]) => {
      const cells = stories
        .map(
          e => `<figure class="cell">
  <figcaption>${escapeHtml(e.name)}</figcaption>
  <iframe src="${escapeHtml(base)}/iframe.html?id=${encodeURIComponent(e.id)}&viewMode=story" width="${cell.width}" height="${cell.height}" loading="eager"></iframe>
</figure>`
        )
        .join('\n');
      return `<section>\n<h2>${escapeHtml(family)}</h2>\n<div class="grid">\n${cells}\n</div>\n</section>`;
    })
    .join('\n');
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>graphos sheet: ${escapeHtml(component)}</title>
<style>
  body { margin: 0; padding: 8px; font-family: system-ui, sans-serif; background: #fff; }
  h1 { font-size: 16px; margin: 4px 0; }
  h2 { margin: 12px 0 4px; font-size: 14px; }
  .grid { display: grid; grid-template-columns: repeat(${cols}, ${cell.width}px); gap: 8px; }
  .cell { margin: 0; }
  .cell figcaption { font-size: 11px; color: #555; padding: 2px 0; }
  .cell iframe { border: 1px solid #ddd; display: block; }
</style>
</head>
<body>
<h1>${escapeHtml(component)}</h1>
${sections}
</body>
</html>`;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/graphos-stories.test.ts`
Expected: PASS (all describe blocks green). Then run the full unit suite to be sure nothing else broke: `pnpm test:unit` → all pass.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/scripts/lib/graphos-stories.ts genesis/a2o/scripts/__tests__/graphos-stories.test.ts
git commit -m "feat(a2o): graphos-stories pure lib — index parsing, segment-aligned matching, sheet HTML

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: CLI `scripts/graphos.ts` (arg parsing TDD, then verbs)

**Files:**
- Test: `genesis/a2o/scripts/__tests__/graphos-cli.test.ts`
- Create: `genesis/a2o/scripts/graphos.ts`

- [ ] **Step 1: Write the failing arg-parser test**

Create `genesis/a2o/scripts/__tests__/graphos-cli.test.ts`:

```typescript
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { parseGraphosArgs } from '../graphos.js';

describe('parseGraphosArgs', () => {
  it('parses list with optional filter', () => {
    const cmd = parseGraphosArgs(['list', 'compute']);
    assert.equal(cmd.verb, 'list');
    assert.equal(cmd.arg, 'compute');
    assert.equal(cmd.base, 'https://storybook.elohim.host');
  });
  it('parses list with no filter', () => {
    assert.equal(parseGraphosArgs(['list']).arg, undefined);
  });
  it('requires an argument for story and sheet', () => {
    assert.throws(() => parseGraphosArgs(['story']));
    assert.throws(() => parseGraphosArgs(['sheet']));
  });
  it('parses story flags', () => {
    const cmd = parseGraphosArgs([
      'story', 'designed-core-elohim-compute-tile--standard',
      '--docs', '--base', 'http://localhost:6006/', '--out', 'my-slug',
      '--viewport', '800x600',
    ]);
    assert.equal(cmd.verb, 'story');
    assert.equal(cmd.docs, true);
    assert.equal(cmd.base, 'http://localhost:6006'); // trailing slash stripped
    assert.equal(cmd.out, 'my-slug');
    assert.deepEqual(cmd.viewport, { width: 800, height: 600 });
  });
  it('parses sheet flags with defaults', () => {
    const cmd = parseGraphosArgs(['sheet', 'elohim-compute-tile']);
    assert.deepEqual(cmd.cell, { width: 420, height: 320 });
    assert.equal(cmd.cols, 3);
    assert.equal(cmd.family, undefined);
  });
  it('parses --family and validates it', () => {
    assert.equal(
      parseGraphosArgs(['sheet', 'x', '--family', 'designed']).family,
      'designed'
    );
    assert.throws(() => parseGraphosArgs(['sheet', 'x', '--family', 'bogus']));
  });
  it('parses --cell and --cols', () => {
    const cmd = parseGraphosArgs(['sheet', 'x', '--cell', '300x200', '--cols', '4']);
    assert.deepEqual(cmd.cell, { width: 300, height: 200 });
    assert.equal(cmd.cols, 4);
  });
  it('rejects unknown verbs and flags', () => {
    assert.throws(() => parseGraphosArgs(['render', 'x']));
    assert.throws(() => parseGraphosArgs(['list', '--bogus']));
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/graphos-cli.test.ts`
Expected: FAIL — `Cannot find module '../graphos.js'`.

- [ ] **Step 3: Write the CLI**

Create `genesis/a2o/scripts/graphos.ts`:

```typescript
/**
 * `graphos` — render the design guide & component library for agent eyes.
 *
 *   list  [filter]     enumerate story ids grouped by component (no browser)
 *   story <story-id>   render one story (viewMode auto-derived; --docs forces)
 *   sheet <component>  composite cell/theme matrix via a generated iframe grid
 *
 * Base defaults to the deployed storybook (graphos as merged to dev); pass
 * `--base http://localhost:6006` with a local `pnpm storybook` running
 * (app/elohim-library) to see in-branch work. Artifacts land in
 * reports/look/<slug>/ — same convention as `look`, visible to the operator
 * via `pnpm reports:serve`.
 * Spec: genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md
 */

import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import {
  componentPrefix,
  groupByComponent,
  listStories,
  sheetHtml,
  storiesForSheet,
  suggestComponents,
  type StorybookIndex,
} from './lib/graphos-stories.js';
import { runLook } from './look.js';

const DEFAULT_BASE = 'https://storybook.elohim.host';
const REPORTS_DIR = 'reports/look';

const USAGE = `Usage:
  graphos list  [filter]                                 [--base <url>]
  graphos story <story-id> [--docs] [--viewport WxH]     [--base <url>] [--out <slug>]
  graphos sheet <component> [--family designed|default]
                [--cell WxH] [--cols N]                  [--base <url>] [--out <slug>]`;

export interface GraphosCommand {
  verb: 'list' | 'story' | 'sheet';
  arg?: string;
  base: string;
  out?: string;
  docs: boolean;
  family?: 'designed' | 'default';
  cell: { width: number; height: number };
  cols: number;
  viewport?: { width: number; height: number };
}

function parseWxH(val: string | undefined, flag: string): { width: number; height: number } {
  const m = /^(\d+)x(\d+)$/.exec(val ?? '');
  if (!m) throw new Error(`${flag} expects WxH (e.g. 420x320), got: ${val}`);
  return { width: Number(m[1]), height: Number(m[2]) };
}

export function parseGraphosArgs(argv: string[]): GraphosCommand {
  const [verb, ...rest] = argv;
  if (verb !== 'list' && verb !== 'story' && verb !== 'sheet') throw new Error(USAGE);
  const cmd: GraphosCommand = {
    verb,
    base: DEFAULT_BASE,
    docs: false,
    cell: { width: 420, height: 320 },
    cols: 3,
  };
  const args = [...rest];
  if (args[0] && !args[0].startsWith('--')) cmd.arg = args.shift();
  if ((verb === 'story' || verb === 'sheet') && !cmd.arg) throw new Error(USAGE);
  for (let i = 0; i < args.length; i++) {
    const flag = args[i];
    const val = args[i + 1];
    switch (flag) {
      case '--base':
        if (!val) throw new Error(`--base expects a URL`);
        cmd.base = val.replace(/\/+$/, '');
        i++;
        break;
      case '--out':
        cmd.out = val;
        i++;
        break;
      case '--docs':
        cmd.docs = true;
        break;
      case '--family':
        if (val !== 'designed' && val !== 'default')
          throw new Error(`--family expects designed|default, got: ${val}`);
        cmd.family = val;
        i++;
        break;
      case '--cell':
        cmd.cell = parseWxH(val, '--cell');
        i++;
        break;
      case '--cols': {
        const n = Number(val);
        if (!Number.isInteger(n) || n < 1) throw new Error(`--cols expects a positive integer`);
        cmd.cols = n;
        i++;
        break;
      }
      case '--viewport':
        cmd.viewport = parseWxH(val, '--viewport');
        i++;
        break;
      default:
        throw new Error(`Unknown flag: ${flag}\n${USAGE}`);
    }
  }
  return cmd;
}

async function fetchIndex(base: string): Promise<StorybookIndex> {
  const url = `${base}/index.json`;
  let res: Response;
  try {
    res = await fetch(url);
  } catch (e) {
    throw new Error(unreachableMsg(base, (e as Error).message));
  }
  if (!res.ok) throw new Error(unreachableMsg(base, `HTTP ${res.status}`));
  return (await res.json()) as StorybookIndex;
}

function unreachableMsg(base: string, detail: string): string {
  const local = /localhost|127\.0\.0\.1/.test(base);
  const hint = local
    ? `No storybook at ${base} — start it with:\n  cd app/elohim-library && pnpm storybook`
    : `Storybook at ${base} is unreachable (site or network down).`;
  return `${hint}\n(${detail} fetching ${base}/index.json)`;
}

function cmdList(index: StorybookIndex, filter?: string): void {
  const groups = groupByComponent(listStories(index, filter));
  let total = 0;
  for (const [prefix, entries] of groups) {
    total += entries.length;
    const names = entries.map(e => (e.type === 'docs' ? `${e.name}[docs]` : e.name));
    console.log(`${prefix}  (${entries.length})`);
    console.log(`    ${names.join(' · ')}`);
  }
  console.log(`\n${total} entries in ${groups.size} components${filter ? ` matching "${filter}"` : ''}`);
}

async function cmdStory(index: StorybookIndex, cmd: GraphosCommand): Promise<boolean> {
  const id = cmd.arg as string;
  const entry = index.entries[id];
  if (!entry) {
    const near = suggestComponents(index, componentPrefix(id));
    throw new Error(
      `Unknown story id: ${id}` +
        (near.length ? `\nNear matches:\n  ${near.join('\n  ')}` : `\nTry: pnpm graphos list <filter>`)
    );
  }
  const mode = cmd.docs || entry.type === 'docs' ? 'docs' : 'story';
  const url = `${cmd.base}/iframe.html?id=${encodeURIComponent(id)}&viewMode=${mode}`;
  const result = await runLook({ url, out: cmd.out ?? id, viewport: cmd.viewport });
  console.log(result.shotPath);
  console.log(result.capturePath);
  return result.ok;
}

async function cmdSheet(index: StorybookIndex, cmd: GraphosCommand): Promise<boolean> {
  const component = cmd.arg as string;
  const entries = storiesForSheet(index, component, cmd.family);
  if (entries.length === 0) {
    const near = suggestComponents(index, component);
    throw new Error(
      `No stories match component: ${component}` +
        (cmd.family ? ` (family ${cmd.family})` : '') +
        (near.length ? `\nNear matches:\n  ${near.join('\n  ')}` : '')
    );
  }
  const slug = cmd.out ?? `sheet-${component}`;
  const outDir = resolve(REPORTS_DIR, slug);
  await mkdir(outDir, { recursive: true });
  const sheetPath = join(outDir, 'sheet.html');
  await writeFile(
    sheetPath,
    sheetHtml({ component, base: cmd.base, entries, cell: cmd.cell, cols: cmd.cols })
  );
  // Width: cols * cell + grid gaps + body padding; full-page shot covers height.
  const width = cmd.cols * cmd.cell.width + (cmd.cols - 1) * 8 + 16 + 2 * cmd.cols;
  const result = await runLook({
    url: pathToFileURL(sheetPath).href,
    out: slug,
    viewport: { width, height: 800 },
  });
  console.log(result.shotPath);
  console.log(result.capturePath);
  console.log(sheetPath);
  return result.ok;
}

async function main(): Promise<void> {
  let cmd: GraphosCommand;
  try {
    cmd = parseGraphosArgs(process.argv.slice(2));
  } catch (e) {
    console.error((e as Error).message);
    process.exit(2);
  }
  const index = await fetchIndex(cmd.base);
  if (cmd.verb === 'list') {
    cmdList(index, cmd.arg);
    process.exit(0);
  }
  const ok = cmd.verb === 'story' ? await cmdStory(index, cmd) : await cmdSheet(index, cmd);
  process.exit(ok ? 0 : 1);
}

// Run only when invoked directly (not when imported by tests).
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch(e => {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(2);
  });
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/graphos-cli.test.ts scripts/__tests__/graphos-stories.test.ts`
Expected: PASS. Then `pnpm test:unit` → all pass.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/scripts/graphos.ts genesis/a2o/scripts/__tests__/graphos-cli.test.ts
git commit -m "feat(a2o): graphos CLI — list/story/sheet verbs over storybook index via runLook

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Wire `pnpm graphos` + a2o CLAUDE.md tools bullet

**Files:**
- Modify: `genesis/a2o/package.json` (scripts block, next to `"look"`)
- Modify: `genesis/a2o/CLAUDE.md` (Tools section, after the `look` bullet)

- [ ] **Step 1: Add the script entry**

In `genesis/a2o/package.json`, the scripts block currently has (line ~32):

```json
    "look": "tsx scripts/look.ts",
```

Add directly after it:

```json
    "graphos": "tsx scripts/graphos.ts",
```

- [ ] **Step 2: Add the CLAUDE.md tools bullet**

In `genesis/a2o/CLAUDE.md`, in the `## Tools` section, directly after the `**Render & see (\`look\`)**` bullet, add:

```markdown
- **Design-guide & component-library eyes (`graphos`)**: `pnpm graphos list [filter]` / `pnpm graphos story <story-id> [--docs]` / `pnpm graphos sheet <component> [--family designed|default] [--cell WxH] [--cols N]` — enumerate and render graphos stories from the deployed Storybook (`https://storybook.elohim.host`, latest dev) or a locally running one (`--base http://localhost:6006`; start with `cd app/elohim-library && pnpm storybook`). `sheet` writes a composite cell/theme matrix (Library A `default` vs Library B `designed` sections) as ONE `shot.png` plus the live-iframe `sheet.html`, in `reports/look/<slug>/`. Same capture + `reports:serve` conventions as `look`. Spec: `genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md`.
```

- [ ] **Step 3: Verify the script resolves**

Run: `cd /projects/elohim/genesis/a2o && pnpm graphos 2>&1 | head -5`
Expected: the USAGE block on stderr, exit code 2 (no verb given). No module errors.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/package.json genesis/a2o/CLAUDE.md
git commit -m "feat(a2o): wire pnpm graphos + CLAUDE.md tools bullet

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: Smoke against the deployed storybook (visual verification)

**Files:** none created (report artifacts only; fixes if smoke finds bugs)

- [ ] **Step 1: list**

Run: `cd /projects/elohim/genesis/a2o && pnpm graphos list elohim-compute-tile`
Expected: two component groups (`default-core-elohim-compute-tile`, `designed-core-elohim-compute-tile`), ~12 cells each, total line at the end.

- [ ] **Step 2: story**

Run: `pnpm graphos story designed-core-elohim-compute-tile--standard --viewport 800x600`
Expected: prints `reports/look/designed-core-elohim-compute-tile--standard/shot.png` + capture path, exit 0. Read the shot.png — expect the warm-earth compute tile ("Matthew's household / 5 GB of 15 GB free").

- [ ] **Step 3: sheet**

Run: `pnpm graphos sheet elohim-compute-tile`
Expected: prints shot/capture/sheet.html paths. Read the shot.png — expect TWO labeled sections (`default` then `designed`), each a 3-column grid of ~12 captioned cells. If iframes render blank: check capture.json `httpErrors` first (a story erroring inside its iframe is evidence, not a sheet failure); if ALL are blank, the `file://` + networkidle path needs investigation — report findings, do not paper over.

- [ ] **Step 4: error paths**

Run: `pnpm graphos sheet bogus-component; echo "exit=$?"`
Expected: `No stories match component: bogus-component` (+ near matches or nothing), exit=2.
Run: `pnpm graphos list --base http://localhost:6006 2>&1 | head -3; echo "exit=$?"`
Expected (no local storybook running): the "start it with: cd app/elohim-library && pnpm storybook" hint, exit=2.

- [ ] **Step 5: Commit (only if smoke forced code fixes)**

```bash
cd /projects/elohim
git add genesis/a2o/scripts/graphos.ts genesis/a2o/scripts/lib/graphos-stories.ts genesis/a2o/scripts/__tests__/graphos-stories.test.ts genesis/a2o/scripts/__tests__/graphos-cli.test.ts
git commit -m "fix(a2o): graphos smoke findings

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: Root CLAUDE.md available-rails pointer block + spec touchpoint + memory

**Files:**
- Modify: `/projects/elohim/CLAUDE.md` (new subsection under `## Development Workflow`, after `### Exploration Fallback`)
- Modify: `genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md` (Touchpoints section)
- Modify: `/projects/.claude-config/projects/-projects-elohim/memory/feedback_frontend_review_eyes_first.md` + `MEMORY.md`

- [ ] **Step 1: Add the CLAUDE.md subsection**

In `/projects/elohim/CLAUDE.md`, after the `### Exploration Fallback` subsection (before `### Story Harvest…`), insert. Tone: what is AVAILABLE, not what to do:

```markdown
### Frontend Eyes (available rails)

Frontend review/refinement is eyes-first: render before reading source. Rails available to any agent (run from `genesis/a2o`):

- **`pnpm look <url> [--as <FixtureHuman>]`** — render any URL headless; writes `reports/look/<slug>/{shot.png,capture.json}` (console/pageerror/failed-request/httpError capture). Deployed app: `https://doorway-alpha.elohim.host`.
- **`pnpm graphos {list|story|sheet}`** — enumerate/render the graphos component library + design guide from the deployed Storybook (`https://storybook.elohim.host`, latest dev) or a local `pnpm storybook` (`--base http://localhost:6006`). `sheet <component>` = the full cell/theme matrix (Library A vs Library B sections) in ONE composite image. Design: `genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md`.
- **`pnpm reports:serve`** (port 4201) — the operator sees the same artifacts the agent reads (symmetric vision).
- Can't-find ≠ never-implemented: if a described view doesn't render, suspect present reachability (capture.json `httpErrors`, routes, env) before concluding absence.
```

- [ ] **Step 2: Seal the managed surfaces**

```bash
cd /projects/elohim
python3 .claude/scripts/memory-kit/cite-gen.py --seal CLAUDE.md
python3 .claude/scripts/memory-kit/cite-gen.py --seal genesis/a2o/CLAUDE.md
```
Expected: both end with `✅ gate: all cites content-addressed + resolvable`. If the tool rewrites the doc pointers into envelope form, keep the tool's output verbatim.

- [ ] **Step 3: Update the spec Touchpoints**

In `genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md`, in `## Touchpoints`, replace the `genesis/a2o/CLAUDE.md` bullet block with:

```markdown
- `genesis/a2o/package.json` — one script line: `"graphos": "tsx scripts/graphos.ts"`.
- `genesis/a2o/CLAUDE.md` — Tools bullet beside the `look` entry.
- Root `CLAUDE.md` — "Frontend Eyes (available rails)" subsection: cited pointers
  so any agent discovers what is AVAILABLE (look, graphos, storybook, reports:serve)
  when planning frontend design changes — capability discovery, not prescription.
- Memory `frontend-review-eyes-first` — names `pnpm graphos` for the
  Storybook/design-guide surfaces.
```

Then re-seal: `python3 .claude/scripts/memory-kit/cite-gen.py --seal genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md`

- [ ] **Step 4: Update memory (MAIN SESSION ONLY — subagents skip this step; it lives outside the repo)**

In `feedback_frontend_review_eyes_first.md`, replace the Storybook bullet (item 2 under How to apply) with one naming `pnpm graphos list|story|sheet` instead of raw `iframe.html` URL construction, and the graphos bullet (item 3) likewise. Update the matching `MEMORY.md` index line. Re-seal the memory file.

- [ ] **Step 5: Run full unit suite once more, then commit**

```bash
cd /projects/elohim/genesis/a2o && pnpm test:unit
cd /projects/elohim
git add CLAUDE.md genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md
git commit -m "docs: frontend-eyes available-rails pointers in gospel CLAUDE.md + graphos spec touchpoints

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Plan self-review (done at write time)

- **Spec coverage:** list/story/sheet verbs (T2), base-URL-agnostic (T2 `--base`), segment-aligned matching (T1), viewMode auto-derivation (T2 cmdStory), sheet mechanics incl. file:// + networkidle + subframe capture (T1 sheetHtml, T2 cmdSheet), error handling incl. deployed-vs-local hint + nearest-match + pre-browser validation (T2, smoked in T4), unit tests for pure functions (T1, T2), package.json + a2o CLAUDE.md (T3), root CLAUDE.md pointers (T5 — operator addition), memory (T5). Out-of-scope items: none planned. ✅
- **Placeholders:** none — every step carries full code/commands/expected output. ✅
- **Type consistency:** `StoryEntry`/`StorybookIndex`/`GraphosCommand` defined once, imported elsewhere; `runLook` signature matches `look.ts` (`{url, out, viewport}` subset). ✅
