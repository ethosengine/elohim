#!/usr/bin/env node
// Execute the preflight's setup cures; the preflight remains the readiness authority.
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { projectsForChanges } from './gate-runner.mjs';
import { declaredPinStores } from './suite-inventory.mjs';
import { ATTESTED_SUBMODULES, checkSubmodules, DIST_CHECKS, formatRow, manifestNames, runPreflight, tmpdirRepository, WORKSPACE_INSTALL } from './gate-preflight.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

function capture(root, args, spawn) {
  const result = spawn('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed: ${result.error?.message || result.stderr || result.status}`);
  return String(result.stdout);
}

/** Match just gate's branch range plus staged, unstaged and untracked paths. No fetch. */
export function currentChanges(options) {
  return currentChangeList(options).paths;
}

/**
 * The same paths, and whether they are the whole of this worktree's change. When `base` does not
 * resolve the committed part is the tip commit alone: enough to select by, as before, but a pin
 * move or a Cargo.toml in an earlier commit of the branch is not in it. `exact: false` says so,
 * and a caller that skips a check by the list must then treat it as no list (the hook's
 * GATE_NARROW_BASE=none).
 */
export function currentChangeList({ root, spawn = spawnSync, base = 'origin/dev' }) {
  const ref = spawn('git', ['rev-parse', '--verify', base], { cwd: root, encoding: 'utf8' });
  if (ref.error || ref.status === null) throw new Error(`cannot inspect ${base}: ${ref.error?.message || ref.signal}`);
  const exact = ref.status === 0;
  const committed = exact
    ? capture(root, ['diff', '--name-only', '--no-renames', '-z', capture(root, ['merge-base', 'HEAD', base], spawn).trim(), 'HEAD'], spawn)
    : capture(root, ['diff-tree', '--root', '--no-commit-id', '--name-only', '-r', '-z', 'HEAD'], spawn);
  const paths = [committed,
    // --no-renames: a file renamed away is listed under its old name too, so a Cargo.toml or a
    // deny.toml moved aside still counts as a change to it.
    capture(root, ['diff', '--name-only', '--no-renames', '-z'], spawn),
    capture(root, ['diff', '--cached', '--name-only', '--no-renames', '-z'], spawn),
    capture(root, ['ls-files', '--others', '--exclude-standard', '-z'], spawn),
  ];
  return { paths: [...new Set(paths.flatMap(value => value.split('\0').filter(Boolean)))].sort(), exact };
}

/** A linked checkout shares the main checkout's .git directory, not its worktree git dir. */
export function mainCheckout({ root, spawn = spawnSync }) {
  const common = resolve(root, capture(root, ['rev-parse', '--git-common-dir'], spawn).trim());
  const own = resolve(root, capture(root, ['rev-parse', '--git-dir'], spawn).trim());
  if (common === own) return null;
  if (basename(common) !== '.git') throw new Error(`cannot locate main checkout from common git directory ${common}`);
  return dirname(common);
}

export function submoduleCommand(module, { main, exists = existsSync }) {
  // --reference alone still clones the remote URL. Use the local checkout as the clone
  // source too, forbid network protocols and refuse an unavailable pin instead of fetching.
  const args = ['git', ...LOCAL_ONLY];
  let reference;
  if (main) {
    reference = join(main, module);
    if (!exists(join(reference, '.git'))) throw new Error(`local submodule objects unavailable: ${reference}; initialize it in the main checkout first`);
    args.push('-c', `submodule.${module}.url=${reference}`);
  }
  args.push('submodule', 'update', '--init', '--no-fetch');
  if (reference) args.push('--reference', reference);
  return [...args, '--', module];
}

const LOCAL_ONLY = ['-c', 'protocol.allow=never', '-c', 'protocol.file.allow=always'];

/** The commit `module` is pinned to at HEAD (its gitlink), or null when it cannot be read. */
export function pinOf(root, module, spawn = spawnSync) {
  const r = spawn('git', ['-C', root, 'ls-tree', 'HEAD', '--', module], { encoding: 'utf8' });
  const match = r.status === 0 && /^160000 commit ([0-9a-f]{40,64})\t/.exec(String(r.stdout));
  return match ? match[1] : null;
}

/**
 * The attested submodules checked out at another commit than their pin (`+` in `git submodule
 * status`). Read here rather than taken from the preflight row, so prep stays outside the
 * gate-*.mjs files that fence sibling reuse.
 */
export function movedSubmodules(root, spawn = spawnSync) {
  const r = spawn('git', ['-C', root, 'submodule', 'status', '--', ...ATTESTED_SUBMODULES], { encoding: 'utf8' });
  if (r.status !== 0) return [];
  return String(r.stdout).split('\n').filter(line => line.startsWith('+')).map(line => line.trim().split(/\s+/)[1]);
}

/** Every other checkout of this repository (the main one and each linked worktree), never `root`. */
export function siblingCheckouts(root, spawn = spawnSync) {
  const r = spawn('git', ['-C', root, 'worktree', 'list', '--porcelain'], { encoding: 'utf8' });
  if (r.status !== 0) return [];
  return String(r.stdout).split('\n')
    .filter(line => line.startsWith('worktree '))
    .map(line => resolve(line.slice('worktree '.length)))
    .filter(path => path !== resolve(root));
}

/**
 * Row 11 (a): the first checkout in `candidates` whose copy of `module` holds commit `pin`, or
 * null. Read with `cat-file -e` in that checkout's submodule; nothing is fetched or written.
 */
export function pinSource(module, pin, { candidates, spawn = spawnSync, exists = existsSync }) {
  for (const candidate of candidates) {
    const dir = join(candidate, module);
    if (!exists(join(dir, '.git'))) continue;
    const held = spawn('git', ['-C', dir, 'cat-file', '-e', `${pin}^{commit}`], { encoding: 'utf8' });
    if (held.status === 0) return candidate;
  }
  return null;
}

/**
 * Whether the work in this worktree needs `module`. The attested submodules (the list the pre-push
 * hook refuses without) always; any other only when a selected project reads it: the orchestrator
 * gate's suite view reads every declared pin, and a project's manifest may name a submodule as a
 * step source. Null `projects` (selection unavailable) and an unreadable manifest mean needed.
 */
export function neededSubmodule(module, projects, { read = readFileSync, root = ROOT } = {}) {
  if (ATTESTED_SUBMODULES.includes(module) || !projects) return true;
  if (projects.some(project => project.name === 'orchestrator')) return true;
  return projects.some(project => manifestNames(module, project, { read, root }));
}

function noSource(module, pin, candidates) {
  return new Error(`${module}: no local checkout holds the pinned commit ${pin.slice(0, 12)} ` +
    `(tried ${candidates.length ? candidates.join(', ') : 'none'}); fetch it from the remote with ` +
    `git submodule update --init -- ${module}, or bring a checkout that holds it`);
}

export function prep({ root = ROOT, spawn = spawnSync, exists = existsSync,
  select = projectsForChanges, preflight = runPreflight, declaredPins = declaredPinStores,
  distChecks = DIST_CHECKS, tmp = tmpdirRepository, log = console.log } = {}) {
  const attempt = ([command, ...args]) => {
    log(`[prep] ${[command, ...args].join(' ')}`);
    const result = spawn(command, args, { cwd: root, stdio: 'inherit' });
    return result.status === 0 ? null : `${command} failed (${result.status ?? result.signal ?? result.error?.message ?? 'could not start'})`;
  };
  const execute = command => {
    const failure = attempt(command);
    if (failure) throw new Error(failure);
  };
  let failed = false;
  const refused = []; // moved submodules prep would not touch (E8 #2): prep exits 1 naming each
  // Said first, and whatever is selected: the preflight refuses a push for this only once a gate is
  // selected, and a device should learn it before then. prep removes nothing.
  try {
    const inside = tmp();
    if (inside) log(`[prep] WARN ${inside.why}; a push that selects any gate is refused for it.${inside.placeholder ? ` ${inside.placeholder}.` : ''} Cure: ${inside.cure} (prep does not remove it)`);
  } catch { /* the preflight's own row still says it */ }
  try {
    const main = mainCheckout({ root, spawn });
    // Thin-device station 1: an absent attested submodule is brought in only when this worktree's
    // own changes need it (gate-preflight attestedNeed). The selection is read before any checkout,
    // from the manifests that are here; when it or the change list cannot be read, all are required.
    let early = null;
    let modules = checkSubmodules({ root, spawn });
    if (modules.missing?.length) {
      let before = null;
      try {
        const list = currentChangeList({ root, spawn });
        early = select(root, list.paths);
        before = list.exact ? list.paths : null; // a tip-only list requires every one
      } catch { early = null; before = null; }
      modules = checkSubmodules({ root, spawn, projects: early, changed: before });
      // A checkout adds manifests to the registry: the selection is read again once they exist.
      if (modules.missing?.length) early = null;
    }
    log(formatRow(modules));
    if (modules.level === 'WARN' && !modules.why.includes('differs from the pin')) {
      throw new Error('cannot inspect submodules; refusing an incomplete gate registry');
    }
    // Where a pinned commit can come from, after the main checkout: every sibling worktree.
    const others = () => siblingCheckouts(root, spawn).filter(path => path !== main);
    const initModule = module => {
      const first = attempt(submoduleCommand(module, { main, exists }));
      if (!first) return;
      // Row 11 (a): the main checkout's copy lacks the pin ("unable to read tree"). Try each
      // sibling worktree's copy as the local source, and say which one served it.
      const pin = pinOf(root, module, spawn);
      if (!pin) throw new Error(`${first}; and the pin of ${module} could not be read`);
      const candidates = others();
      const source = pinSource(module, pin, { candidates, spawn, exists });
      if (!source) throw noSource(module, pin, [main, ...candidates].filter(Boolean));
      log(`[prep] ${module}: the main checkout lacks pin ${pin.slice(0, 12)}; using ${source}`);
      // The failed first attempt leaves a submodule repository behind, and a second `update --init`
      // reuses it rather than cloning from the new source: fetch the pin into it first.
      if (exists(join(root, module, '.git'))) {
        execute(['git', '-C', join(root, module), ...LOCAL_ONLY, 'fetch', '--no-tags', join(source, module), pin]);
      }
      execute(submoduleCommand(module, { main: source, exists }));
    };
    for (const module of modules.missing || []) initModule(module);
    // The orchestrator gate's view also reads the submodules its read list declares. Bring each into
    // an object store unless its pinned commit is already readable (brit's store is several GB: never
    // clone what is there). The attested three are handled above and by the moved-pin step below.
    // Selection is read here only to learn which declared submodules the work needs; it is
    // computed once and the later log line reuses it. A failure leaves every one needed.
    if (!early) { try { early = select(root, currentChanges({ root, spawn })); } catch { early = null; } }
    for (const { path: module, oid, store } of declaredPins({ root })) {
      if (store || ATTESTED_SUBMODULES.includes(module)) continue;
      if (!neededSubmodule(module, early, { root })) {
        // Not needed and held by no local checkout: never fetch; leave it uninitialised.
        const holders = [main, ...others()].filter(Boolean);
        if (!pinSource(module, oid, { candidates: holders, spawn, exists })) {
          log(`[prep] ${module}: not needed for this worktree and not present locally; left uninitialised`);
          continue;
        }
      }
      log(`[prep] ${module}: pin ${oid.slice(0, 12)} is in no object store here; initializing it`);
      initModule(module);
    }
    // Row 11 (b): an initialized submodule left at another commit (a rebase moved the gitlink)
    // makes the gate test bytes the push does not carry. A clean one is moved to its pin;
    // a dirty one is someone's work and keeps the warning.
    // Read the moved set directly: the preflight row stops at the first missing submodule and
    // then never names a moved one (live, 2026-11-18: sophia missing hid rakia at another commit).
    for (const module of movedSubmodules(root, spawn)) {
      const dir = join(root, module);
      const status = spawn('git', ['-C', dir, 'status', '--porcelain'], { encoding: 'utf8' });
      const lines = String(status.stdout ?? '').split('\n').filter(Boolean);
      // Row 11's signature: an aborted checkout leaves every tracked file staged as deleted, so its
      // index holds NO entry. Status lists only changed paths, so staged deletions beside untouched
      // files are someone's work (E6 #1) and keep the warning. An EMPTY index is still ambiguous
      // (E8 #2): an aborted checkout and a deliberate deletion of every file look the same, so prep
      // never forces it; it names the state and the one command that would, and refuses this
      // submodule.
      const allDeleted = lines.length > 0 && lines.every((line) => line.startsWith('D  '));
      const index = allDeleted ? spawn('git', ['-C', dir, 'ls-files'], { encoding: 'utf8' }) : null;
      const emptyIndex = allDeleted && index?.status === 0 && !String(index.stdout ?? '').trim();
      if (emptyIndex) {
        refused.push(module);
        log(`[prep] REFUSED ${module}: differs from its pin and every tracked file (${lines.length}) is staged as deleted, an aborted checkout or a deliberate deletion; prep forces nothing. To restore the pin and discard those deletions: git submodule update --force -- ${module}`);
        continue;
      }
      if (status.status !== 0 || lines.length) {
        log(`[prep] WARN ${module}: differs from its pin and has local changes (or cannot be read); left as is`);
        continue;
      }
      const pin = pinOf(root, module, spawn);
      if (!pin) throw new Error(`the pin of ${module} could not be read`);
      if (!pinSource(module, pin, { candidates: [root], spawn, exists })) {
        const candidates = [main, ...others()].filter(Boolean);
        const source = pinSource(module, pin, { candidates, spawn, exists });
        if (!source) throw noSource(module, pin, candidates);
        log(`[prep] ${module}: pin ${pin.slice(0, 12)} fetched from ${source}`);
        execute(['git', '-C', dir, ...LOCAL_ONLY, 'fetch', '--no-tags', join(source, module), pin]);
      }
      log(`[prep] ${module}: moving the clean checkout to its pin ${pin.slice(0, 12)}`);
      execute(['git', ...LOCAL_ONLY, 'submodule', 'update', '--no-fetch', '--', module]);
    }
    execute(WORKSPACE_INSTALL);

    // Load the registry only after the submodules exist: they own attested manifests.
    const list = currentChangeList({ root, spawn });
    const changed = list.paths;
    const projects = early ?? select(root, changed);
    log(`[prep] selected projects: ${projects.map(project => project.name).join(' ') || '(none)'}`);
    for (const check of distChecks) {
      // Re-evaluate after each cure: one library build can satisfy another prerequisite.
      const result = check({ root, projects });
      if (result) log(formatRow(result));
      if (result?.level === 'REFUSED') {
        if (!result.commands?.length) throw new Error(`no executable cure for ${result.check}`);
        for (const command of result.commands) execute(command);
      }
    }
    if (!list.exact) log('[prep] origin/dev does not resolve here, so only the tip commit\'s paths are known: no check is skipped by the change list');
    const rows = preflight({ root, projects, changed: list.exact ? changed : null, spawn, onRow: result => log(formatRow(result)) });
    failed = rows.some(result => result.level === 'REFUSED');
    log(`[prep] preflight ${failed ? 'REFUSED' : 'clear'}${rows.some(result => result.level === 'WARN') ? ' (warnings above)' : ''}`);
  } catch (error) {
    failed = true;
    log(`[prep] REFUSED: ${error.message}`);
  }
  // Last assertion even after a failed cure. A missing hook must never be a silent pass.
  const hook = spawn('test', ['-x', '.husky/_/pre-push'], { cwd: root });
  const executable = hook.status === 0;
  log(executable
    ? '[prep] ok: .husky/_/pre-push is executable'
    : '[prep] REFUSED: .husky/_/pre-push is missing or not executable; run pnpm install with Husky enabled');
  return failed || refused.length || !executable ? 1 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = prep();
}
