#!/usr/bin/env node
// The orchestrator suite's read fence (E11). The orchestrator gate keys on the reads its suite
// declares in suite-reads.json, not on the whole commit tree; this fence is what makes that list
// a claim the suite enforces rather than a promise. Loaded with NODE_OPTIONS=--import=<this file>
// for every test file the list names (suite-run.mjs sets it), it watches what each test process,
// and every node process the test starts, reads in the repository, and appends a JSON line per
// violation to GATE_READ_FENCE_LOG. It never changes a test's outcome or a child's exit status:
// `node read-fence.mjs check <log>` afterwards fails the recipe naming each file and path.
//
// A named test file may read, without declaring it:
//   - anything under genesis/orchestrator (the gate's project tree keys it),
//   - its own file, and probe (exists/stat/realpath) a directory that is an ancestor of a
//     declared read or of genesis/orchestrator.
// Everything else it reads (content, probe or module load) must be covered by a declared read:
// an exact path, a directory prefix, or a `**/<name>` basename walk (gate-input-audit.mjs
// readCovers). A directory listing is allowed under a declared prefix, or anywhere when the file
// declares a `**/<name>` walk. RESIDUAL, named: such a walk could still depend on other names in
// the listings it reads; a test that counts entries in a listing is not caught.
//
// Child processes: a node child must carry this fence (NODE_OPTIONS naming it) or it is a
// violation. Any other child is unseen by the fence, so it must be bound to a fixture: its working
// directory outside the repository, and no argument, and no environment value the test added,
// naming a path in the repository. Otherwise it is a violation, and the file belongs on the tree
// side of the list (`tree: <reason>`), where orchestrator-tree keys it on the whole commit tree.
import fs from 'node:fs';
import childProcess from 'node:child_process';
import module from 'node:module';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { homedir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { isReadPattern, readCovers } from './gate-input-audit.mjs';

export const PROJECT_DIR = 'genesis/orchestrator';
const SELF = fileURLToPath(import.meta.url);
const CONTENT = new Set(['readFileSync', 'readFile', 'promises.readFile', 'openSync', 'open', 'promises.open',
  'createReadStream', 'copyFileSync', 'copyFile', 'promises.copyFile', 'cpSync', 'cp', 'promises.cp', 'module']);
const LISTING = new Set(['readdirSync', 'readdir', 'promises.readdir', 'opendirSync', 'opendir', 'promises.opendir',
  'globSync', 'glob', 'promises.glob', 'watch']);
// Inherited environment that names the checkout without being a read: where the shell or pnpm was.
const AMBIENT_ENV = new Set(['PWD', 'OLDPWD', 'INIT_CWD']);
const GIT_ENV = new Set(['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_COMMON_DIR', 'GIT_OBJECT_DIRECTORY',
  'GIT_ALTERNATE_OBJECT_DIRECTORIES']);

const inside = (rel, dir) => rel === dir || rel.startsWith(`${dir}/`);
const ancestorOf = (dir, rel) => dir === '.' || rel === dir || rel.startsWith(`${dir}/`);

/** The suite list's entry for one test file, by its path relative to the repository root. */
export function entryFor(list, rel) {
  for (const group of list.groups) {
    for (const entry of group.files) {
      if (path.posix.normalize(path.posix.join(PROJECT_DIR, entry.file)) === rel) return entry;
    }
  }
  return null;
}

/**
 * Judge one read by a named test file. `read` is { op, path } with a repo-relative path; returns
 * null when it is allowed, else the reason.
 */
export function judgeRead(read, entry, testFile) {
  const rel = read.path;
  const reads = entry.reads || [];
  if (inside(rel, PROJECT_DIR) || rel === testFile) return null;
  if (reads.some(declared => readCovers(declared, rel))) return null;
  if (LISTING.has(read.op)) {
    if (reads.some(isReadPattern)) return null;
    return 'undeclared listing';
  }
  if (!CONTENT.has(read.op)) {
    // A probe of a directory on the way to something declared reveals only that it exists.
    const targets = [PROJECT_DIR, testFile, ...reads.filter(r => !isReadPattern(r))];
    if (targets.some(target => ancestorOf(rel, target))) return null;
    if (reads.some(isReadPattern)) {
      const name = rel.split('/').pop();
      if (reads.some(r => isReadPattern(r) && r.slice(3) === name)) return null;
    }
  }
  return 'undeclared read';
}

/** Does `value` name `root` or a path under it anywhere (`--git-dir=<root>`, `cd <root>/x; …`)? */
function underRoot(root, value) {
  if (typeof value !== 'string') return false;
  for (let at = value.indexOf(root); at !== -1; at = value.indexOf(root, at + 1)) {
    const next = value[at + root.length];
    if (next === undefined || next === '/' || !/[\w.-]/.test(next)) return true;
  }
  return false;
}

/**
 * Judge one child process. `spawn` is { cmd, args, cwd (absolute), env (the child's, or null when
 * inherited), shellText }. `baseEnv` is the environment the fence process itself started with.
 */
export function judgeSpawn(spawn, { root, gitDir = null, fence, baseEnv }) {
  const base = path.basename(String(spawn.cmd).trim().split(/\s+/)[0] || '');
  const env = spawn.env || baseEnv;
  if (base === 'node' || spawn.cmd === process.execPath) {
    // The fence installs only with its log and root: NODE_OPTIONS alone would load it inert.
    const fenced = String(env.NODE_OPTIONS || '').includes(fence)
      && env.GATE_READ_FENCE_LOG === baseEnv.GATE_READ_FENCE_LOG && env.GATE_READ_FENCE_ROOT === baseEnv.GATE_READ_FENCE_ROOT
      && env.GATE_READ_FENCE_FILE === baseEnv.GATE_READ_FENCE_FILE && env.GATE_READ_FENCE_GITDIR === baseEnv.GATE_READ_FENCE_GITDIR;
    return fenced ? null : 'node child without the read fence';
  }
  // A child carrying the preload library is seen: read-fence-preload.c logs what it reads and
  // judges, at exec, a program it cannot see (static, set-id, or a child that clears the fence).
  const so = baseEnv.GATE_READ_FENCE_SO;
  if (so && String(env.LD_PRELOAD || '').split(/[:\s]+/).includes(so)
    && env.GATE_READ_FENCE_SO === so && env.GATE_READ_FENCE_LOG === baseEnv.GATE_READ_FENCE_LOG
    && env.GATE_READ_FENCE_ROOT === baseEnv.GATE_READ_FENCE_ROOT && env.GATE_READ_FENCE_GITDIR === baseEnv.GATE_READ_FENCE_GITDIR
    && env.GATE_READ_FENCE_FILE === baseEnv.GATE_READ_FENCE_FILE) return null;
  const inRepo = value => underRoot(root, value) || (gitDir && underRoot(gitDir, value));
  const args = (spawn.args || []).map(String);
  // git -C <dir> works in <dir>, not in its process cwd; the argument check below still refuses a
  // -C, --git-dir or --work-tree that names this repository.
  const gitAt = base === 'git' ? args.find((arg, i) => args[i - 1] === '-C') : null;
  const cwd = path.resolve(gitAt ? path.resolve(spawn.cwd, gitAt) : spawn.cwd);
  if (inRepo(cwd)) return 'unfenced child in the repository';
  if ([String(spawn.cmd), ...args].some(inRepo)) return 'unfenced child names a repository path';
  for (const [key, value] of Object.entries(env)) {
    // An inherited GIT_DIR (a hook's environment) redirects git whatever its cwd: always judged.
    if (!GIT_ENV.has(key) && (AMBIENT_ENV.has(key) || !spawn.env || baseEnv[key] === value)) continue;
    if (inRepo(value)) return `unfenced child given a repository path in ${key}`;
  }
  return null;
}

// The preload library's raw operations, as read-fence.mjs classes them.
const CHILD_OP = { open: 'readFileSync', exec: 'readFileSync', stat: 'statSync', opendir: 'readdirSync' };

/**
 * Judge the preload library's raw rows (kind `child`, no reason) against each file's entry; rows
 * that already carry a reason (node-side judgements, object-store reads, unseen execs) stand.
 * A raw row with no file comes from a process no test owns (the runner starting up) and is not
 * judged: every test process and child carries GATE_READ_FENCE_FILE from read-fence.mjs.
 */
export function judgeRows(rows, list) {
  return rows.flatMap(row => {
    if (row.reason) return [row];
    if (row.kind !== 'child' || !row.file) return [];
    const entry = entryFor(list, row.file);
    if (!entry || entry.tree) return []; // reported once by the node side, which saw the file run
    const reason = judgeRead({ op: CHILD_OP[row.op] || 'readFileSync', path: row.path }, entry, row.file);
    return reason ? [{ ...row, reason }] : [];
  });
}

/**
 * Build read-fence-preload.c once per source digest into this device's cache, at the baseline
 * x86-64 level so a v2 host can load it, and return the library's path. A missing compiler or a
 * failed build refuses: the named side cannot see its children without the library.
 */
export function buildPreload({ env = process.env, spawn = childProcess.spawnSync } = {}) {
  const source = path.join(path.dirname(SELF), 'read-fence-preload.c');
  const digest = createHash('sha256').update(fs.readFileSync(source)).digest('hex').slice(0, 16);
  const cache = path.join(env.XDG_CACHE_HOME || path.join(homedir(), '.cache'), 'elohim', 'read-fence');
  const so = path.join(cache, `read-fence-preload-${digest}.so`);
  if (fs.existsSync(so)) return so;
  fs.mkdirSync(cache, { recursive: true });
  const partial = `${so}.${process.pid}.partial`;
  const cc = env.CC || 'cc';
  const built = spawn(cc, ['-O2', '-shared', '-fPIC', '-march=x86-64', '-o', partial, source, '-ldl'], { encoding: 'utf8' });
  if (built.error || built.status !== 0) {
    throw new Error(`suite-run: the read fence needs a C compiler to build ${source} (${cc}: ${built.error?.message || built.stderr?.trim() || `exit ${built.status}`}); install gcc, or set CC`);
  }
  fs.renameSync(partial, so);
  return so;
}

/** Print each violation in a fence log; exit status 1 when there is any. */
export function checkLog(logPath, out = process.stdout, list = null) {
  let text = '';
  try { text = fs.readFileSync(logPath, 'utf8'); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  const all = text.split('\n').filter(Boolean).map(line => JSON.parse(line));
  const judged = list ? judgeRows(all, list) : all.filter(row => row.reason);
  // One line per file and path or command: readFileSync and the openSync it makes are one read.
  const byWhat = new Map();
  for (const row of judged) {
    const key = JSON.stringify([row.file, row.kind, row.path ?? row.cmd, row.args ?? null, row.reason]);
    if (!byWhat.has(key)) byWhat.set(key, row);
  }
  const rows = [...byWhat.values()];
  for (const row of rows) {
    const what = row.cmd !== undefined ? `${row.cmd} ${(row.args || []).join(' ')}`.slice(0, 200)
      : `${row.path} (${row.op}${row.exe ? `, ${row.exe.split('/').pop()}` : ''})`;
    out.write(`READ-FENCE: ${row.file}: ${row.reason}: ${what}\n`);
  }
  if (rows.length) out.write(`READ-FENCE: ${rows.length} violation(s): declare each read in genesis/orchestrator/suite-reads.json, or move the file to the tree side with a reason\n`);
  return rows.length ? 1 : 0;
}

function install() {
  const root = path.resolve(process.env.GATE_READ_FENCE_ROOT);
  const gitDir = process.env.GATE_READ_FENCE_GITDIR ? path.resolve(process.env.GATE_READ_FENCE_GITDIR) : null;
  const log = process.env.GATE_READ_FENCE_LOG;
  const list = JSON.parse(fs.readFileSync(path.join(root, PROJECT_DIR, 'suite-reads.json'), 'utf8'));
  const own = process.execArgv.includes('--test-isolation=process') ? process.argv[1] : null;
  const testFile = own ? path.relative(root, path.resolve(own)).split(path.sep).join('/') : process.env.GATE_READ_FENCE_FILE;
  const appendFileSync = fs.appendFileSync;
  if (!testFile) {
    // A runner inside another test's context (NODE_TEST_CONTEXT) runs its files in this process:
    // their reads would belong to no file, so the run itself is the violation.
    if (process.env.NODE_TEST_CONTEXT) {
      appendFileSync(log, `${JSON.stringify({ file: process.argv.slice(1).join(' ').slice(0, 200), kind: 'list', path: '-', op: 'run',
        reason: 'tests ran in-process, unattributable (run with --test-isolation=process and no NODE_TEST_CONTEXT)' })}\n`);
    }
    return; // the test runner itself: it lists and loads test files, it judges nothing
  }
  process.env.GATE_READ_FENCE_FILE = testFile;
  const entry = entryFor(list, testFile);
  const baseEnv = { ...process.env };
  const seen = new Set();
  const report = row => {
    const line = JSON.stringify({ file: testFile, ...row });
    if (seen.has(line)) return;
    seen.add(line);
    appendFileSync(log, `${line}\n`);
  };
  if (!entry || entry.tree) {
    report({ kind: 'list', path: testFile, op: 'run', reason: entry ? 'tree file run under the fence' : 'test file absent from suite-reads.json' });
    return;
  }
  const rel = target => {
    try {
      if (target == null || typeof target === 'number') return null;
      let s = target instanceof URL ? fileURLToPath(target) : Buffer.isBuffer(target) ? target.toString() : String(target);
      if (s.startsWith('file:')) s = fileURLToPath(s);
      s = path.resolve(s);
      // The object store holds every commit's tree: a read there is a read no declaration names.
      if (gitDir && (s === gitDir || s.startsWith(`${gitDir}/`))) return `<git-dir>/${path.relative(gitDir, s)}`;
      if (s !== root && !s.startsWith(`${root}/`)) return null;
      const r = path.relative(root, s).split(path.sep).join('/') || '.';
      return /(^|\/)node_modules(\/|$)/.test(r) ? null : r;
    } catch { return null; }
  };
  const read = (op, target) => {
    const r = rel(target);
    if (!r) return;
    const reason = r.startsWith('<git-dir>') ? 'repository object store read' : judgeRead({ op, path: r }, entry, testFile);
    if (reason) report({ kind: 'read', op, path: r, reason });
  };
  const wrap = (holder, name, label) => {
    const orig = holder[name];
    if (typeof orig !== 'function') return;
    const wrapped = function (target, ...rest) { read(label, target); return orig.call(this, target, ...rest); };
    Object.defineProperties(wrapped, Object.getOwnPropertyDescriptors(orig));
    holder[name] = wrapped;
  };
  for (const name of ['readFileSync', 'readFile', 'existsSync', 'exists', 'statSync', 'stat', 'lstatSync', 'lstat',
    'readdirSync', 'readdir', 'opendirSync', 'opendir', 'createReadStream', 'accessSync', 'access', 'realpathSync',
    'realpath', 'globSync', 'glob', 'cpSync', 'cp', 'copyFileSync', 'copyFile', 'openSync', 'open', 'watch', 'watchFile']) {
    wrap(fs, name, name);
  }
  for (const name of ['readFile', 'readdir', 'stat', 'lstat', 'access', 'opendir', 'open', 'glob', 'cp', 'copyFile', 'realpath']) {
    wrap(fs.promises, name, `promises.${name}`);
  }
  const fence = SELF;
  const spawnCheck = (cmd, args, opts) => {
    const reason = judgeSpawn({
      cmd, args, cwd: opts?.cwd ? String(opts.cwd instanceof URL ? fileURLToPath(opts.cwd) : opts.cwd) : process.cwd(),
      env: opts?.env || null,
    }, { root, gitDir, fence, baseEnv: process.env });
    if (reason) report({ kind: 'spawn', cmd: String(cmd).slice(0, 200), args: (args || []).map(String).slice(0, 8).map(a => a.slice(0, 120)), reason });
  };
  for (const name of ['spawn', 'spawnSync', 'execFile', 'execFileSync', 'fork']) {
    const orig = childProcess[name];
    const wrapped = function (cmd, args, opts, ...rest) {
      const list = Array.isArray(args) ? args : [];
      const options = Array.isArray(args) ? opts : args;
      spawnCheck(name === 'fork' ? process.execPath : cmd, name === 'fork' ? [cmd, ...list] : list,
        name === 'fork' ? { ...options, env: options?.env || null } : options);
      return orig.call(this, cmd, args, opts, ...rest);
    };
    Object.defineProperties(wrapped, Object.getOwnPropertyDescriptors(orig));
    childProcess[name] = wrapped;
  }
  for (const name of ['exec', 'execSync']) {
    const orig = childProcess[name];
    const wrapped = function (command, opts, ...rest) {
      spawnCheck('sh', ['-c', String(command)], typeof opts === 'object' ? opts : undefined);
      return orig.call(this, command, opts, ...rest);
    };
    Object.defineProperties(wrapped, Object.getOwnPropertyDescriptors(orig));
    childProcess[name] = wrapped;
  }
  module.syncBuiltinESMExports();
  module.registerHooks({
    load(url, context, next) {
      if (url.startsWith('file:')) read('module', url);
      return next(url, context);
    },
  });
}

if (process.env.GATE_READ_FENCE_LOG && process.env.GATE_READ_FENCE_ROOT && process.argv[1] !== SELF) install();

if (process.argv[1] === SELF && process.argv[2] === 'check') {
  const list = JSON.parse(fs.readFileSync(path.join(path.dirname(SELF), 'suite-reads.json'), 'utf8'));
  process.exitCode = checkLog(process.argv[3], process.stdout, list);
}
