#!/usr/bin/env node
// Execute the preflight's setup cures; the preflight remains the readiness authority.
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { projectsForChanges } from './gate-runner.mjs';
import { ATTESTED_SUBMODULES, checkSubmodules, DIST_CHECKS, formatRow, runPreflight, WORKSPACE_INSTALL } from './gate-preflight.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

function capture(root, args, spawn) {
  const result = spawn('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed: ${result.error?.message || result.stderr || result.status}`);
  return String(result.stdout);
}

/** Match just gate's branch range plus staged, unstaged and untracked paths. No fetch. */
export function currentChanges({ root, spawn = spawnSync, base = 'origin/dev' }) {
  const ref = spawn('git', ['rev-parse', '--verify', base], { cwd: root, encoding: 'utf8' });
  if (ref.error || ref.status === null) throw new Error(`cannot inspect ${base}: ${ref.error?.message || ref.signal}`);
  const committed = ref.status === 0
    ? capture(root, ['diff', '--name-only', '-z', capture(root, ['merge-base', 'HEAD', base], spawn).trim(), 'HEAD'], spawn)
    : capture(root, ['diff-tree', '--root', '--no-commit-id', '--name-only', '-r', '-z', 'HEAD'], spawn);
  const paths = [committed,
    capture(root, ['diff', '--name-only', '-z'], spawn),
    capture(root, ['diff', '--cached', '--name-only', '-z'], spawn),
    capture(root, ['ls-files', '--others', '--exclude-standard', '-z'], spawn),
  ];
  return [...new Set(paths.flatMap(value => value.split('\0').filter(Boolean)))].sort();
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

function noSource(module, pin, candidates) {
  return new Error(`${module}: no local checkout holds the pinned commit ${pin.slice(0, 12)} ` +
    `(tried ${candidates.length ? candidates.join(', ') : 'none'}); fetch it from the remote with ` +
    `git submodule update --init -- ${module}, or bring a checkout that holds it`);
}

export function prep({ root = ROOT, spawn = spawnSync, exists = existsSync,
  select = projectsForChanges, preflight = runPreflight,
  distChecks = DIST_CHECKS, log = console.log } = {}) {
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
  try {
    const main = mainCheckout({ root, spawn });
    const modules = checkSubmodules({ root, spawn });
    log(formatRow(modules));
    if (modules.level === 'WARN' && !modules.why.includes('differs from the pin')) {
      throw new Error('cannot inspect submodules; refusing an incomplete gate registry');
    }
    // Where a pinned commit can come from, after the main checkout: every sibling worktree.
    const others = () => siblingCheckouts(root, spawn).filter(path => path !== main);
    for (const module of modules.missing || []) {
      const first = attempt(submoduleCommand(module, { main, exists }));
      if (!first) continue;
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
      // index holds NO entry. "Every status line is a staged deletion" alone is not proof (E6 #1):
      // status lists only changed paths, so one intentional staged deletion beside untouched files
      // looks the same. Only an empty index is no one's work, cured like a clean checkout.
      const allDeleted = lines.length > 0 && lines.every((line) => line.startsWith('D  '));
      const index = allDeleted ? spawn('git', ['-C', dir, 'ls-files'], { encoding: 'utf8' }) : null;
      const aborted = allDeleted && index?.status === 0 && !String(index.stdout ?? '').trim();
      if (status.status !== 0 || (lines.length && !aborted)) {
        log(`[prep] WARN ${module}: differs from its pin and has local changes (or cannot be read); left as is`);
        continue;
      }
      if (aborted) log(`[prep] ${module}: every tracked file is staged as deleted (an aborted checkout); restoring it`);
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
      execute(['git', ...LOCAL_ONLY, 'submodule', 'update', '--no-fetch', ...(aborted ? ['--force'] : []), '--', module]);
    }
    execute(WORKSPACE_INSTALL);

    // Load the registry only after the submodules exist: they own attested manifests.
    const changed = currentChanges({ root, spawn });
    const projects = select(root, changed);
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
    const rows = preflight({ root, projects, changed, spawn, onRow: result => log(formatRow(result)) });
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
  return failed || !executable ? 1 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = prep();
}
