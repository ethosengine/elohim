#!/usr/bin/env node
// Advisory only. Preserve the oracle's complete affected_by arrays, unlike depthOne.
import { spawnSync } from 'node:child_process';
import { readFileSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { resolveBritBin } from './gate-oracle.mjs';
import { loadManifests } from './manifest-utils.mjs';
import { projectsFromStale } from './graph-walker.mjs';
import { readCycleRows } from './gate-cost.mjs';
import { subjectFor } from './gate-cycle.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const TIMEOUT = 15_000;
/** A linked worktree's main checkout, or null. The oracle builds its graph from every manifest,
 *  pinned submodules included, and a worktree whose submodules are not checked out cannot. */
export function mainCheckout(root) {
  try {
    const m = /^gitdir: (.+)$/m.exec(readFileSync(resolve(root, '.git'), 'utf8'));
    return m && /[\\/]worktrees[\\/][^\\/]+$/.test(m[1].trim()) ? resolve(root, m[1].trim(), '../../..') : null;
  } catch { return null; }
}
function run(spawn, binary, args, root) {
  const r = spawn(binary, args, { cwd: root, encoding: 'utf8', timeout: TIMEOUT, maxBuffer: 8 * 1024 * 1024 });
  if (r?.error?.code === 'ETIMEDOUT') throw new Error('timeout after 15 s');
  if (r?.error) throw new Error(r.error.code || r.error.message);
  if (!r || r.status !== 0) throw new Error(`${binary} exited ${r?.status ?? 'without status'}: ${String(r?.stderr || '').trim().split('\n')[0]}`);
  return r.stdout;
}

export function affected(files, { spawn = spawnSync, root = ROOT, britBin = resolveBritBin(), manifests } = {}) {
  try {
    if (!britBin) throw new Error('no brit executable (BRIT_BIN or PATH)');
    let truncated = false;
    const expanded = new Set();
    for (const file of files.filter(Boolean)) {
      let directory = file.endsWith('/');
      try { directory ||= statSync(resolve(root, file)).isDirectory(); } catch { /* literal new file */ }
      if (directory || /[*?[]/.test(file)) {
        // Existing gate helpers use read-only ls-files. Brit help is unavailable on this device.
        const listed = run(spawn, 'git', ['ls-files', '-z', '--', file], root).split('\0').filter(Boolean);
        for (const path of listed) {
          if (expanded.size < 2000) expanded.add(path);
          else if (!expanded.has(path)) truncated = true;
        }
      } else if (expanded.size < 2000) expanded.add(file);
      else truncated = true;
    }
    const paths = [...expanded];
    const ask = at => JSON.parse(run(spawn, britBin,
      ['build', 'affected', '--repo', at, '--files', paths.join(',')], at));
    let parsed = { affected: [] };
    let graph = root;
    if (paths.length) {
      try { parsed = ask(root); } catch (first) {
        // Advisory only: the graph of the main checkout stands in, and the result says so.
        graph = mainCheckout(root);
        if (!graph) throw first;
        parsed = ask(graph);
      }
    }
    if (!Array.isArray(parsed.affected)) throw new Error('no affected list');
    // The documented JSON contract includes changedFile/upstreamNode reasons. Names alone
    // cannot establish direct ownership; retain them but report the missing reasons honestly.
    if (parsed.affected.some(s => !s?.qualified_name || !Array.isArray(s.affected_by))) {
      return { steps: parsed.affected, projects: [], fallback: 'dependency reasons unavailable', truncated };
    }
    const stale = new Map(parsed.affected.map(s => [s.qualified_name, s.affected_by.map(r =>
      r.kind === 'changedFile' ? `source: ${r.path}` : `upstream: ${r.upstream}`)]));
    const projects = projectsFromStale(manifests ?? loadManifests(graph), stale, paths)
      .map(p => ({ ...p, direct: p.reasons.some(r => r.startsWith('source: ') || r.startsWith('buildProcess: ')) }));
    return { steps: parsed.affected, projects, truncated, ...(graph === root ? {} : { graph }) };
  } catch (error) {
    return { steps: [], projects: [], fallback: error.message };
  }
}

/** Pure; secondsTable is a snapshot keyed by gate project, never a live lookup. */
export function impact(mine, theirs, secondsTable = {}) {
  if (mine.fallback || theirs.fallback) return [];
  const other = new Map(theirs.projects.map(p => [p.name, p]));
  return mine.projects.flatMap(p => {
    const q = other.get(p.name);
    if (!q || (!p.direct && !q.direct)) return [];
    const seconds = secondsTable[p.name];
    return [{ project: p.name, direction: p.direct && q.direct ? 'shared'
      : q.direct ? 'upstream-of-me' : 'downstream-of-me',
    seconds: Number.isFinite(seconds) && seconds >= 0 ? seconds : null }];
  });
}

export function localSeconds(root, projects) {
  try {
    const path = resolve(root, '.eprfs/status/flows.jsonl');
    // The sidecar holds every flow (17 MB on shem, 2026-11-22). Only the measure's own lines are
    // parsed; past the cap the cost is unknown, never invented.
    if (statSync(path).size > 64 * 1024 * 1024) return {};
    const { rows } = readCycleRows(path, (f, enc) => readFileSync(f, enc).split('\n')
      .filter(line => line.includes('gate-cycle-seconds@1')).join('\n'));
    const anchors = projects.map(p => [p.name, subjectFor(p, root)]);
    const table = {};
    for (const row of rows) {
      for (const [name, anchor] of anchors) {
        if (row.project === name || (row.project === undefined && anchor !== '.' &&
            row.subject === anchor && anchors.filter(([, a]) => a === anchor).length === 1)) table[name] = row.value;
      }
    }
    return table;
  } catch { return {}; }
}

export function main(argv, options = {}, print = console.log) {
  const args = [...argv];
  const json = args.includes('--json');
  if (json) args.splice(args.indexOf('--json'), 1);
  const values = {};
  while (args.length) {
    const key = args.shift();
    if (!['--mine', '--theirs'].includes(key) || !args[0] || args[0].startsWith('--')) {
      print('usage: gate-impact --mine a,b --theirs c,d [--json]'); return 2;
    }
    values[key] = args.shift().split(',').filter(Boolean);
  }
  if (!values['--mine']?.length || !values['--theirs']?.length) {
    print('usage: gate-impact --mine a,b --theirs c,d [--json]'); return 2;
  }
  const mine = affected(values['--mine'], options);
  const theirs = mine.fallback ? mine : affected(values['--theirs'], options);
  const fallback = mine.fallback || theirs.fallback;
  const truncated = Boolean(mine.truncated || theirs.truncated);
  const rows = impact(mine, theirs, fallback ? {} : localSeconds(mine.graph || options.root || ROOT, mine.projects));
  if (json) print(JSON.stringify({ rows, fallback: fallback || null, truncated }));
  else if (fallback) print(`path-fallback: ${fallback}`);
  else {
    if (truncated) print('tracked listing truncated at 2000 files');
    if (!rows.length) print('no shared gates');
    for (const r of rows) print(`${r.project}: ${r.direction}; ${r.seconds === null ? 'unknown' : `~${r.seconds}s`}`);
  }
  return 0;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) process.exitCode = main(process.argv.slice(2));
