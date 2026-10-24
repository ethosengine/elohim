import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const read = path => parse(readFileSync(join(ROOT, path), 'utf8'));
const base = read('devfile.base.yaml');
const command = id => base.commands.find(item => item.id === id).exec.commandLine;

function scratch(t) {
  const dir = mkdtempSync(join(tmpdir(), 'devfile-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

test('both workspace children inherit the tools environment and commands', () => {
  for (const file of ['devfile.yaml', 'devfile_x86_v2.yaml']) {
    const child = read(file);
    assert.match(child.parent.uri, /\/dev\/devfile\.base\.yaml$/);
    assert.equal(child.parent.components?.find(item => item.name === 'tools')?.container.env, undefined);
    assert.equal(child.parent.commands, undefined);
    assert.equal(child.commands, undefined);
    assert.equal(child.events, undefined);
  }
});

test('container Rust env reaches a non-interactive background shell without profile files', () => {
  const env = Object.fromEntries(base.components.find(item => item.name === 'tools').container.env
    .map(({ name, value }) => [name, value]));
  const r = spawnSync('/bin/bash', ['--noprofile', '--norc', '-c',
    '( /bin/bash --noprofile --norc -c \'printf "%s\\n%s\\n" "$RUSTUP_HOME" "$PATH"\' ) & wait'],
  { env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  assert.equal(r.status, 0, r.stderr);
  const [rustup, path] = r.stdout.trim().split('\n');
  assert.equal(rustup, '/opt/rust/rustup');
  assert.ok(path.split(':').includes('/opt/rust/cargo/bin'));
  assert.doesNotMatch(path, /\$\{?PATH/, 'container env must not rely on shell interpolation');
});

test('postStart restores Git trust once after a HOME wipe, before hook repair', t => {
  const cwd = scratch(t);
  const env = { PATH: process.env.PATH, GIT_CONFIG_NOSYSTEM: '1', GIT_CONFIG_GLOBAL: join(cwd, 'gitconfig') };
  mkdirSync(join(cwd, '.husky'));
  // Hook repair must run after trust restoration; the helper itself has real Git tests.
  writeFileSync(join(cwd, '.husky/hooks-path.mjs'), `
    import { execFileSync } from 'node:child_process';
    import { writeFileSync } from 'node:fs';
    const trust = execFileSync('git', ['config', '--global', '--get-all', 'safe.directory'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    if (trust.trim() !== '*') process.exit(1);
    writeFileSync('hook-repair-reached', 'trusted');
  `);
  assert.equal(base.events.postStart[0], 'setup-git');
  for (let i = 0; i < 2; i++) {
    const r = spawnSync('/bin/sh', ['-c', command('setup-git')], { cwd, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    assert.equal(r.status, 0, r.stderr);
    assert.equal(readFileSync(join(cwd, 'hook-repair-reached'), 'utf8'), 'trusted');
    rmSync(join(cwd, 'hook-repair-reached'));
  }
  const trust = spawnSync('git', ['config', '--global', '--get-all', 'safe.directory'], { env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  assert.equal(trust.status, 0, trust.stderr);
  assert.equal(trust.stdout, '*\n');
});

test('peer-serve is on demand and forwards defaults and optional overrides as literal arguments', t => {
  const cwd = scratch(t);
  const stub = join(cwd, 'epr stub');
  writeFileSync(stub, `#!/bin/sh
printf '%s\\n' "$EPR_PEER_RELAY_URL" "$@"
`, { mode: 0o755 });
  writeFileSync(join(cwd, 'epr'), readFileSync(stub), { mode: 0o755 });
  assert.ok(!JSON.stringify(base.events).includes('peer-serve'));
  assert.ok(!base.commands.some(item => item.composite?.commands?.includes('peer-serve')));
  for (const overrides of [
    {},
    { EPR_BIN: stub, EPR_PEER_PORT: '8888', EPR_PEER_RELAY_URL: 'https://relay.example.test', EPR_PEER_ADDRESS: 'peer with spaces; literal' },
    { EPR_BIN: '', EPR_PEER_PORT: '', EPR_PEER_RELAY_URL: '', EPR_PEER_ADDRESS: '' },
  ]) {
    const r = spawnSync('/bin/sh', ['-c', command('peer-serve')], {
      cwd, env: { PATH: `${cwd}:${process.env.PATH}`, ...overrides }, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
    });
    assert.equal(r.status, 0, r.stderr);
    assert.deepEqual(r.stdout.trim().split('\n'), [
      overrides.EPR_PEER_RELAY_URL || 'https://relay.alpha.elohim.host',
      'peer', 'serve', '--root', '/projects/elohim', '--handle', 'matthew',
      '--listen', `0.0.0.0:${overrides.EPR_PEER_PORT || '7777'}`,
      ...(overrides.EPR_PEER_ADDRESS ? ['--peer', overrides.EPR_PEER_ADDRESS] : []),
    ]);
  }
});
