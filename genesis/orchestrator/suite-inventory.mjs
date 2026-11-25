// The orchestrator suite's named half, resolved by one function on both sides (E11): what its key
// hashes and what its view holds. gate-evidence.mjs keys on `suiteInventory(...).digest`, the package
// closure's digest and node's version; gate-runner.mjs hands those to suite-run.mjs, which resolves
// its own inventory with the same function, materialises the view from it, and writes one result
// for the attempt; gate-runner.mjs decides from that result (suiteDecision).
//
// An inventory is: the commit, every superproject entry the view holds (path, mode, type, object
// id), and every declared submodule pin with the entries taken from it. The view's packages are
// the closure of the list's `modules` in this checkout's pnpm install, hashed file by file.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync, copyFileSync, cpSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync,
  readlinkSync, realpathSync, renameSync, symlinkSync, writeFileSync,
} from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { isReadPattern, suiteReadSet } from './gate-input-audit.mjs';

export const SUITE_DIR = 'genesis/orchestrator';
export const SUITE_LIST = `${SUITE_DIR}/suite-reads.json`;
/** Where gate-runner.mjs names the fresh directory one attempt's builder writes its result to. */
export const SUITE_RESULT_ENV = 'GATE_SUITE_RESULT';
/** Where gate-runner.mjs hands the builder the key's expectation. */
export const SUITE_EXPECT_ENV = 'GATE_SUITE_EXPECT';
const EXPECT_FIELDS = ['commit', 'inventory', 'packages', 'node'];

/** `commit:inventory:packages:node`: no braces, which a Task's declared environment refuses. */
export function encodeExpectation(expected) {
  return EXPECT_FIELDS.map(field => {
    const value = String(expected?.[field] ?? '');
    if (!value || /[:{}\s]/.test(value)) throw new Error(`suite-run: expectation ${field} is not encodable: ${value}`);
    return value;
  }).join(':');
}

/** The expectation `encodeExpectation` wrote, or null for an empty value; a malformed one throws. */
export function decodeExpectation(text) {
  if (!text || !text.trim()) return null;
  const parts = text.trim().split(':');
  if (parts.length !== EXPECT_FIELDS.length || parts.some(part => !part)) throw new Error(`suite-run: ${SUITE_EXPECT_ENV} is malformed`);
  return Object.fromEntries(EXPECT_FIELDS.map((field, i) => [field, parts[i]]));
}
// Machine-written ledgers the gate itself appends to; gate-evidence.mjs GATE_EXHAUST.
const EXHAUST = /^\.claude\/data\/([^/]+\.jsonl|ci-cursor\.json|governance-cursor\.json|runtime-cursor\.json)$/;
const isHabitAtom = path => /(^|\/)\.epr-meta\/[^/]+\.habit\.md$/.test(path);
const sha256 = data => createHash('sha256').update(data).digest('hex');

/**
 * The environment of every git call that resolves or builds a view, on both launch paths: no
 * inherited GIT_* variable, no system or global configuration, a fixed locale and timezone, and
 * safe.directory from the command line so ownership does not depend on the caller's config.
 */
export function gitEnv({ execPath = process.execPath } = {}) {
  return {
    PATH: [...new Set([dirname(execPath), '/usr/local/bin', '/usr/bin', '/bin'])].join(':'),
    LANG: 'C.UTF-8',
    TZ: 'UTC',
    GIT_CONFIG_NOSYSTEM: '1',
    GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_CONFIG_COUNT: '1',
    GIT_CONFIG_KEY_0: 'safe.directory',
    GIT_CONFIG_VALUE_0: '*',
    GIT_TERMINAL_PROMPT: '0',
  };
}

export function git(args, { cwd, env = gitEnv(), input, encoding = 'utf8', spawn = spawnSync }) {
  const result = spawn('git', args, {
    cwd, env, input, maxBuffer: 256 * 1024 * 1024, ...(encoding === 'buffer' ? {} : { encoding }),
  });
  if (result.error || result.status !== 0) {
    const said = result.error?.message || String(result.stderr || '').trim().split('\n').pop() || `exit ${result.status}`;
    throw new Error(`suite-run: git ${args.slice(0, 2).join(' ')} in ${cwd} failed: ${said}`);
  }
  return result.stdout;
}

/**
 * Which repository paths the view holds: `has(path)` for a superproject path, `inside(path)`
 * for a path inside a declared submodule. Derived from the read set the key hashes.
 */
export function viewMatcher(text, listPath = SUITE_LIST) {
  const reads = suiteReadSet(text, listPath);
  if (!reads) throw new Error(`suite-run: ${listPath} is not a suite list`);
  const names = reads.paths.filter(isReadPattern).map(path => path.slice(3));
  const exact = reads.paths.filter(path => !isReadPattern(path));
  const dir = listPath.slice(0, listPath.lastIndexOf('/'));
  const base = path => path.slice(path.lastIndexOf('/') + 1);
  const has = path => {
    if (path === dir || path.startsWith(`${dir}/`)) return !isHabitAtom(path);
    return exact.some(read => path === read || path.startsWith(`${read}/`)) || names.includes(base(path));
  };
  const inside = path => names.includes(base(path));
  return { has, inside, reads: reads.paths };
}

/** `[{ mode, type, oid, path }]` for every entry of `commit` in the repository at `cwd`. */
function treeEntries(cwd, commit, env, spawn = spawnSync) {
  return git(['ls-tree', '-r', '-z', '--full-tree', commit], { cwd, env, spawn }).split('\0').filter(Boolean).map(entry => {
    const tab = entry.indexOf('\t');
    const [mode, type, oid] = entry.slice(0, tab).split(' ');
    return { mode, type, oid, path: entry.slice(tab + 1) };
  });
}

/**
 * The submodule paths `.gitmodules` declares `update = none`: a fresh clone and CI never hold their
 * files, so the view takes nothing from them (their pin is still keyed). From `commit`'s copy of
 * the file, or the working tree's when `file` is given. No file, or none readable, is the empty set.
 */
export function updateNoneSubmodules({ root, commit, file, env = gitEnv(), spawn = spawnSync }) {
  const source = file ? ['--file', file] : ['--blob', `${commit}:.gitmodules`];
  const r = spawn('git', ['config', '-z', ...source, '--get-regexp', '^submodule\\..*\\.(path|update)$'], { cwd: root, env, encoding: 'utf8' });
  if (r.status !== 0) return new Set();
  const path = new Map(), update = new Map();
  for (const record of String(r.stdout).split('\0').filter(Boolean)) {
    const nl = record.indexOf('\n');
    const key = record.slice(0, nl), value = record.slice(nl + 1);
    const match = /^submodule\.(.*)\.(path|update)$/.exec(key);
    if (match) (match[2] === 'path' ? path : update).set(match[1], value);
  }
  return new Set([...path].filter(([name]) => update.get(name)?.toLowerCase() === 'none').map(([, value]) => value));
}

/** A git dir that holds `oid`: the submodule's checkout, then this checkout's module store, then the common dir's. */
function moduleStore(root, path, oid, env, spawn = spawnSync) {
  const candidates = [];
  if (existsSync(join(root, path, '.git'))) candidates.push(join(root, path));
  candidates.push(git(['rev-parse', '--path-format=absolute', '--git-path', `modules/${path}`], { cwd: root, env, spawn }).trim());
  candidates.push(join(git(['rev-parse', '--path-format=absolute', '--git-common-dir'], { cwd: root, env, spawn }).trim(), 'modules', path));
  for (const dir of candidates) {
    if (!existsSync(dir)) continue;
    if (spawn('git', ['cat-file', '-e', `${oid}^{commit}`], { cwd: dir, env }).status === 0) return dir;
  }
  return null;
}

/**
 * The submodule pins the named half's view reads, and which of them no object store here holds.
 * Declared set: the gitlink paths that `suite-reads.json` names as exact reads, found in `commit`'s
 * tree. Lookup: `moduleStore`, the one the view builder uses. Nothing is fetched or written.
 * A submodule `.gitmodules` declares `update = none` is left out: the view takes nothing from it and
 * `git submodule update --init` skips it. A repository with no list at `commit` declares nothing.
 */
export function declaredPinStores({ root, commit = 'HEAD', env = gitEnv() }) {
  let reads;
  try { reads = suiteReadSet(blobAt(root, commit, SUITE_LIST, env), SUITE_LIST); } catch { return []; }
  const exact = (reads?.paths || []).filter(path => !isReadPattern(path) && path !== '.');
  if (!exact.length) return [];
  const none = updateNoneSubmodules({ root, commit, env });
  const listing = git(['ls-tree', '-z', commit, '--', ...exact], { cwd: root, env }).split('\0').filter(Boolean);
  return listing.map(entry => {
    const tab = entry.indexOf('\t');
    const [, type, oid] = entry.slice(0, tab).split(' ');
    return { type, oid, path: entry.slice(tab + 1) };
  }).filter(e => e.type === 'commit' && exact.includes(e.path) && !none.has(e.path))
    .map(({ path, oid }) => ({ path, oid, store: moduleStore(root, path, oid, env) }));
}

/** The text of `path` at `commit`. */
export function blobAt(root, commit, path, env = gitEnv()) {
  return git(['show', `${commit}:${path}`], { cwd: root, env });
}

/**
 * The inventory of the named half at `commit`, from the list text at that commit. `digest` covers
 * every entry's path, mode, type and object id and every pin with its entries; it leaves the commit
 * out, so a commit that changes nothing the view holds keeps it.
 */
export function suiteInventory({ root, commit, listText, env = gitEnv(), spawn = spawnSync }) {
  const matcher = viewMatcher(listText);
  const all = treeEntries(root, commit, env, spawn).filter(e => matcher.has(e.path));
  const entries = all.filter(e => e.type === 'blob');
  const none = updateNoneSubmodules({ root, commit, env, spawn });
  const pins = all.filter(e => e.type === 'commit').map(link => {
    // update = none: the pin is keyed, nothing is taken from the submodule, here or on any device.
    if (none.has(link.path)) return { path: link.path, oid: link.oid, store: null, entries: [] };
    const store = moduleStore(root, link.path, link.oid, env, spawn);
    if (!store) {
      throw new Error(`suite-run: submodule ${link.path} at ${link.oid} is in no object store here; run: git submodule update --init ${link.path}`);
    }
    const inner = treeEntries(store, link.oid, env, spawn).filter(e => e.type === 'blob' && matcher.inside(e.path));
    return { path: link.path, oid: link.oid, store, entries: inner };
  });
  const row = e => [e.path, e.mode, e.type, e.oid];
  const digest = sha256(JSON.stringify({
    v: 1, entries: entries.map(row), pins: pins.map(p => [p.path, p.oid, p.entries.map(row)]),
  }));
  return { commit, matcher, entries, pins, digest };
}

/**
 * Writes blob entries under `dest` from the object store at `cwd`. Every `cat-file --batch`
 * response is checked: the object id, the type, the size and the newline after the content; a
 * `missing` object is an error naming it, never an empty file.
 */
export function writeBlobs(cwd, entries, dest, env = gitEnv()) {
  if (!entries.length) return 0;
  const out = git(['cat-file', '--batch'], { cwd, env, input: `${entries.map(e => e.oid).join('\n')}\n`, encoding: 'buffer' });
  let at = 0;
  for (const entry of entries) {
    const nl = out.indexOf(10, at);
    if (nl < 0) throw new Error(`suite-run: cat-file ended before ${entry.oid} (${entry.path})`);
    const header = out.subarray(at, nl).toString('utf8');
    if (header === `${entry.oid} missing`) throw new Error(`suite-run: object ${entry.oid} (${entry.path}) is missing from ${cwd}`);
    const [oid, type, size] = header.split(' ');
    const length = Number(size);
    if (oid !== entry.oid || type !== 'blob' || !/^\d+$/.test(size || '')) {
      throw new Error(`suite-run: cat-file answered "${header}" for ${entry.oid} (${entry.path})`);
    }
    const end = nl + 1 + length;
    if (end >= out.length || out[end] !== 10) throw new Error(`suite-run: cat-file framing broken after ${entry.oid} (${entry.path})`);
    const bytes = out.subarray(nl + 1, end);
    at = end + 1;
    const file = join(dest, entry.path);
    mkdirSync(dirname(file), { recursive: true });
    if (entry.mode === '120000') symlinkSync(bytes.toString('utf8'), file);
    else writeFileSync(file, bytes, { mode: entry.mode === '100755' ? 0o755 : 0o644 });
  }
  if (at !== out.length) throw new Error(`suite-run: cat-file answered more than was asked in ${cwd}`);
  return entries.length;
}

/** Materialises an inventory at `dest`; returns the number of files written. */
export function materialise(inventory, dest, { root, env = gitEnv() }) {
  let files = writeBlobs(root, inventory.entries, dest, env);
  for (const pin of inventory.pins) {
    mkdirSync(join(dest, pin.path), { recursive: true });
    if (!pin.store) continue;
    files += writeBlobs(pin.store, pin.entries, join(dest, pin.path), env);
  }
  return files;
}

/**
 * The paths the view holds that differ from HEAD: modified, staged, untracked, renamed (either
 * side) or a submodule that moved or holds changes. The gate's recording decision and the view's
 * builder both ask this, with these flags.
 */
export function dirtyInView(matcher, { cwd, env = gitEnv() }) {
  const out = git(['status', '--porcelain=v1', '-z', '--untracked-files=all', '--ignore-submodules=none'], { cwd, env });
  const paths = [];
  const rows = out.split('\0');
  for (let i = 0; i < rows.length; i++) {
    const row = rows[i];
    if (!row) continue;
    const both = row[0] === 'R' || row[0] === 'C' ? [row.slice(3), rows[++i]] : [row.slice(3)];
    for (const p of both) if (p && !EXHAUST.test(p) && matcher.has(p)) paths.push(p);
  }
  return paths;
}

function isLink(path) {
  try { return lstatSync(path).isSymbolicLink(); } catch { return false; }
}

function copyWorkingFile(from, to) {
  mkdirSync(dirname(to), { recursive: true });
  const stat = lstatSync(from);
  if (stat.isSymbolicLink()) symlinkSync(readlinkSync(from), to);
  else if (stat.isFile()) { copyFileSync(from, to); chmodSync(to, stat.mode & 0o777); }
}

/** Copies the working tree's bytes of every path the view holds (a dirty run, recorded nowhere). */
export function copyWorkingTree(matcher, dest, { root, env = gitEnv() }) {
  let files = 0;
  const none = updateNoneSubmodules({ root, file: join(root, '.gitmodules'), env });
  const rows = git(['ls-files', '-z', '--cached', '--others', '--exclude-standard', '--stage'], { cwd: root, env }).split('\0').filter(Boolean);
  for (const row of rows) {
    // --stage prints `<mode> <oid> <stage>\t<path>` for tracked files and a bare path for untracked ones.
    const tab = row.indexOf('\t');
    const path = tab < 0 ? row : row.slice(tab + 1);
    const [mode, oid] = tab < 0 ? ['', null] : row.slice(0, tab).split(' ');
    if (!matcher.has(path) || EXHAUST.test(path)) continue;
    if (mode === '160000') {
      const module = join(root, path);
      mkdirSync(join(dest, path), { recursive: true });
      if (none.has(path)) continue;
      if (existsSync(join(module, '.git'))) {
        for (const inner of git(['ls-files', '-z', '--cached', '--others', '--exclude-standard'], { cwd: module, env }).split('\0').filter(Boolean)) {
          if (!matcher.inside(inner) || !existsSync(join(module, inner))) continue;
          copyWorkingFile(join(module, inner), join(dest, path, inner));
          files++;
        }
      } else {
        const store = moduleStore(root, path, oid, env);
        if (!store) throw new Error(`suite-run: submodule ${path} at ${oid} is in no object store here; run: git submodule update --init ${path}`);
        files += writeBlobs(store, treeEntries(store, oid, env).filter(e => e.type === 'blob' && matcher.inside(e.path)), join(dest, path), env);
      }
      continue;
    }
    if (!existsSync(join(root, path)) && !isLink(join(root, path))) continue; // deleted in the working tree
    copyWorkingFile(join(root, path), join(dest, path));
    files++;
  }
  return files;
}

function listDeps(modules) {
  const out = [];
  for (const name of readdirSync(modules)) {
    if (name.startsWith('.')) continue;
    if (name.startsWith('@')) for (const sub of readdirSync(join(modules, name))) out.push(`${name}/${sub}`);
    else out.push(name);
  }
  return out;
}

/**
 * The package closure of `modules` (a map of package directory, '.' for the root, to the names its
 * node_modules holds) in this checkout's pnpm install: `packages`, the store directories copied
 * whole, and `links`, every link to recreate with its text. Paths are relative to `root`.
 */
export function packageClosure(root, modules = {}) {
  const store = join(root, 'node_modules', '.pnpm');
  const packages = new Set();
  const links = new Map();
  const queue = [];
  const enqueue = real => { const rel = relative(root, real); if (!packages.has(rel)) { packages.add(rel); queue.push(real); } };
  for (const [dir, names] of Object.entries(modules)) {
    for (const name of names) {
      let real;
      try { real = realpathSync(join(root, dir, 'node_modules', name)); }
      catch { throw new Error(`suite-run: module ${name} is not installed in ${dir}; run: just dev prep`); }
      if (real.startsWith(`${store}${sep}`)) enqueue(real);
      else if (!real.startsWith(`${root}${sep}`)) throw new Error(`suite-run: module ${name} resolves outside this checkout (${real})`);
      const at = join(dir, 'node_modules', name);
      links.set(at, relative(dirname(join(root, at)), real));
    }
  }
  while (queue.length) {
    const real = queue.shift();
    // .pnpm/<entry>/node_modules holds the package and a link to each of its dependencies.
    const entryModules = real.slice(0, real.lastIndexOf(`${sep}node_modules${sep}`) + `${sep}node_modules`.length);
    for (const dep of listDeps(entryModules)) {
      const from = join(entryModules, dep);
      if (!isLink(from)) continue;
      links.set(relative(root, from), readlinkSync(from));
      const target = realpathSync(from);
      if (target.startsWith(`${store}${sep}`)) enqueue(target);
    }
  }
  return { packages: [...packages].sort(), links: [...links.entries()].sort((a, b) => a[0].localeCompare(b[0])) };
}

/** Copies a package closure from `root` to `dest`. */
export function copyClosure(closure, dest, { root }) {
  for (const pkg of closure.packages) cpSync(join(root, pkg), join(dest, pkg), { recursive: true, verbatimSymlinks: true });
  for (const [at, text] of closure.links) {
    const to = join(dest, at);
    if (existsSync(to) || isLink(to)) continue;
    mkdirSync(dirname(to), { recursive: true });
    symlinkSync(text, to);
  }
}

function listing(dir, prefix = '') {
  const rows = [];
  for (const entry of readdirSync(join(dir, prefix), { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    const path = join(dir, rel);
    if (entry.isSymbolicLink()) rows.push([rel, 'link', readlinkSync(path)]);
    else if (entry.isDirectory()) rows.push(...listing(dir, rel));
    else if (entry.isFile()) rows.push([rel, (lstatSync(path).mode & 0o111) ? 'exec' : 'file', sha256(readFileSync(path))]);
  }
  return rows;
}

/** One digest over every file, mode and link of a package closure as it lies under `base`. */
export function closureDigest(closure, base) {
  return sha256(JSON.stringify({
    v: 1, links: closure.links, packages: closure.packages.map(pkg => [pkg, listing(join(base, pkg))]),
  }));
}

/**
 * Refuses a closure package whose name and version the lock does not record, or whose store
 * directory names another version: an install that drifted from pnpm-lock.yaml.
 */
export function checkLockVersions(closure, lockText, { root }) {
  for (const pkg of closure.packages) {
    const { name, version } = JSON.parse(readFileSync(join(root, pkg, 'package.json'), 'utf8'));
    const key = `${name}@${version}`;
    const entry = pkg.split(sep).find((part, i, parts) => parts[i - 1] === '.pnpm') || '';
    const recorded = lockText.includes(`\n  ${key}:`) || lockText.includes(`\n  '${key}':`);
    if (!recorded || !entry.startsWith(`${name.replace('/', '+')}@${version}`)) {
      throw new Error(`suite-run: installed ${key} (${pkg}) is not what pnpm-lock.yaml records; run: pnpm install --frozen-lockfile (just dev prep)`);
    }
  }
}

/** Every symlink under `root` that resolves outside it, as `link -> target`. */
export function linksOutside(root) {
  const top = resolve(root);
  const inside = path => path === top || path.startsWith(`${top}${sep}`);
  const out = [];
  const walk = dir => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isSymbolicLink()) {
        const lexical = resolve(dirname(path), readlinkSync(path));
        let real = lexical;
        try { real = realpathSync(path); } catch { /* dangling: judged by its text */ }
        if (!inside(lexical) || !inside(real)) out.push(`${relative(top, path)} -> ${readlinkSync(path)}`);
      } else if (entry.isDirectory()) walk(path);
    }
  };
  walk(top);
  return out;
}

/** Writes one attempt's result into `dir` as its last act: a temporary name, then a rename. */
export function writeResult(dir, result) {
  const tmp = join(dir, `.result-${process.pid}.tmp`);
  writeFileSync(tmp, `${JSON.stringify(result)}\n`);
  renameSync(tmp, join(dir, 'result.json'));
}

/** The result one attempt wrote, or null when there is none or it does not parse. */
export function readResult(dir) {
  try { return JSON.parse(readFileSync(join(dir, 'result.json'), 'utf8')); } catch { return null; }
}

/**
 * The parent's one decision for a run of the named half, from the result it wrote, never from the
 * exit status alone. `{ status, record, why }`:
 *   - no result, an unparsable one, or a verdict that disagrees with the process status (a signal
 *     included): red, nothing recorded, "the named half did not report";
 *   - `pass` on the key's bytes (full half, digests equal, HEAD and the view's paths unchanged
 *     since the start): 0, a green recorded; `pass` otherwise: 0, nothing recorded, why;
 *   - `fail` on the key's bytes: red, a red recorded (it vetoes an older green for the key);
 *     `fail` otherwise: red, nothing recorded.
 */
export function suiteDecision({ exit, signal = null, result, expected, gateSha, headAfter, dirtyAfter }) {
  const ran = exit === 0 && !signal ? 'pass' : 'fail';
  const red = typeof exit === 'number' && exit !== 0 ? exit : 1;
  if (!result || !['pass', 'fail'].includes(result.verdict) || result.verdict !== ran) {
    const said = !result ? 'no result' : `result ${result.verdict ?? 'without a verdict'}, process ${signal || `exit ${exit}`}`;
    return { status: red, record: false, why: `the named half did not report (${said})` };
  }
  const why = (() => {
    if (!expected) return 'no expectation was computed for this run';
    if (result.bytes !== 'key') return `it tested other bytes (${(result.why || []).join('; ') || 'not the key\'s'})`;
    if (result.full !== true) return 'it was not the full named half';
    for (const field of ['commit', 'inventory', 'packages', 'node']) {
      if (result[field] !== expected[field]) return `its ${field} ${String(result[field]).slice(0, 12)} is not the key's ${String(expected[field]).slice(0, 12)}`;
    }
    if (headAfter !== gateSha) return `HEAD moved to ${String(headAfter).slice(0, 9)} during the run (gated ${String(gateSha).slice(0, 9)})`;
    if (dirtyAfter === null) return 'the view\'s paths could not be read after the run';
    if (dirtyAfter.length) return `declared paths changed during the run (${dirtyAfter.slice(0, 3).join(', ')})`;
    return null;
  })();
  if (ran === 'pass') return why ? { status: 0, record: false, why: `a pass, recorded nowhere: ${why}` } : { status: 0, record: true };
  return why ? { status: red, record: false, why: `a red, recorded nowhere: ${why}` } : { status: red, record: true };
}

/** The view's paths that now differ from HEAD, from the list at `commit`; null when unreadable. */
export function dirtyAfterRun(root, commit, env = gitEnv()) {
  try { return dirtyInView(viewMatcher(blobAt(root, commit, SUITE_LIST, env)), { cwd: root, env }); } catch { return null; }
}
