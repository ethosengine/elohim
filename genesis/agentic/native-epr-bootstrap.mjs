#!/usr/bin/env node
// One transitional caller of the native store, removed once pre-native devices have arrived.
// No store records are interpreted here. A failed build/landing leaves installation untouched.
import { mkdirSync, openSync, closeSync, readFileSync } from 'node:fs';
import { spawn, spawnSync } from 'node:child_process';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gateChildEnv, worktreeTargetDir } from '../orchestrator/gate-runner.mjs';

const root = resolve(process.argv[2]);
const script = fileURLToPath(import.meta.url);
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
      CARGO_TARGET_DIR: worktreeTargetDir(project.run.cargo.targetDir, root),
    };
    const run = (bin, args) => {
      const result = spawnSync(bin, args, { cwd: root, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
      if (result.status !== 0) throw new Error(`${bin}: ${result.stderr || result.error || result.status}`);
      return result.stdout;
    };
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
