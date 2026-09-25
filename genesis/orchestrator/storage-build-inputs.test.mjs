/**
 * Storage build inputs — one dependency surface, three lists kept honest
 *
 * The storage binary is compiled in Docker from a build context that includes
 * its sibling path-dep crates. Three lists describe that one surface:
 *
 *   1. the `path = "…"` closure of elohim/elohim-storage/Cargo.toml (the truth)
 *   2. the Dockerfile's COPY sources (what reaches the compiler)
 *   3. the edge manifest's `cargo-build-storage` source globs (what re-triggers
 *      the build)
 *
 * A crate in (1) missing from (2) fails the image build. A path in (2) missing
 * from (3) is worse: a change to that crate alone matches nothing, the
 * orchestrator does not rebuild edge, and the fleet runs a STALE storage binary
 * with no red anywhere (backlog edge-buildmanifest-sibling-crate-source-globs).
 * Every crate carved out of elohim-storage widens that hole unless this holds.
 *
 * Run:
 *   node --test genesis/orchestrator/storage-build-inputs.test.mjs
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, normalize, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const STORAGE = 'elohim/elohim-storage';

/** Repo-relative dirs of every `path = "…"` dependency, transitively. */
function pathDepClosure(crateDir, seen = new Set()) {
  const manifest = join(root, crateDir, 'Cargo.toml');
  if (!existsSync(manifest)) return seen;
  const text = readFileSync(manifest, 'utf8');
  for (const m of text.matchAll(/\bpath\s*=\s*"([^"]+)"/g)) {
    const dep = relative(root, resolve(root, crateDir, m[1]));
    if (dep.startsWith('..') || seen.has(dep) || dep === STORAGE) continue;
    if (!existsSync(join(root, dep, 'Cargo.toml'))) continue; // a [[bin]]/[lib] path, not a crate
    seen.add(dep);
    pathDepClosure(dep, seen);
  }
  return seen;
}

/** Build-context sources the storage Dockerfile copies (stage copies excluded). */
function dockerCopySources() {
  const text = readFileSync(join(root, STORAGE, 'Dockerfile'), 'utf8');
  const sources = new Set();
  for (const line of text.split('\n')) {
    const m = line.match(/^COPY\s+(?!--from)(.+)$/);
    if (!m) continue;
    const parts = m[1].trim().split(/\s+/).filter(p => !p.startsWith('--'));
    parts.slice(0, -1).forEach(p => sources.add(normalize(p).replace(/\/$/, '')));
  }
  return sources;
}

function manifestSources() {
  const manifest = JSON.parse(readFileSync(join(root, 'elohim/holochain/build-manifest.json'), 'utf8'));
  return manifest.steps['cargo-build-storage'].inputs.sources;
}

/** True when `path` (a file or dir) falls under a `dir/**` glob or equals a literal source. */
function covered(path, globs) {
  return globs.some(g => {
    if (g.endsWith('/**')) {
      const base = g.slice(0, -3);
      return path === base || path.startsWith(`${base}/`) || base.startsWith(`${path}/`);
    }
    return g === path || g.startsWith(`${path}/`);
  });
}

test('every path-dep crate in the storage closure reaches the Docker build context', () => {
  const copies = [...dockerCopySources()];
  const missing = [...pathDepClosure(STORAGE)].filter(
    dep => !copies.some(c => dep === c || dep.startsWith(`${c}/`)),
  );
  assert.deepEqual(missing, [], `path-dep crates the storage Dockerfile never COPYs: ${missing.join(', ')}`);
});

test('every source the storage Dockerfile copies re-triggers the edge build', () => {
  const globs = manifestSources();
  const unwatched = [...dockerCopySources()].filter(src => !covered(src, globs));
  assert.deepEqual(
    unwatched,
    [],
    `copied into the storage image but absent from cargo-build-storage sources — a change there ships a stale binary: ${unwatched.join(', ')}`,
  );
});

// The Dockerfile builds storage from /app with every sibling crate COPY'd flat beside it, so each
// of storage's own relative path deps is re-rooted by a `sed 's|path = "<rel>"|…|'` line. A COPY
// without its rewrite still fails the image: cargo looks for /<crate>/Cargo.toml (edge #1484,
// elohim-epr-index at ../epr-index — COPY'd, never re-rooted, invisible to the native gates).
test("every direct path dep of storage is re-rooted by the Dockerfile's sed rewrites", () => {
  const cargo = readFileSync(join(root, STORAGE, 'Cargo.toml'), 'utf8');
  const docker = readFileSync(join(root, STORAGE, 'Dockerfile'), 'utf8');
  const copyDests = [...docker.matchAll(/^COPY\s+(?!--from)\S+\s+(\/\S+?)\/?$/gm)].map(m => m[1]);
  const unrooted = [...cargo.matchAll(/\bpath\s*=\s*"(\.\.\/[^"]+)"/g)]
    .map(m => m[1])
    .filter(rel => existsSync(join(root, STORAGE, rel, 'Cargo.toml')))
    .filter(rel => !docker.includes(`s|path = "${rel}"|`))
    // …or COPY'd to the absolute place the unrewritten path resolves to from /app (/sdk, /vendor).
    .filter(rel => {
      const abs = resolve('/app', rel);
      return !copyDests.some(d => abs === d || abs.startsWith(`${d}/`));
    });
  assert.deepEqual(
    unrooted,
    [],
    `storage path deps with no Dockerfile path rewrite (cargo resolves them outside /app): ${unrooted.join(', ')}`,
  );
});
