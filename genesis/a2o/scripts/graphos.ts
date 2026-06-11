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
