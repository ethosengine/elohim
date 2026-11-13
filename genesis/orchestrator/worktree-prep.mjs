#!/usr/bin/env node
// Execute the preflight's setup cures; the preflight remains the readiness authority.
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { projectsForChanges } from './gate-runner.mjs';
import { checkSubmodules, DIST_CHECKS, formatRow, runPreflight, WORKSPACE_INSTALL } from './gate-preflight.mjs';

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
  const args = ['git', '-c', 'protocol.allow=never', '-c', 'protocol.file.allow=always'];
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

export function prep({ root = ROOT, spawn = spawnSync, exists = existsSync,
  select = projectsForChanges, preflight = runPreflight,
  distChecks = DIST_CHECKS, log = console.log } = {}) {
  const execute = ([command, ...args]) => {
    log(`[prep] ${[command, ...args].join(' ')}`);
    const result = spawn(command, args, { cwd: root, stdio: 'inherit' });
    if (result.status !== 0) throw new Error(`${command} failed (${result.status ?? result.signal ?? result.error?.message ?? 'could not start'})`);
  };
  let failed = false;
  try {
    const main = mainCheckout({ root, spawn });
    const modules = checkSubmodules({ root, spawn });
    log(formatRow(modules));
    if (modules.level === 'WARN' && !modules.why.includes('differs from the pin')) {
      throw new Error('cannot inspect submodules; refusing an incomplete gate registry');
    }
    for (const module of modules.missing || []) execute(submoduleCommand(module, { main, exists }));
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
