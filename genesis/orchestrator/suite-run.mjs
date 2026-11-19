#!/usr/bin/env node
// Run the orchestrator suite from suite-reads.json (E11).
//   node suite-run.mjs named   the named files, in a view that holds only their declared reads
//   node suite-run.mjs tree    the files keyed on the whole commit tree, in this checkout
//   node suite-run.mjs all     both (package.json `test`)
// Groups run in list order, one runner per group, as the old package.json chain did. Every group
// runs even after one fails, so one run reports every red; the exit status is the first failure's.
//
// The named half is sound by absence. Its view is a directory outside every checkout, built from
// one resolved inventory (suite-inventory.mjs), the same value the gate's key hashes:
//   - genesis/orchestrator at HEAD minus its habit atoms,
//   - every declared read and every named test file outside genesis/orchestrator, with the
//     implicit install reads (suiteReadSet in gate-input-audit.mjs),
//   - inside a declared submodule, the files matching a declared `**/<name>` read, at the
//     submodule's pinned commit, read from its checkout or from the shared module store,
//   - node_modules holding only the closure of the list's `modules`, copied from this checkout's
//     install after each package is checked against pnpm-lock.yaml, and hashed into the key.
// Each entry keeps its path, mode and object id; every `cat-file` answer is checked. When a path
// in the view differs from HEAD the view is copied from the working tree instead and one line says
// so. gate-runner.mjs hands the builder what the key expects in GATE_SUITE_EXPECT (the gated
// commit's full sha and the key's inventory, package and node digests). The builder resolves its
// own inventory with the same function and says when it had to use working-tree bytes or any of
// the four differs. As its last act it writes one result to the fresh directory gate-runner.mjs
// names in GATE_SUITE_RESULT (the tested commit and digests, the key's bytes or other bytes and
// why, whether it was the full half, each group's status, the findings, and a verdict); it exits 0
// when the verdict is pass, 1 when it is fail, and a signal status (128 + n) when it was
// interrupted before it could report, which writes no result; gate-runner.mjs decides from the
// result, and no result is a red recorded nowhere. A named run
// with no expectation (a hand-run `suite-run.mjs named`) runs and records nothing: no state passes
// between runs and the exit status is the only thing the parent learns. The builder's git calls
// and the tests' environment are constructed (gitEnv, namedEnv), never inherited. After the copy,
// a symlink that resolves outside the view refuses the run. Each node group writes its own skip
// report; a missing report, or a skipped test the list's `allowedSkips` does not name, turns the
// half red. Each probe the list's `tools` names (argv, run with the view's environment) must exit
// 0, or the run is refused before any test starts.
//
// Both run modes meet here: the direct path and an ark Task each run run-local-gate.sh, then
// `just gate`, then this file, in the same checkout, and the builder's environment is constructed
// on both.
//
// Known limits, recorded rather than solved:
//   - A test that hard-codes this repository's absolute path, or any other absolute path, still
//     reaches it: nothing confines the process to the view. Landlock in ark's supervisor closes
//     that later.
//   - A read the test treats as optional passes vacuously when the file is absent from the view,
//     unless it registers a skip. The offline tracer (read-fence on ref self/nt-e11) is the
//     cross-check; it is not on the gate's path.
//   - The view is the checkout's HEAD. A pushed commit that is not HEAD is not tested here: the gate
//     says so and records nothing. A commit made while the gate runs is caught after it: the gate
//     records only when HEAD is still the sha it resolved at the start and the view's paths are
//     still clean.
//   - The gate trusts the code it runs as its own user: a named test process could overwrite the
//     result file after the builder's rename, exactly as it could write evidence directly; this
//     design does not defend against the suite's own code.
//   - By ruling, a red on the key's bytes is recorded only when HEAD and the declared paths are
//     unchanged after the run: a red from a run during which the tree moved is not attributed to it.
//   - Node's version is keyed; git, bash, python3 and the coreutils on PATH are not (every gate has
//     this limit; only rustc is keyed elsewhere).
//   - Ruled to stay named: four wall-clock upper bounds in gate-live.test.mjs and gate-tap.test.mjs
//     (each bounds a deadline the code sets; a failure there is a red, never a false green).
//   - A package is checked against the lock by name, version and store directory, and hashed file by
//     file; its importer edges and tarball integrity are not checked.
//   - The scratch root is the OS temporary directory (realpath of tmpdir()); a node_modules there or
//     in any ancestor refuses the run, since a missing import could resolve from it.
import { spawnSync } from 'node:child_process';
import {
  chmodSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  SUITE_DIR, SUITE_EXPECT_ENV, SUITE_LIST, SUITE_RESULT_ENV, blobAt, decodeExpectation, checkLockVersions, closureDigest, copyClosure, copyWorkingTree,
  dirtyInView, git, gitEnv, linksOutside, writeResult, materialise, packageClosure, suiteInventory, viewMatcher,
} from './suite-inventory.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const DAY_MS = 24 * 3600 * 1000;
const EXPECT_ENV = SUITE_EXPECT_ENV;

export { viewMatcher, dirtyInView };

export function loadSuite(text = readFileSync(join(HERE, 'suite-reads.json'), 'utf8')) {
  return JSON.parse(text);
}

/** The commands one side runs: [{ runner, cmd, args }] in group order. */
export function plan(suite, side, { skipsDir = null, reporter = './suite-skips.mjs' } = {}) {
  const steps = [];
  for (const [index, group] of suite.groups.entries()) {
    const files = group.files.filter(entry => side === 'all' || (side === 'tree') === Boolean(entry.tree)).map(entry => entry.file);
    if (!files.length) continue;
    if (group.runner === 'bash') {
      for (const file of files) steps.push({ runner: 'bash', cmd: 'bash', args: [file] });
    } else {
      const tsx = group.runner === 'node-tsx' ? ['--import', 'tsx'] : [];
      // The named half also writes each group's skipped tests to its own report, beside the spec
      // output: node truncates a destination it opens, so groups never share one.
      const skips = skipsDir ? join(skipsDir, `group-${index}.jsonl`) : null;
      const reporters = skips ? ['--test-reporter=spec', '--test-reporter-destination=stdout',
        `--test-reporter=${reporter}`, `--test-reporter-destination=${skips}`] : [];
      steps.push({ runner: group.runner, cmd: process.execPath, skips, args: [...tsx, '--test', '--test-isolation=process', ...reporters, ...files] });
    }
  }
  return steps;
}

/**
 * The named half's environment, constructed rather than inherited:
 *   PATH                     node's own directory, then /usr/local/bin:/usr/bin:/bin (device tools)
 *   HOME                     an empty directory in the scratch, so no dotfile or tool state is read
 *   TMPDIR                   a 1777 directory in the scratch, for fixtures (a test may drop its uid)
 *   LANG, TZ                 C.UTF-8 and UTC, so text tools and dates behave the same on every device
 *   GIT_CEILING_DIRECTORIES  the scratch, so git in the view finds no enclosing repository
 *   GIT_CONFIG_NOSYSTEM      1, so /etc/gitconfig is not read
 * Nothing else passes: no COMPUTE_*, GATE_*, EPR_BIN, NODE_OPTIONS or CI variable reaches a test.
 */
export function namedEnv(scratch, { execPath = process.execPath } = {}) {
  const path = [...new Set([dirname(execPath), '/usr/local/bin', '/usr/bin', '/bin'])].join(':');
  return {
    PATH: path,
    HOME: join(scratch, 'home'),
    TMPDIR: join(scratch, 'tmp'),
    LANG: 'C.UTF-8',
    TZ: 'UTC',
    GIT_CEILING_DIRECTORIES: scratch,
    GIT_CONFIG_NOSYSTEM: '1',
  };
}

/** The tool probes (argv) the list names that do not exit 0 in `env`, each with its cure. */
export function missingTools(tools = [], env) {
  return tools.flatMap(argv => {
    const r = spawnSync(argv[0], argv.slice(1), { env, stdio: 'ignore', timeout: 30_000 });
    return r.status === 0 ? [] : [`\`${argv.join(' ')}\` ${r.error ? `could not run (${r.error.code})` : `exited ${r.status ?? r.signal}`}: install a working ${argv[0]} on ${env.PATH}`];
  });
}

/** Skips the list does not allow, from the reporter's lines. `file` is relative to genesis/orchestrator. */
export function unexpectedSkips(lines, allowed = [], { dir }) {
  const rows = lines.split('\n').filter(Boolean).map(line => JSON.parse(line))
    .map(row => ({ ...row, file: row.file ? relative(dir, row.file) : '' }));
  return rows.filter(row => !allowed.some(a => a.file === row.file && a.test === row.name));
}

/**
 * The skip verdict over every node group the half ran: each group's report must exist (a missing
 * report is a red, not "no skips"), and every skip in any of them must be allowed.
 */
export function skipFindings(steps, allowed, { dir }) {
  const findings = [];
  for (const [index, step] of steps.entries()) {
    if (!step.skips) continue;
    if (!existsSync(step.skips)) { findings.push(`group ${index + 1} wrote no skip report (${step.skips})`); continue; }
    for (const row of unexpectedSkips(readFileSync(step.skips, 'utf8'), allowed, { dir })) {
      findings.push(`skipped and not in allowedSkips: ${row.file} › ${row.name} (${row.reason})`);
    }
  }
  return findings;
}

/** What differs between the view the builder made and the key's expectation, one line each. */
export function expectationDiffers(built, expected) {
  if (built.source !== 'head') return [`the view holds working-tree bytes (${built.dirty.length} path(s) differ from HEAD)`];
  return ['commit', 'inventory', 'packages', 'node'].filter(field => built[field] !== expected[field])
    .map(field => `${field}: view ${String(built[field]).slice(0, 12)}, key ${String(expected[field]).slice(0, 12)}`);
}

/**
 * The directory views are made in: the OS temporary directory, refused when it lies inside a
 * checkout of this repository. Views older than a day there are removed first.
 */
export function scratchBase({ root = ROOT, env = gitEnv(), now = Date.now() } = {}) {
  const base = realpathSync(tmpdir());
  const listed = git(['worktree', 'list', '--porcelain'], { cwd: root, env }).split('\n')
    .filter(line => line.startsWith('worktree ')).map(line => resolve(line.slice(9)));
  const owner = [root, ...listed].find(dir => base === dir || base.startsWith(`${dir}${sep}`));
  if (owner) throw new Error(`suite-run: the temporary directory ${base} is inside the checkout ${owner}; set TMPDIR outside it`);
  for (let dir = base; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'node_modules'))) throw new Error(`suite-run: ${join(dir, 'node_modules')} sits above the view, where a missing import could resolve; set TMPDIR elsewhere`);
    if (dir === dirname(dir)) break;
  }
  for (const name of readdirSync(base)) {
    if (!name.startsWith('orchestrator-view-')) continue;
    try {
      if (now - lstatSync(join(base, name)).mtimeMs > DAY_MS) rmSync(join(base, name), { recursive: true, force: true });
    } catch { /* another run's view, or gone already */ }
  }
  return base;
}

function seconds(start) {
  return (Number(process.hrtime.bigint() - start) / 1e9).toFixed(1);
}

function sizeOf(dir) {
  let bytes = 0;
  const walk = d => {
    for (const entry of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, entry.name);
      if (entry.isDirectory()) walk(p);
      else if (entry.isFile()) bytes += lstatSync(p).size;
    }
  };
  walk(dir);
  return bytes;
}

let interrupted = null;

function runSteps(steps, side, { cwd, env, spawn }) {
  // node --test skips a file it cannot find without failing, so a listed file that is absent here
  // refuses the half before anything runs.
  const absent = steps.flatMap(step => step.args.filter(a => /\.test\.(mjs|ts|sh)$/.test(a) && !existsSync(resolve(cwd, a))));
  if (absent.length) {
    process.stdout.write(`[suite] ${side}: listed but absent: ${absent.join(' ')}\n`);
    return { status: 1, groups: [], findings: [`listed but absent: ${absent.join(' ')}`] };
  }
  let status = 0;
  const groups = [];
  for (const step of steps) {
    const result = spawn(step.cmd, step.args, { cwd, stdio: 'inherit', env });
    const code = result.status ?? 1;
    groups.push({ runner: step.runner, files: step.args.filter(a => /\.test\./.test(a)).length, status: result.status, signal: result.signal || null });
    if (code !== 0) {
      process.stdout.write(`[suite] ${side} group (${step.runner}) failed with ${code}: ${step.args.filter(a => /\.test\./.test(a)).length} file(s)\n`);
      if (status === 0) status = code;
    }
    if (result.signal || interrupted) {
      process.stdout.write(`[suite] ${side}: stopped by ${result.signal || interrupted}\n`);
      return { status: status || 130, groups, findings: [`stopped by ${result.signal || interrupted}`] };
    }
  }
  return { status, groups, findings: [] };
}

/** Builds the view at `view`; returns what was built, for the comparison with the key and the plan. */
export function buildView(view, { root = ROOT, env = gitEnv() } = {}) {
  // HEAD and the list are read once; the inventory, the plan and the lock come from that read.
  const commit = git(['rev-parse', '--verify', 'HEAD^{commit}'], { cwd: root, env }).trim();
  let headList = null;
  try { headList = blobAt(root, commit, SUITE_LIST, env); } catch { /* a list not yet committed is working-tree bytes */ }
  const dirty = headList === null ? [SUITE_LIST] : dirtyInView(viewMatcher(headList), { cwd: root, env });
  let built;
  if (dirty.length === 0) {
    const inventory = suiteInventory({ root, commit, listText: headList, env });
    const files = materialise(inventory, view, { root, env });
    built = { commit, source: 'head', inventory: inventory.digest, files, listText: headList,
      lockText: blobAt(root, commit, 'pnpm-lock.yaml', env) };
  } else {
    const listText = readFileSync(join(root, SUITE_LIST), 'utf8');
    const files = copyWorkingTree(viewMatcher(listText), view, { root, env });
    built = { commit, source: 'working-tree', inventory: null, files, listText,
      lockText: readFileSync(join(root, 'pnpm-lock.yaml'), 'utf8') };
  }
  const suite = JSON.parse(built.listText);
  const closure = packageClosure(root, suite.modules || {});
  checkLockVersions(closure, built.lockText, { root });
  copyClosure(closure, view, { root });
  const outside = linksOutside(view);
  if (outside.length) throw new Error(`suite-run: ${outside.length} link(s) in the view resolve outside it: ${outside.slice(0, 3).join(', ')}`);
  return { ...built, dirty, suite, packages: closureDigest(closure, view), packageCount: closure.packages.length, node: process.version };
}

function runNamed({ spawn = spawnSync, keep = false, only = null, expected = readExpected(), result }) {
  const env = gitEnv();
  const base = scratchBase({ env });
  const scratch = mkdtempSync(join(base, 'orchestrator-view-'));
  try {
    chmodSync(scratch, 0o755);
    const view = join(scratch, 'checkout');
    const testEnv = namedEnv(scratch);
    mkdirSync(testEnv.HOME, { mode: 0o755 });
    mkdirSync(testEnv.TMPDIR);
    chmodSync(testEnv.TMPDIR, 0o1777);
    mkdirSync(join(scratch, 'skips'));
    const started = process.hrtime.bigint();
    const built = buildView(view, { env });
    Object.assign(result, { commit: built.commit, inventory: built.inventory, packages: built.packages, node: built.node });
    const mb = (sizeOf(view) / 1e6).toFixed(1);
    process.stdout.write(`[suite] named: view of ${built.commit.slice(0, 9)}: ${built.files} files, ${built.packageCount} packages, ${mb} MB, built in ${seconds(started)} s at ${view} (inventory ${String(built.inventory || 'working tree').slice(0, 12)}, packages ${built.packages.slice(0, 12)})\n`);
    if (built.dirty.length) {
      process.stdout.write(`[suite] named: ${built.dirty.length} declared path${built.dirty.length === 1 ? '' : 's'} differ from HEAD (${built.dirty.slice(0, 3).join(', ')}${built.dirty.length > 3 ? ', …' : ''}): testing the working tree's bytes\n`);
    }
    result.why = expected ? expectationDiffers(built, expected) : ['no expectation was handed'];
    result.bytes = result.why.length === 0 ? 'key' : 'other';
    if (result.bytes === 'other') process.stdout.write(`[suite] named: these are not the key's bytes (${result.why.join('; ')}): the tests run, and nothing is recorded for them\n`);
    const missing = missingTools(built.suite.tools, testEnv);
    if (missing.length) {
      process.stdout.write(`[suite] named: REFUSED: ${missing.join('; ')}\n`);
      result.findings.push(...missing.map(m => `tool: ${m}`));
      return;
    }
    const ran = process.hrtime.bigint();
    // The reporter is the view's copy of suite-skips.mjs, not this checkout's.
    const steps = plan(built.suite, 'named', { skipsDir: join(scratch, 'skips'), reporter: join(view, SUITE_DIR, 'suite-skips.mjs') }).flatMap(step => {
      if (!only) return [step];
      const args = step.args.filter(arg => !/\.test\./.test(arg) || arg === only);
      return args.some(arg => arg === only) ? [{ ...step, args }] : [];
    });
    if (only && !steps.length) throw new Error(`suite-run: ${only} is not a named file`);
    const run = runSteps(steps, 'named', { cwd: join(view, SUITE_DIR), env: testEnv, spawn });
    result.groups = run.groups;
    result.findings.push(...run.findings, ...skipFindings(steps, built.suite.allowedSkips, { dir: join(view, SUITE_DIR) }));
    for (const finding of result.findings) process.stdout.write(`[suite] named: ${finding}\n`);
    // Any group that failed or died, any finding, fails the half, whatever the other groups did.
    const groupsPassed = run.groups.length === steps.length && run.groups.every(g => g.status === 0 && !g.signal);
    result.verdict = groupsPassed && result.findings.length === 0 ? 'pass' : 'fail';
    process.stdout.write(`[suite] named: ${result.verdict === 'pass' ? 'green' : 'red'} in ${seconds(ran)} s\n`);
  } finally {
    if (!keep) rmSync(scratch, { recursive: true, force: true });
  }
}

/** The key's expectation gate-runner.mjs hands over, or null when none was handed. */
function readExpected(env = process.env) {
  return decodeExpectation(env[EXPECT_ENV]);
}

function runTree(suite, { spawn = spawnSync } = {}) {
  const env = { ...process.env };
  delete env.NODE_TEST_CONTEXT;
  delete env[EXPECT_ENV];
  delete env[SUITE_RESULT_ENV];
  const started = process.hrtime.bigint();
  const { status } = runSteps(plan(suite, 'tree'), 'tree', { cwd: HERE, env, spawn });
  process.stdout.write(`[suite] tree: ${status === 0 ? 'green' : `red (${status})`} in ${seconds(started)} s\n`);
  return status;
}

/**
 * The named half as one attempt: whatever happens, its last act is the result written to the
 * directory GATE_SUITE_RESULT names (when one is named), and it exits 0 exactly on `pass`.
 */
function named(options) {
  const result = { v: 1, commit: null, inventory: null, packages: null, node: process.version, bytes: 'other', why: [],
    full: !options.only, groups: [], findings: [], verdict: 'fail' };
  try { runNamed({ ...options, result }); }
  catch (error) { console.error(error.message); result.findings.push(`the named half could not run: ${error.message}`); result.verdict = 'fail'; }
  if (interrupted) { result.findings.push(`stopped by ${interrupted}`); result.verdict = 'fail'; }
  const dir = (process.env[SUITE_RESULT_ENV] || '').trim();
  if (dir) writeResult(dir, result);
  return result.verdict === 'pass' ? 0 : 1;
}

function main(side, flags) {
  if (!['named', 'tree', 'all'].includes(side)) throw new Error('Usage: node suite-run.mjs named|tree|all [--keep] [--only <named file>]');
  const keep = flags.includes('--keep');
  const at = flags.indexOf('--only');
  const only = at >= 0 ? flags[at + 1] : null;
  if (side === 'named') return named({ keep, only });
  if (side === 'tree') return runTree(loadSuite());
  // A named half that cannot be built still lets the tree half run and report.
  const first = named({ keep });
  const tree = runTree(loadSuite());
  return first || tree;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  // A signal stops the current group and lets the scratch view be removed before exiting.
  for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, () => { interrupted = signal; });
  try { process.exitCode = main(process.argv[2], process.argv.slice(3)); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
  if (interrupted) process.exitCode = 128 + ({ SIGHUP: 1, SIGINT: 2, SIGTERM: 15 })[interrupted];
}
