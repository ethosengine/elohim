import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { argumentsFor, denyWarning } from './cargo-lint.mjs';
import { createRecognizer } from './gate-failure-lines.mjs';

const wrapper = fileURLToPath(new URL('./cargo-lint.mjs', import.meta.url));
// A broken toolchain must fail the proof; only a missing cargo executable skips it.
const cargoAvailable = spawnSync('cargo', ['--version']).error?.code !== 'ENOENT';

test('removes only deny-warnings and carries metadata resolution flags', () => {
  const result = argumentsFor([
    '--manifest-path',
    'space dir/Cargo.toml',
    '--locked',
    '--offline',
    '--all-targets',
    '--',
    '-D',
    'warnings',
    '-Dclippy::all',
    '-W',
    'unused',
    '-Adead_code',
    '-Funsafe_code',
    '-Dwarnings',
    '--deny=warnings',
  ]);
  assert.deepEqual(result.metadata, [
    'metadata',
    '--no-deps',
    '--format-version',
    '1',
    '--manifest-path',
    'space dir/Cargo.toml',
    '--locked',
    '--offline',
  ]);
  assert.deepEqual(result.clippy.slice(-6), [
    '--',
    '-Dclippy::all',
    '-W',
    'unused',
    '-Adead_code',
    '-Funsafe_code',
  ]);
});

test('denied warning closes a live lint block, including ANSI and existing notes', () => {
  for (const rendered of [
    `warning: long suggestion\n${'  | example\n'.repeat(50)}`,
    'warning: unused variable\n --> src/lib.rs:2:3\n\n',
    '\x1b[33mwarning\x1b[0m: unused variable\n --> src/lib.rs:2:3\n\n',
    'warning: unused variable\n  = note: `-D unused` implied by `-D warnings`\n\n',
  ]) {
    const events = [];
    const rec = createRecognizer({
      project: 'fixture',
      emit: (e) => events.push(e),
    });
    const denied = denyWarning(rendered);
    denied.split('\n').forEach((l) => rec.push(l));
    assert.equal(events.length, 1, 'emits before end() / cargo exit');
    assert.equal(events[0].failure.class, 'lint');
    assert.equal((denied.match(/= note:/g) || []).length, 1);
  }
});

test(
  'real Cargo verdict equivalence and graph completion',
  {
    skip: !cargoAvailable && 'cargo is absent; real clippy equivalence requires Rust and clippy',
    timeout: 180_000,
  },
  (t) => {
    const dir = mkdtempSync(join(tmpdir(), 'cargo-lint-'));
    t.after(() => rmSync(dir, { recursive: true, force: true }));
    const write = (path, text) => {
      mkdirSync(resolve(dir, path, '..'), { recursive: true });
      writeFileSync(join(dir, path), text);
    };
    write('ws/Cargo.toml', '[workspace]\nmembers = ["first", "second"]\nresolver = "2"\n');
    write(
      'ws/first/Cargo.toml',
      '[package]\nname = "first"\nversion = "0.1.0"\nedition = "2021"\n',
    );
    write(
      'ws/second/Cargo.toml',
      '[package]\nname = "second"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\nfirst = { path = "../first" }\noutsider = { path = "../../outside" }\n',
    );
    write(
      'outside/Cargo.toml',
      '[package]\nname = "outsider"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n',
    );
    write('outside/src/lib.rs', 'pub fn value() -> u8 { let outside_warning = 1; 1 }\n');
    write('ws/first/src/lib.rs', 'pub fn value() -> u8 { let first_warning = 1; 1 }\n');
    write(
      'ws/second/src/lib.rs',
      'pub fn value() -> u8 { let second_warning = 1; first::value() + outsider::value() }\n',
    );
    write(
      'ws/second/tests/integration.rs',
      '#[test]\nfn works() { let integration_warning = 1; assert_eq!(second::value(), 2); }\n',
    );
    const env = {
      ...process.env,
      RUSTFLAGS: '',
      RUSTC_WRAPPER: '',
      CARGO_TARGET_DIR: join(dir, 'target'),
      CARGO_BUILD_JOBS: '1',
    };
    const run = (old, extra = [], selection = ['--workspace']) => {
      const args = ['--offline', ...selection, '--all-targets', '--', '-D', 'warnings', ...extra];
      const r = spawnSync(
        old ? 'cargo' : process.execPath,
        old ? ['clippy', ...args] : [wrapper, ...args],
        { cwd: join(dir, 'ws'), env, encoding: 'utf8', timeout: 60_000 },
      );
      assert.ifError(r.error);
      return { code: r.status, text: r.stdout + r.stderr };
    };
    const old = run(true);
    const fresh = run(false);
    assert.equal(old.code, 101, old.text);
    assert.match(old.text, /first_warning/);
    assert.doesNotMatch(old.text, /second_warning|integration_warning/);
    assert.equal(fresh.code, 1, fresh.text);
    for (const name of [
      'first_warning',
      'second_warning',
      'integration_warning',
      'outside_warning',
    ])
      assert.match(fresh.text, new RegExp(name));
    assert.match(fresh.text, /finding\(s\) in first, second; 0 target\(s\) did not compile/);
    t.diagnostic(
      `lint chain: old EXIT=101 (first only); new EXIT=1 (first + second + integration); ${fresh.text.match(/cargo-lint: .*finding.*$/m)?.[0]}`,
    );
    write('ws/first/src/lib.rs', 'pub fn value() -> u8 { 1 }\n');
    write('ws/second/src/lib.rs', 'pub fn value() -> u8 { first::value() + outsider::value() }\n');
    write(
      'ws/second/tests/integration.rs',
      '#[test]\nfn works() { assert_eq!(second::value(), 2); }\n',
    );
    for (const old of [true, false]) {
      const r = run(old);
      assert.equal(r.code, 0, r.text);
      assert.match(r.text, /outside_warning/);
    }
    t.diagnostic('outside path dependency warning only: old EXIT=0; new EXIT=0');
    write('outside/src/lib.rs', 'pub fn value() -> u8 { 1 }\n');
    for (const old of [true, false]) assert.equal(run(old).code, 0);
    t.diagnostic('clean: old EXIT=0; new EXIT=0');
    write('ws/first/src/lib.rs', 'pub fn value() -> u8 { let first_warning = 1; 1 }\n');
    const selectedOld = run(true, [], ['-p', 'second']);
    const selectedNew = run(false, [], ['-p', 'second']);
    assert.equal(
      selectedNew.code === 0,
      selectedOld.code === 0,
      selectedOld.text + selectedNew.text,
    );
    t.diagnostic(
      `selected dependent with a warning in its workspace dependency: old EXIT=${selectedOld.code}; new EXIT=${selectedNew.code}`,
    );
    for (const old of [true, false]) assert.equal(run(old, ['-A', 'unused_variables']).code, 0);
    for (const old of [true, false]) assert.equal(run(old, ['-D', 'unused_variables']).code, 101);
    write('ws/first/src/lib.rs', 'pub fn value() -> u8 { missing_symbol }\n');
    for (const old of [true, false]) {
      const r = run(old);
      assert.equal(r.code, 101, r.text);
      if (!old) assert.match(r.text, /[1-9]\d* target\(s\) did not compile/);
    }
    t.diagnostic(
      'real compile error and explicit deny: old EXIT=101; new EXIT=101; explicit allow: both EXIT=0',
    );
  },
);
