#!/usr/bin/env node
// Cross-workspace import ratchet — the rail that keeps a BUNDLE seam from
// being mistaken for a DOMAIN seam.
//
// Every directory in app/ that ships to users is an EPR app: an independently
// built, content-addressed Angular bundle (see app/CLAUDE.md). That boundary
// is a DEPLOYMENT fact. It is NOT, on its own, a module boundary — because each
// app's tsconfig `paths` maps sibling `@app/<pillar>/*` aliases straight at the
// other workspace's `src/app/*`. Deep paths, both directions, no public API.
// Two workspaces, one TypeScript program.
//
// So a workspace extraction reads as "this domain is now self-contained" while
// nothing anywhere checks that it is. That is exactly how app/lamad/ ended up
// mutually entangled with app/elohim-app/ — including the cross-pillar content
// substrate (`models/content-node.model`) living inside the learning bundle and
// being imported by the core `elohim` pillar, i.e. the dependency arrow running
// core → pillar. Nobody added that on purpose in one move; it accreted one
// import at a time, each locally reasonable, none of them observable.
//
// This rail makes each cross-workspace edge an explicit, counted, checked-in
// fact. It is a RATCHET, not a ban: today's edges are recorded in the baseline
// as declared debt, and the rail fails on
//
//   * a NEW cross-workspace module specifier (an edge nobody declared), or
//   * MORE references to an existing one (the entanglement deepening).
//
// Edges that SHRINK are reported and ask you to re-baseline. The number only
// goes down. When a direction reaches zero the cycle is gone and the seam is
// real; delete its baseline entry and it can never come back silently.
//
// The rail deliberately does not judge which placement is correct — that is an
// architect call (see genesis/data/timeline/backlog/arch-frontend-bundle-seams-backlog.md
// row 1). It only guarantees the next such drift is a decision somebody made
// rather than one that happened.
//
// usage: lint-workspace-imports.mjs <appDir>   (the directory holding tsconfig.json)
//        lint-workspace-imports.mjs <appDir> --write-baseline

import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { join, resolve, relative, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const BASELINE_PATH = join(SCRIPT_DIR, 'workspace-import-baseline.json');

const appDir = process.argv[2];
const writeBaseline = process.argv.includes('--write-baseline');
if (!appDir) {
  console.error('usage: lint-workspace-imports.mjs <appDir> [--write-baseline]');
  process.exit(2);
}
const appRoot = resolve(appDir);
const appName = basename(appRoot);

const tsconfigPath = join(appRoot, 'tsconfig.json');
if (!existsSync(tsconfigPath)) {
  console.error(`workspace-imports: no tsconfig.json at ${tsconfigPath}`);
  process.exit(2);
}

// tsconfig.json is JSONC — both app tsconfigs carry explanatory comments.
const stripJsonc = (src) => {
  let out = '';
  let inStr = false;
  let strQuote = '';
  let inLine = false;
  let inBlock = false;
  for (let i = 0; i < src.length; i++) {
    const c = src[i];
    const n = src[i + 1];
    if (inLine) {
      if (c === '\n') {
        inLine = false;
        out += c;
      }
      continue;
    }
    if (inBlock) {
      if (c === '*' && n === '/') {
        inBlock = false;
        i++;
      }
      continue;
    }
    if (inStr) {
      out += c;
      if (c === '\\') {
        out += src[++i] ?? '';
        continue;
      }
      if (c === strQuote) inStr = false;
      continue;
    }
    if (c === '"' || c === "'") {
      inStr = true;
      strQuote = c;
      out += c;
      continue;
    }
    if (c === '/' && n === '/') {
      inLine = true;
      i++;
      continue;
    }
    if (c === '/' && n === '*') {
      inBlock = true;
      i++;
      continue;
    }
    out += c;
  }
  return out;
};

const tsconfig = JSON.parse(stripJsonc(readFileSync(tsconfigPath, 'utf8')));
const paths = tsconfig?.compilerOptions?.paths ?? {};

// An alias is CROSS-WORKSPACE when its target resolves outside this app root.
// `@elohim/*` library aliases and `@holochain/*` / node_modules targets are NOT
// workspace peers — this rail guards app↔app edges only, the ones that look
// like a domain boundary and are not enforced as one.
const crossAliases = [];
for (const [alias, targets] of Object.entries(paths)) {
  if (!alias.startsWith('@app/')) continue;
  const target = Array.isArray(targets) ? targets[0] : targets;
  if (typeof target !== 'string') continue;
  const abs = resolve(appRoot, target);
  if (abs.startsWith(appRoot + '/')) continue; // own tree — fine
  if (abs.includes('/node_modules/')) continue;
  const peer = relative(resolve(appRoot, '..'), abs).split('/')[0];
  crossAliases.push({ alias, prefix: alias.replace(/\*$/, ''), peer });
}

if (crossAliases.length === 0) {
  console.log(`lint-workspace-imports: ${appName} declares no cross-workspace @app aliases — clean`);
  process.exit(0);
}

// --- scan -------------------------------------------------------------------

const walk = (dir, acc = []) => {
  for (const entry of readdirSync(dir)) {
    if (entry === 'node_modules' || entry === 'dist' || entry.startsWith('.')) continue;
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, acc);
    else if (/\.ts$/.test(entry)) acc.push(full);
  }
  return acc;
};

const srcDir = join(appRoot, 'src');
if (!existsSync(srcDir)) {
  console.error(`workspace-imports: no src/ under ${appRoot}`);
  process.exit(2);
}

// `from '<spec>'`, `import '<spec>'`, `import('<spec>')`, `require('<spec>')`
const SPEC_RE = /(?:\bfrom\s*|\bimport\s*\(?\s*|\brequire\s*\(\s*)['"]([^'"]+)['"]/g;

/** specifier -> { count, files:Set } */
const edges = new Map();
for (const file of walk(srcDir)) {
  const src = readFileSync(file, 'utf8');
  for (const m of src.matchAll(SPEC_RE)) {
    const spec = m[1];
    const hit = crossAliases.find((a) => spec === a.prefix.replace(/\/$/, '') || spec.startsWith(a.prefix));
    if (!hit) continue;
    const e = edges.get(spec) ?? { count: 0, files: new Set(), peer: hit.peer };
    e.count += 1;
    e.files.add(relative(appRoot, file));
    edges.set(spec, e);
  }
}

const observed = Object.fromEntries([...edges.entries()].sort().map(([k, v]) => [k, v.count]));
const observedTotal = Object.values(observed).reduce((a, b) => a + b, 0);

// --- baseline ---------------------------------------------------------------

const baseline = existsSync(BASELINE_PATH) ? JSON.parse(readFileSync(BASELINE_PATH, 'utf8')) : { apps: {} };

if (writeBaseline) {
  baseline.apps ??= {};
  baseline.apps[appName] = {
    note: baseline.apps[appName]?.note ?? 'declared cross-workspace debt — ratchet down only',
    edges: observed,
  };
  writeFileSync(BASELINE_PATH, JSON.stringify(baseline, null, 2) + '\n');
  console.log(
    `lint-workspace-imports: baseline written for ${appName} — ` +
      `${Object.keys(observed).length} specifier(s), ${observedTotal} reference(s)`,
  );
  process.exit(0);
}

const declared = baseline.apps?.[appName]?.edges;
if (!declared) {
  console.error(
    `workspace-imports: ${appName} has cross-workspace @app aliases but no baseline entry.\n` +
      `  Seed it deliberately:  node app/scripts/lint-workspace-imports.mjs ${appDir} --write-baseline\n` +
      `  A missing baseline is not "clean" — it is an unmeasured seam.`,
  );
  process.exit(1);
}

let failures = 0;
const fail = (msg) => {
  console.error(`workspace-imports: ${msg}`);
  failures += 1;
};

for (const [spec, count] of Object.entries(observed)) {
  const allowed = declared[spec];
  if (allowed === undefined) {
    const where = [...edges.get(spec).files].slice(0, 4);
    fail(
      `NEW cross-workspace edge  ${appName} → ${spec}  (${count} ref(s))\n` +
        where.map((f) => `    ${f}`).join('\n') +
        (edges.get(spec).files.size > 4 ? `\n    … +${edges.get(spec).files.size - 4} more` : '') +
        `\n    This crosses an EPR-app bundle boundary into another workspace's private source tree.\n` +
        `    Decide the placement first (does this type belong to that bundle's domain, or in a\n` +
        `    shared home both consume?). If the edge is genuinely intended, re-baseline with\n` +
        `    --write-baseline and say why in the commit.`,
    );
  } else if (count > allowed) {
    const where = [...edges.get(spec).files].slice(0, 4);
    fail(
      `edge DEEPENED  ${appName} → ${spec}  (${allowed} → ${count} ref(s))\n` +
        where.map((f) => `    ${f}`).join('\n') +
        `\n    The baseline is a ratchet: declared cross-workspace debt may shrink, never grow.`,
    );
  }
}

const shrunk = [];
for (const [spec, allowed] of Object.entries(declared)) {
  const count = observed[spec] ?? 0;
  if (count < allowed) shrunk.push(`${spec}  ${allowed} → ${count}${count === 0 ? '  (GONE)' : ''}`);
}

if (failures > 0) {
  const total = Object.values(declared).reduce((a, b) => a + b, 0);
  console.error(
    `\n${failures} cross-workspace import failure(s) in ${appName}.\n` +
      `Declared debt: ${Object.keys(declared).length} specifier(s) / ${total} reference(s).\n` +
      `Context: genesis/data/timeline/backlog/arch-frontend-bundle-seams-backlog.md (row 1)`,
  );
  process.exit(1);
}

if (shrunk.length > 0) {
  console.log(`lint-workspace-imports: ${appName} clean — ${shrunk.length} edge(s) shrank, please re-baseline:`);
  for (const s of shrunk) console.log(`  ${s}`);
  console.log(`  node app/scripts/lint-workspace-imports.mjs ${appDir} --write-baseline`);
} else {
  console.log(
    `lint-workspace-imports: ${appName} clean — ` +
      `${Object.keys(observed).length} declared cross-workspace specifier(s), ${observedTotal} reference(s)`,
  );
}
