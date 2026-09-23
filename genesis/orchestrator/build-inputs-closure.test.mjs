/**
 * Build inputs closure — every Rust build unit watches its whole path-dep closure
 *
 * A build unit's watch globs (`inputs.sources`) decide when the orchestrator and
 * the local gate rebuild it. Its Cargo path-dependency closure decides what its
 * binary is actually made of. When the two disagree, a change to a dependency
 * crate matches no glob, nothing rebuilds, and the fleet runs a STALE binary with
 * no red anywhere. storage-build-inputs.test.mjs holds that line for
 * cargo-build-storage alone; this test holds it for every manifest.
 *
 * What counts as a unit
 *   - a manifest step whose sources name a Rust crate specifically (a crate dir,
 *     its src/, its Cargo.toml, or a pattern over *.rs / Cargo.toml), or which a
 *     gate project with a `run.cargo` block triggers;
 *   - a gate project with a `run.cargo` block (reported as `gate:<name>`), whose
 *     watch surface is its own `inputs.sources` plus its trigger steps' sources.
 *
 * Which crates a unit builds (its roots)
 *   - the crates its globs name specifically (see nameKind); only when there are
 *     none do crates a wider glob merely sweeps past (crates/**) stand in;
 *   - for a cargo gate project, also the crates at or under its `dir`; for
 *     `dir: "."`, its `cargo.workspace` when that path is itself a package;
 *   - of those, a crate another one already reaches through its closure is a
 *     watched dependency, not a separate root (see topRoots).
 *
 * The closure of a root
 *   - `path = "…"` dependencies, recursively (dependencies of every kind,
 *     [patch] entries and [workspace.dependencies]); git and registry deps have no
 *     path and are ignored;
 *   - `name.workspace = true` dependencies, resolved through the governing
 *     workspace root's [workspace.dependencies];
 *   - the members of a workspace root the unit builds;
 *   - the governing workspace root's Cargo.toml (for a root, or for a dependency
 *     that inherits from it) and Cargo.lock (for a root), when the workspace
 *     root lies outside the crate — cargo reads both, so a lockfile bump there
 *     is a dependency change too;
 *   - a path inside a git submodule is not followed: the gitlink itself joins
 *     the closure, and only a glob matching its bare path covers it (that is the
 *     only path a submodule pointer move shows as changed).
 *
 * Coverage reuses the storage test's semantics: a directory is covered when a
 * glob's static part lies within it or equals it (partial coverage counts), or
 * when the directory lies under a glob's static base and the glob matches a file
 * in it; a file or gitlink is covered when a glob matches it exactly.
 *
 * Fixing a failure means ADDING globs to the named step's inputs.sources (for a
 * gate project with inline inputs, to its gate inputs.sources) — never removing
 * a dependency from the closure's view.
 *
 * Run:
 *   node --test genesis/orchestrator/build-inputs-closure.test.mjs
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, posix, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import picomatch from 'picomatch';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');

/** Directory names never walked: build output, vendored installs, other checkouts. */
const SKIP_NAMES = new Set(['node_modules', 'target', '.worktrees', '.git']);
/** Repo-relative trees never walked: runtime state, research clones, test reports. */
const SKIP_PATHS = new Set(['genesis/local-dev', 'genesis/research/repos', 'genesis/a2o/reports']);

const abs = rel => resolve(root, rel);
const parentOf = rel => (rel.includes('/') ? rel.slice(0, rel.lastIndexOf('/')) : '');
const under = (path, dir) => dir === '' || path === dir || path.startsWith(`${dir}/`);

/** Submodule paths from .gitmodules — the gitlinks of this tree. */
const SUBMODULES = (() => {
  const file = abs('.gitmodules');
  if (!existsSync(file)) return [];
  return [...readFileSync(file, 'utf8').matchAll(/^\s*path\s*=\s*(\S+)\s*$/gm)].map(m => m[1]);
})();
const submoduleOf = path => SUBMODULES.find(s => under(path, s)) ?? null;

/** Walk the tree (skipping SKIP_* and dot-dirs), calling onFile(relPath) for each file. */
function walk(relDir, onFile, { intoSubmodules = false } = {}) {
  let entries;
  try {
    entries = readdirSync(abs(relDir || '.'), { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const rel = relDir ? `${relDir}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      if (entry.name.startsWith('.') || SKIP_NAMES.has(entry.name) || SKIP_PATHS.has(rel)) continue;
      if (!intoSubmodules && SUBMODULES.includes(rel)) continue;
      walk(rel, onFile, { intoSubmodules });
    } else if (entry.isFile()) {
      onFile(rel);
    }
  }
}

function discoverManifests() {
  const found = [];
  walk('', rel => rel.endsWith('/build-manifest.json') && found.push(rel), { intoSubmodules: true });
  return found.sort().map(path => ({ path, content: JSON.parse(readFileSync(abs(path), 'utf8')) }));
}

/** Every crate dir outside submodules (a dir holding a Cargo.toml, package or workspace). */
const CRATES = (() => {
  const dirs = [];
  walk('', rel => rel.endsWith('/Cargo.toml') && dirs.push(parentOf(rel)));
  return dirs.sort();
})();
const CRATE_SET = new Set(CRATES);

const fileCache = new Map();
/** Files on disk under a dir (build output and dot-dirs excluded). */
function filesUnder(dir) {
  if (!fileCache.has(dir)) {
    const files = [];
    walk(dir, rel => files.push(rel), { intoSubmodules: true });
    fileCache.set(dir, files);
  }
  return fileCache.get(dir);
}

// ── Cargo.toml reading ──────────────────────────────────────────────

/** Split a Cargo.toml into [header] → body sections (array-of-table headers included). */
function sections(text) {
  const out = [];
  let current = { header: '', body: '' };
  for (const line of text.split('\n')) {
    const h = line.match(/^\s*\[\[?\s*([^\]]+?)\s*\]\]?\s*(?:#.*)?$/);
    if (h) {
      out.push(current);
      current = { header: h[1].replace(/\s+/g, ''), body: '' };
    } else {
      current.body += `${line}\n`;
    }
  }
  out.push(current);
  return out;
}

const pathValues = text => [...text.matchAll(/\bpath\s*=\s*["']([^"']+)["']/g)].map(m => m[1]);
const stringArray = (body, key) => {
  const m = body.match(new RegExp(`^\\s*${key}\\s*=\\s*\\[([\\s\\S]*?)\\]`, 'm'));
  return m ? [...m[1].matchAll(/["']([^"']+)["']/g)].map(x => x[1]) : [];
};

const cargoCache = new Map();
function cargo(crateDir) {
  if (cargoCache.has(crateDir)) return cargoCache.get(crateDir);
  const text = readFileSync(abs(`${crateDir}/Cargo.toml`), 'utf8');
  const secs = sections(text);
  const sec = name => secs.find(s => s.header === name);
  const pkg = sec('package');
  const ws = sec('workspace');

  const workspaceDeps = new Map();
  const wsDeps = sec('workspace.dependencies');
  if (wsDeps) {
    for (const m of wsDeps.body.matchAll(/^\s*([A-Za-z0-9_-]+)\s*=\s*\{([^}]*)\}/gm)) {
      const p = pathValues(m[2])[0];
      if (p) workspaceDeps.set(m[1], p);
    }
  }
  for (const s of secs) {
    const m = s.header.match(/^workspace\.dependencies\.([A-Za-z0-9_-]+)$/);
    const p = m && pathValues(s.body)[0];
    if (p) workspaceDeps.set(m[1], p);
  }

  // Dependency names inherited with `workspace = true` (any dependency table).
  const inheritedDeps = new Set();
  for (const s of secs) {
    if (/(^|\.)(dev-|build-)?dependencies$/.test(s.header)) {
      for (const m of s.body.matchAll(/^\s*([A-Za-z0-9_-]+)\s*(?:\.\s*workspace\s*=\s*true|=\s*\{[^}]*\bworkspace\s*=\s*true)/gm)) {
        inheritedDeps.add(m[1]);
      }
    }
    const table = s.header.match(/(?:^|\.)(?:dev-|build-)?dependencies\.([A-Za-z0-9_-]+)$/);
    if (table && /^\s*workspace\s*=\s*true/m.test(s.body)) inheritedDeps.add(table[1]);
  }

  const info = {
    hasPackage: Boolean(pkg),
    isWorkspace: Boolean(ws),
    explicitWorkspace: pkg?.body.match(/^\s*workspace\s*=\s*["']([^"']+)["']/m)?.[1] ?? null,
    members: ws ? stringArray(ws.body, 'members') : [],
    exclude: ws ? stringArray(ws.body, 'exclude') : [],
    inherits: /\bworkspace\s*=\s*true/.test(text),
    allPaths: pathValues(text),
    patchPaths: secs.filter(s => s.header.startsWith('patch.')).flatMap(s => pathValues(s.body)),
    workspaceDeps,
    inheritedDeps,
  };
  cargoCache.set(crateDir, info);
  return info;
}

const join = (base, rel) => posix.normalize(base ? `${base}/${rel}` : rel).replace(/\/$/, '');

function membersOf(wsDir) {
  const { members, exclude } = cargo(wsDir);
  const excluded = exclude.map(e => join(wsDir, e));
  const matchers = members.map(m => picomatch(join(wsDir, m)));
  return CRATES.filter(
    c => c !== wsDir && matchers.some(m => m(c)) && !excluded.some(e => under(c, e)),
  );
}

/** The workspace root that governs a crate's build (itself when it is one), or null. */
function workspaceRootOf(crateDir) {
  const info = cargo(crateDir);
  if (info.isWorkspace) return crateDir;
  if (info.explicitWorkspace) return join(crateDir, info.explicitWorkspace);
  for (let d = parentOf(crateDir); ; d = parentOf(d)) {
    if (CRATE_SET.has(d) && cargo(d).isWorkspace) {
      const excluded = cargo(d).exclude.some(e => under(crateDir, join(d, e)));
      return !excluded && membersOf(d).includes(crateDir) ? d : null;
    }
    if (d === '') return null;
  }
}

// ── Closure ─────────────────────────────────────────────────────────

/**
 * The path-dependency closure of a set of root crates: crate dirs, workspace-root
 * files (Cargo.toml / Cargo.lock) and gitlinks, plus `via` — for each entry, the
 * crate that brought it in (null for a root) so a failure can show the chain.
 */
function closureOf(roots) {
  const dirs = new Set();
  const files = new Set();
  const gitlinks = new Set();
  const via = new Map();
  const rootSet = new Set(roots);
  const note = (entry, from) => via.has(entry) || via.set(entry, from);

  const follow = (from, base, rel) => {
    const target = join(base, rel);
    if (target.startsWith('..')) return; // outside the repository
    const sub = submoduleOf(target);
    if (sub) {
      gitlinks.add(sub);
      note(sub, from);
      return;
    }
    if (CRATE_SET.has(target)) visit(target, from); // else a [lib]/[[bin]] path, not a crate
  };

  function visit(crateDir, from = null) {
    if (dirs.has(crateDir)) return;
    dirs.add(crateDir);
    note(crateDir, from);
    const info = cargo(crateDir);
    const isRoot = rootSet.has(crateDir);
    const ws = workspaceRootOf(crateDir);

    for (const p of info.hasPackage ? info.allPaths : info.patchPaths) follow(crateDir, crateDir, p);
    if (ws) {
      const wsInfo = cargo(ws);
      for (const name of info.inheritedDeps) {
        const p = wsInfo.workspaceDeps.get(name);
        if (p) follow(crateDir, ws, p);
      }
      if (ws !== crateDir) {
        const add = f => {
          files.add(f);
          note(f, crateDir);
        };
        if (isRoot || info.inherits) add(`${ws}/Cargo.toml`);
        if (isRoot && existsSync(abs(`${ws}/Cargo.lock`))) add(`${ws}/Cargo.lock`);
        if (isRoot) for (const p of wsInfo.patchPaths) follow(crateDir, ws, p);
      }
    }
    if (info.isWorkspace && (isRoot || !info.hasPackage)) {
      for (const member of membersOf(crateDir)) visit(member, crateDir);
    }
  }

  for (const r of roots) visit(r);
  return { dirs, files, gitlinks, via };
}

/** "a ← b ← root" — how a closure entry was reached. */
function chainOf(entry, via) {
  const chain = [];
  for (let at = via.get(entry); at; at = via.get(at)) chain.push(at);
  return chain.length ? ` (via ${chain.reverse().join(' → ')})` : '';
}

// ── Globs ───────────────────────────────────────────────────────────

const matcherCache = new Map();
function glob(g) {
  if (!matcherCache.has(g)) {
    const scan = picomatch.scan(g);
    matcherCache.set(g, {
      negated: scan.negated,
      isGlob: scan.isGlob,
      base: scan.isGlob ? scan.base : g,
      match: picomatch(g),
    });
  }
  return matcherCache.get(g);
}

/**
 * How a glob names a crate, or null when it does not:
 *   'specific' — it points at the crate: the crate dir or its src/ is the glob's
 *                static base, it is the crate's Cargo.toml, or it is a pattern over
 *                Rust sources / manifests that matches the crate's Cargo.toml;
 *   'sweep'    — a wider glob merely includes the crate's Cargo.toml (crates/**).
 * Repo-wide patterns (empty static base) and submodule paths name nothing. A
 * sweep never names a crate governed by a workspace outside the sweep (such a
 * crate is built by that workspace), and a literal workspace-manifest glob
 * (elohim/Cargo.toml) names no member of a virtual workspace.
 */
function nameKind(g, crate) {
  const { negated, isGlob, base, match } = glob(g);
  if (negated || base === '' || submoduleOf(base)) return null;
  if (base === crate || base === `${crate}/src`) return 'specific';
  if (!match(`${crate}/Cargo.toml`)) return null;
  if (!isGlob) return cargo(crate).hasPackage ? 'specific' : null;
  if (/\.rs\b|Cargo\.toml/.test(g.slice(base.length))) return 'specific';
  const ws = workspaceRootOf(crate);
  return !ws || under(ws, base) ? 'sweep' : null;
}

function namedCrates(globs) {
  const specific = new Set();
  const all = new Set();
  for (const g of globs) {
    for (const c of CRATES) {
      const kind = nameKind(g, c);
      if (kind) all.add(c);
      if (kind === 'specific') specific.add(c);
    }
  }
  return { specific, all };
}

function coversDir(dir, globs) {
  return globs.some(g => {
    const { negated, isGlob, base, match } = glob(g);
    if (negated) return false;
    if (base !== '' && under(base, dir)) return true; // the glob lies within (or is) the dir
    if (isGlob && under(dir, base)) return match(dir) || filesUnder(dir).some(f => match(f));
    return false;
  });
}

const coversPath = (path, globs) => globs.some(g => !glob(g).negated && glob(g).match(path));

// ── Units ───────────────────────────────────────────────────────────

/** Crates a cargo gate project builds: those at or under its dir, else its workspace package. */
function cargoGateRoots(project) {
  if (!project.run?.cargo) return [];
  const dir = project.dir;
  if (dir && dir !== '.') return submoduleOf(dir) ? [] : CRATES.filter(c => under(c, dir));
  const ws = project.run.cargo.workspace;
  if (ws && CRATE_SET.has(ws) && !submoduleOf(ws) && cargo(ws).hasPackage) return [ws];
  return [];
}

/**
 * The crates a unit builds, from the crates it names: a named crate that another
 * named crate already reaches through its closure is a watched dependency, not a
 * separate build — it stays in the closure but brings no workspace lockfile of
 * its own.
 */
function topRoots(named) {
  const list = [...new Set(named)];
  const reach = new Map(list.map(c => [c, closureOf([c]).dirs]));
  return list.filter(c => !list.some(o => o !== c && reach.get(o).has(c) && !reach.get(c).has(o)));
}

/** Specific names win; sweeps stand in only when nothing else says what the unit builds. */
const chooseRoots = (named, extra = []) =>
  topRoots([...extra, ...(named.specific.size || extra.length ? named.specific : named.all)]);

function buildUnits() {
  const units = [];
  for (const { path, content } of discoverManifests()) {
    const steps = content.steps || {};
    const projects = Object.entries(content.gate?.projects || {});
    const triggerSteps = p => p.steps || (p.inputs ? [] : Object.keys(steps));
    const cargoTriggered = new Set(
      projects.filter(([, p]) => p.run?.cargo).flatMap(([, p]) => triggerSteps(p)),
    );

    for (const [name, step] of Object.entries(steps)) {
      const sources = step.inputs?.sources || [];
      const named = namedCrates(sources);
      if (!cargoTriggered.has(name) && named.specific.size === 0) continue;
      const roots = chooseRoots(named);
      if (roots.length) units.push({ manifest: path, step: name, globs: sources, roots });
    }

    for (const [name, project] of projects) {
      const inline = project.inputs?.sources || [];
      const named = namedCrates(inline);
      if (!project.run?.cargo && named.specific.size === 0) continue;
      const roots = chooseRoots(named, cargoGateRoots(project));
      const globs = [...inline, ...triggerSteps(project).flatMap(s => steps[s]?.inputs?.sources || [])];
      if (roots.length) units.push({ manifest: path, step: `gate:${name}`, globs, roots });
    }
  }
  return units;
}

function missingFor(unit) {
  const { dirs, files, gitlinks, via } = closureOf(unit.roots);
  return [
    ...[...dirs].filter(d => !coversDir(d, unit.globs)).map(d => `${d}/${chainOf(d, via)}`),
    ...[...files].filter(f => !coversPath(f, unit.globs)).map(f => `${f}${chainOf(f, via)}`),
    ...[...gitlinks]
      .filter(s => !coversPath(s, unit.globs))
      .map(s => `${s} (gitlink)${chainOf(s, via)}`),
  ].sort();
}

const units = buildUnits();

test('the closure rail finds the Rust build units it guards', () => {
  const names = units.map(u => `${u.manifest}:${u.step}`);
  assert.ok(
    names.includes('elohim/holochain/build-manifest.json:cargo-build-storage'),
    `storage is no longer a discovered unit — discovery broke: ${names.join(', ')}`,
  );
  assert.ok(units.length >= 10, `only ${units.length} Rust build units discovered: ${names.join(', ')}`);
});

for (const unit of units) {
  test(`${unit.manifest} ${unit.step} watches its whole Cargo path-dependency closure`, () => {
    const missing = missingFor(unit);
    assert.deepEqual(
      missing,
      [],
      `${unit.manifest} step '${unit.step}' (roots: ${unit.roots.join(', ')}) — no watch glob matches ` +
        `these closure paths, so a change there ships a stale build:\n  ${missing.join('\n  ')}\n` +
        `Add globs for them to that step's inputs.sources.`,
    );
  });
}
