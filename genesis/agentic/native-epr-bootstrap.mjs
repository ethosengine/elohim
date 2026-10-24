#!/usr/bin/env node
// One transitional caller of the native store, removed once pre-native devices have arrived.
// No store records are interpreted here. A failed build/landing leaves installation untouched.
import { mkdirSync, openSync, closeSync, readFileSync, realpathSync, existsSync } from 'node:fs';
import { spawn, spawnSync } from 'node:child_process';
import { resolve, join, dirname, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gateChildEnv, worktreeTargetDir } from '../orchestrator/gate-runner.mjs';

const root = realpathSync(resolve(process.argv[2]));
const script = fileURLToPath(import.meta.url);
// Match device_key::resolve_path_with and git::trust_file before asking Git anything.
function trustFile(env) {
  if (env.ELOHIM_DEVICE_KEY_FILE) {
    const key = env.ELOHIM_DEVICE_KEY_FILE;
    if (!isAbsolute(key)) throw new Error('ELOHIM_DEVICE_KEY_FILE must be absolute and outside a checkout');
    for (let dir = dirname(key); ; dir = dirname(dir)) {
      if (existsSync(join(dir, '.git'))) throw new Error('ELOHIM_DEVICE_KEY_FILE must be outside a checkout');
      if (dir === dirname(dir)) break;
    }
    return join(dirname(key), 'trusted-checkouts');
  }
  const config = env.XDG_CONFIG_HOME && isAbsolute(env.XDG_CONFIG_HOME)
    ? env.XDG_CONFIG_HOME : env.HOME ? join(env.HOME, '.config') : null;
  if (!config) throw new Error('device config unavailable: set HOME or absolute XDG_CONFIG_HOME');
  return join(config, 'elohim/device/trusted-checkouts');
}
const state = join(root, 'genesis/local-dev/native-epr-bootstrap');
mkdirSync(state, { recursive: true });
if (!process.argv.includes('--worker')) {
  const log = openSync(join(state, 'bootstrap.log'), 'a');
  const child = spawn('flock', ['-n', join(state, 'worker.lock'), process.execPath, script, root, '--worker'], {
    cwd: root, detached: true, stdio: ['ignore', log, log],
  });
  child.on('error', error => process.stderr.write(`native evaluator bootstrap: ${error.message}\n`));
  child.unref();
  closeSync(log);
  process.stderr.write('native evaluator bootstrapping\n');
} else {
  try {
    const manifest = JSON.parse(readFileSync(join(root, 'elohim/eprfs/build-manifest.json')));
    const project = { name: 'eprfs', ...manifest.gate.projects.eprfs };
    const base = gateChildEnv(project, process.env, root);
    const env = { ...base, ...JSON.parse(base.GATE_CARGO_ENV || '{}'),
      RUSTFLAGS: project.run.cargo.rustflags,
    };
    const declaration = trustFile(env);
    let trusted = false;
    try { trusted = readFileSync(declaration, 'utf8').split(/\r?\n/).includes(root); }
    catch { /* Missing or unreadable declarations grant no checkout trust. */ }
    if (trusted) {
      const index = Number(env.GIT_CONFIG_COUNT || 0);
      if (!Number.isSafeInteger(index) || index < 0) throw new Error('invalid GIT_CONFIG_COUNT');
      env[`GIT_CONFIG_KEY_${index}`] = 'safe.directory';
      env[`GIT_CONFIG_VALUE_${index}`] = root;
      env.GIT_CONFIG_COUNT = String(index + 1);
    } else {
      const quoted = "'" + root.replaceAll("'", "'\\''") + "'";
      process.stderr.write(`checkout is not device-trusted; cure: epr device trust ${quoted}; pre-native cure: git config --global --add safe.directory ${quoted}\n`);
    }
    const run = (bin, args) => {
      if (bin === 'git' && trusted) args = ['-c', `safe.directory=${root}`, ...args];
      const result = spawnSync(bin, args, { cwd: root, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
      if (result.error || result.status !== 0) throw new Error(`${bin}: ${result.stderr || result.error || result.status}`);
      return result.stdout;
    };
    env.CARGO_TARGET_DIR = worktreeTargetDir(project.run.cargo.targetDir, root, () => ({
      gitDir: run('git', ['rev-parse', '--path-format=absolute', '--git-dir']).trim(),
      commonDir: run('git', ['rev-parse', '--path-format=absolute', '--git-common-dir']).trim(),
    }));
    // Capture provenance before compiling. A checkout move during compilation cannot be landed.
    const commit = run('git', ['rev-parse', 'HEAD']).trim();
    const before = run('git', ['status', '--porcelain', '--untracked-files=normal']);
    run('cargo', ['build', '--manifest-path', 'elohim/eprfs/Cargo.toml',
      '-p', 'elohim-epr-cli', '-p', 'elohim-device-runtime', '--bins']);
    if (before !== run('git', ['status', '--porcelain', '--untracked-files=normal'])
      || commit !== run('git', ['rev-parse', 'HEAD']).trim()) {
      throw new Error('checkout changed during build; retry from stable sources');
    }
    const binary = join(env.CARGO_TARGET_DIR, 'debug/epr');
    const call = args => run(binary, ['bin', ...args, '--tool', 'epr', '--repo', root]);
    // The native source-closure reader owns cleanliness; unrelated hook ledgers may be dirty.
    if (!JSON.parse(call(['source-state'])).clean) throw new Error('source closure dirty; landing refused');
    call(['migrate']);
    const record = JSON.parse(call(['ingest', binary, '--source-commit', commit]));
    call(['land', record.cid, '--commit', commit]);
    call(['install-launcher', '--launcher', join(env.CARGO_TARGET_DIR, 'debug/bin-launch')]);
    process.stdout.write(`native evaluator ready: ${record.cid}\n`);
  } catch (error) {
    process.stderr.write(`native evaluator bootstrap: ${error.message}\n`);
    process.exitCode = 1;
  }
}
