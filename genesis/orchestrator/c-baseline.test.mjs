// c-baseline.mjs: a gate on a declared workspace builds C at the declared level and nothing
// above it, or the device withholds its offer (backlog steward-device-arrival-open-items row 29.2:
// a drawn epr whose ring C carried AVX2 died with SIGILL on a v2 sibling).
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { arch, platform } from 'node:os';
import { ABOVE_BASELINE, aboveBaseline, applicableTargets, cargoEnvKeys, checkCBaseline, cFamilyKey, readDeclaredTargets } from './c-baseline.mjs';

const TRIPLE = 'x86_64-unknown-linux-gnu';
const record = {
  _comment: 'row 29.2',
  [TRIPLE]: { isa: 'x86-64', cflags: '-march=x86-64', workspaces: ['elohim/eprfs'] },
};
const BASELINE_MACROS = '#define __SSE__ 1\n#define __SSE2__ 1\n#define __MMX__ 1\n#define __x86_64__ 1\n';
const V3_MACROS = `${BASELINE_MACROS}#define __AVX2__ 1\n#define __BMI2__ 1\n#define __SSE4_2__ 1\n`;
const noFiles = () => { const e = new Error('ENOENT'); e.code = 'ENOENT'; throw e; };
const probing = (stdout, calls = []) => (cmd, args, opts) => { calls.push({ cmd, args, opts }); return { status: 0, stdout, stderr: '' }; };
const base = { root: '/r', record, readFile: noFiles, hostTriple: TRIPLE };

test('macros above the x86-64 baseline are named; the baseline set is not', () => {
  assert.deepEqual(aboveBaseline(BASELINE_MACROS), []);
  assert.deepEqual(aboveBaseline(V3_MACROS), ['__AVX2__', '__BMI2__', '__SSE4_2__']);
  assert.deepEqual(aboveBaseline('#define __AVX512VL__ 1\n#define __AVX512F__ 1\n'), ['__AVX512F__', '__AVX512VL__'], 'any __AVX512*__');
  for (const name of ['__SSE3__', '__SSSE3__', '__SSE4_1__', '__POPCNT__', '__AVX__', '__BMI__', '__F16C__', '__FMA__', '__LZCNT__', '__MOVBE__', '__XSAVE__']) {
    assert.ok(ABOVE_BASELINE.includes(name), name);
  }
});

test('probe: the declared cflags are passed to CC, and a clean preprocessor is no finding', () => {
  const calls = [];
  const findings = checkCBaseline({ ...base, env: { CC: 'ccache gcc' }, spawn: probing(BASELINE_MACROS, calls) });
  assert.deepEqual(findings, []);
  assert.equal(calls[0].cmd, 'ccache');
  assert.deepEqual(calls[0].args, ['gcc', '-march=x86-64', '-dM', '-E', '-x', 'c', '/dev/null']);
  checkCBaseline({ ...base, env: {}, spawn: probing(BASELINE_MACROS, calls) });
  assert.equal(calls[1].cmd, 'cc', 'no CC probes cc');
});

test('probe: the compiler is the one cc picks, CC_<dashed triple>, CC_<underscored>, HOST_CC, CC, else cc', () => {
  // cc 1.11.1 target_envs: [CC_<target>, CC_<target_u>, <HOST|TARGET>_CC, CC], first present wins;
  // a native build (the only one checked: the host triple) reads HOST_CC, never TARGET_CC.
  const pick = env => { const calls = []; checkCBaseline({ ...base, env, spawn: probing(BASELINE_MACROS, calls) }); return calls[0].cmd; };
  const all = { CC: 'gcc-plain', HOST_CC: 'gcc-host', TARGET_CC: 'gcc-target', CC_x86_64_unknown_linux_gnu: 'gcc-under', 'CC_x86_64-unknown-linux-gnu': 'gcc-dash' };
  assert.equal(pick(all), 'gcc-dash');
  const { 'CC_x86_64-unknown-linux-gnu': _d, ...noDash } = all;
  assert.equal(pick(noDash), 'gcc-under');
  const { CC_x86_64_unknown_linux_gnu: _u, ...noTriple } = noDash;
  assert.equal(pick(noTriple), 'gcc-host');
  const { HOST_CC: _h, ...noHost } = noTriple;
  assert.equal(pick(noHost), 'gcc-plain', 'TARGET_CC is not read for a native build');
  assert.equal(pick({ TARGET_CC: 'gcc-target' }), 'cc');
  assert.equal(pick({ 'CC_x86_64-unknown-linux-gnu': ' ', CC: 'gcc-plain' }), 'cc', 'the first present wins; empty falls back to cc');
});

test('probe: a compiler value with arguments keeps every one, so -mavx2 in CC is caught', () => {
  const calls = [];
  const [f, ...rest] = checkCBaseline({ ...base, env: { HOST_CC: 'ccache gcc -mavx2' },
    spawn: (cmd, args, opts) => { calls.push({ cmd, args }); return { status: 0, stdout: args.includes('-mavx2') ? V3_MACROS : BASELINE_MACROS, stderr: '' }; } });
  assert.equal(rest.length, 0);
  assert.equal(calls[0].cmd, 'ccache');
  assert.deepEqual(calls[0].args, ['gcc', '-mavx2', '-march=x86-64', '-dM', '-E', '-x', 'c', '/dev/null']);
  assert.equal(f.check, 'probe');
  assert.match(f.why, /ccache gcc -mavx2 -march=x86-64 still defines __AVX2__/);
});

test('probe: a compiler that still defines AVX2 at the declared flags is a finding naming the macros', () => {
  const [f, ...rest] = checkCBaseline({ ...base, env: {}, spawn: probing(V3_MACROS) });
  assert.equal(rest.length, 0);
  assert.equal(f.triple, TRIPLE);
  assert.equal(f.check, 'probe');
  assert.match(f.why, /__AVX2__/);
  assert.match(f.why, /-march=x86-64/);
  assert.ok(f.cure);
});

test('probe: a compiler that cannot run is a finding, not a pass', () => {
  const [f] = checkCBaseline({ ...base, env: {}, spawn: () => ({ status: null, error: new Error('spawn cc ENOENT') }) });
  assert.equal(f.check, 'probe');
  assert.match(f.why, /ENOENT/);
});

test('env: every other C-family flags variable is a finding; CC and CXX are allowed', () => {
  const env = {
    CC: 'gcc', CXX: 'g++', CC_x86_64_unknown_linux_gnu: 'gcc',
    CFLAGS: '-O2', CXXFLAGS: '-x', CPPFLAGS: '-DX', ASMFLAGS: '-g', TARGET_CFLAGS: '-x', TARGET_CXXFLAGS: '-x',
    HOST_CFLAGS: '-x', HOST_CXXFLAGS: '-x', CRATE_CC_NO_DEFAULTS: '1',
    'CFLAGS_x86_64-unknown-linux-gnu': '-march=x86-64', 'CXXFLAGS_x86_64-unknown-linux-gnu': '-march=x86-64',
    CFLAGS_aarch64_unknown_linux_gnu: '-O2',
    PATH: '/usr/bin', RUSTFLAGS: '',
  };
  const named = checkCBaseline({ ...base, env, spawn: probing(BASELINE_MACROS) }).filter(f => f.check === 'env').map(f => f.variable).sort();
  assert.deepEqual(named, [
    'ASMFLAGS', 'CFLAGS', 'CFLAGS_aarch64_unknown_linux_gnu', 'CFLAGS_x86_64-unknown-linux-gnu', 'CPPFLAGS', 'CRATE_CC_NO_DEFAULTS', 'CXXFLAGS',
    'CXXFLAGS_x86_64-unknown-linux-gnu', 'HOST_CFLAGS', 'HOST_CXXFLAGS', 'TARGET_CFLAGS', 'TARGET_CXXFLAGS',
  ], 'the dashed spelling is not the declared variable, even with the declared value');
});

test('env: an exported but empty C-family variable is no finding (cc adds no flag for it); a set one is', () => {
  // cc 1.11.1 envflags splits the value on whitespace: '' and '  ' contribute nothing, and
  // CRATE_CC_NO_DEFAULTS='' is not true (env_value_is_true).
  const empty = {
    CFLAGS: '', CXXFLAGS: '  ', CPPFLAGS: '', ASMFLAGS: '\t', TARGET_CFLAGS: '', HOST_CXXFLAGS: '',
    'CFLAGS_x86_64-unknown-linux-gnu': '', CRATE_CC_NO_DEFAULTS: '',
  };
  assert.deepEqual(checkCBaseline({ ...base, env: empty, spawn: probing(BASELINE_MACROS) }), []);
  const set = checkCBaseline({ ...base, env: { ...empty, CFLAGS: '-O2', CRATE_CC_NO_DEFAULTS: '1' }, spawn: probing(BASELINE_MACROS) });
  assert.deepEqual(set.map(f => f.variable).sort(), ['CFLAGS', 'CRATE_CC_NO_DEFAULTS']);
});

test('env: the declared underscore variable passes only at the record\'s cflags', () => {
  const ok = { CFLAGS_x86_64_unknown_linux_gnu: '-march=x86-64', CXXFLAGS_x86_64_unknown_linux_gnu: '-march=x86-64' };
  assert.deepEqual(checkCBaseline({ ...base, env: ok, spawn: probing(BASELINE_MACROS) }), []);
  const off = checkCBaseline({ ...base, env: { ...ok, CXXFLAGS_x86_64_unknown_linux_gnu: '-march=x86-64-v3' }, spawn: probing(BASELINE_MACROS) });
  assert.equal(off.length, 1);
  assert.equal(off[0].variable, 'CXXFLAGS_x86_64_unknown_linux_gnu');
  assert.match(off[0].why, /x86-64-v3/);
  assert.match(off[0].cure, /-march=x86-64/);
});

test('cargo config: a C-family [env] key at the repo root or under the workspace is a finding', () => {
  const files = {
    '/r/.cargo/config.toml': '[registries.elohim]\nindex = "x"\n\n[env]\nRUST_LOG = "info"\nCFLAGS = "-O3"\n',
    '/r/elohim/eprfs/.cargo/config.toml': 'env.CXXFLAGS = "-mavx2"\n[env.TARGET_CFLAGS]\nvalue = "-x"\nforce = true\n',
    '/r/elohim/.cargo/config': '[env]\n"CFLAGS_x86_64_unknown_linux_gnu" = { value = "-march=x86-64", force = true }\n',
  };
  const readFile = path => { if (path in files) return files[path]; return noFiles(); };
  const found = checkCBaseline({ ...base, env: {}, readFile, spawn: probing(BASELINE_MACROS) }).filter(f => f.check === 'cargo-config');
  assert.deepEqual(found.map(f => `${f.file} ${f.variable}`).sort(), [
    '.cargo/config.toml CFLAGS',
    'elohim/.cargo/config CFLAGS_x86_64_unknown_linux_gnu',
    'elohim/eprfs/.cargo/config.toml CXXFLAGS',
    'elohim/eprfs/.cargo/config.toml TARGET_CFLAGS',
  ]);
  const clean = { '/r/.cargo/config.toml': '[profile.dev]\ndebug = "line-tables-only"\n[env]\nRUST_LOG = "info" # CFLAGS in a comment\n' };
  const quiet = checkCBaseline({ ...base, env: {}, readFile: p => (p in clean ? clean[p] : noFiles()), spawn: probing(BASELINE_MACROS) });
  assert.deepEqual(quiet, []);
});

test('cargo config: a dotted sub-key names its first segment, in every place [env] can be written', () => {
  // TOML dotted keys: `CFLAGS.value = …` in [env] is the table form `CFLAGS = { value = … }`.
  assert.deepEqual(cargoEnvKeys('[env]\nCFLAGS.value = "-mavx2"\nCFLAGS.force = true\nRUST_LOG = "x"\n'), ['CFLAGS', 'RUST_LOG']);
  assert.deepEqual(cargoEnvKeys('env.CXXFLAGS.value = "-mavx2"\n'), ['CXXFLAGS']);
  assert.deepEqual(cargoEnvKeys('env . "TARGET_CFLAGS" . value = "-x"\n'), ['TARGET_CFLAGS']);
  assert.deepEqual(cargoEnvKeys('[env.CFLAGS]\nvalue = "-mavx2"\n[ env . "ASMFLAGS" ]\nvalue = "-x"\n'), ['CFLAGS', 'ASMFLAGS']);
  assert.deepEqual(cargoEnvKeys('[env.CPPFLAGS.extra]\nvalue = "-x"\n'), ['CPPFLAGS']);
  assert.deepEqual(cargoEnvKeys('env = { CFLAGS.value = "-mavx2", RUST_LOG = "x" }\n'), ['CFLAGS', 'RUST_LOG']);
  assert.deepEqual(cargoEnvKeys('[env]\n"CFLAGS.odd" = "-x"\n'), ['CFLAGS.odd'], 'a quoted key keeps its dot');
  const files = { '/r/elohim/eprfs/.cargo/config.toml': '[env]\nCFLAGS.value = "-mavx2"\nCFLAGS.force = true\n' };
  const found = checkCBaseline({ ...base, env: {}, readFile: p => (p in files ? files[p] : noFiles()), spawn: probing(BASELINE_MACROS) });
  assert.deepEqual(found.map(f => `${f.check} ${f.variable}`), ['cargo-config CFLAGS'], 'one finding per key and file');
});

test('targets: only a declared target covering a selected workspace, on its own host triple, is checked', () => {
  assert.deepEqual(applicableTargets(record, { workspaces: ['elohim/eprfs'], hostTriple: TRIPLE }).map(t => t.triple), [TRIPLE]);
  assert.deepEqual(applicableTargets(record, { workspaces: ['elohim/elohim-storage'], hostTriple: TRIPLE }), []);
  assert.deepEqual(applicableTargets(record, { hostTriple: 'aarch64-unknown-linux-gnu' }), [], 'a host of another triple builds another target');
  assert.deepEqual(applicableTargets(record, { triple: 'aarch64-unknown-linux-gnu', hostTriple: TRIPLE }), []);
  assert.deepEqual(applicableTargets(null, {}), []);
  const spawn = () => { throw new Error('nothing to probe'); };
  assert.deepEqual(checkCBaseline({ ...base, env: { CFLAGS: '-O3' }, spawn, workspaces: ['elohim/elohim-storage'] }), []);
});

test('the record is read from pool-policy.json declared_targets; absent is null', () => {
  const policy = { cargo_env_overrides: { '*': {} }, declared_targets: record };
  assert.deepEqual(readDeclaredTargets('/r', () => JSON.stringify(policy)), record);
  assert.equal(readDeclaredTargets('/r', () => '{}'), null);
  assert.equal(readDeclaredTargets('/r', noFiles), null);
});

test('C-family keys: flags variables in every spelling, never the compiler choice', () => {
  for (const key of ['CFLAGS', 'CXXFLAGS_x86_64-unknown-linux-gnu', 'HOST_CXXFLAGS', 'ASMFLAGS', 'CRATE_CC_NO_DEFAULTS', 'TARGET_ASMFLAGS']) assert.ok(cFamilyKey(key), key);
  for (const key of ['CC', 'CXX', 'CC_x86_64_unknown_linux_gnu', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'LDFLAGS_X']) assert.equal(cFamilyKey(key), false, key);
});

test('the orchestrator suite runs this file, c-baseline-pin.test.mjs and gate-preflight.test.mjs', () => {
  // E11: the suite is suite-reads.json; package.json test runs both of its halves.
  const suite = JSON.parse(readFileSync(new URL('./suite-reads.json', import.meta.url), 'utf8'));
  const listed = suite.groups.flatMap(group => group.files.map(entry => entry.file));
  for (const file of ['c-baseline.test.mjs', 'gate-preflight.test.mjs', 'c-baseline-pin.test.mjs']) assert.ok(listed.includes(file), file);
});

// The machine fact the record rests on, measured on this device's compiler, never a fixture.
const hasCc = spawnSync('cc', ['--version'], { encoding: 'utf8' });
const x86Linux = arch() === 'x64' && platform() === 'linux';
const preprocess = flags => spawnSync('cc', [...flags, '-dM', '-E', '-x', 'c', '/dev/null'], { encoding: 'utf8' });

test('REAL: -march=x86-64 defines no macro above the baseline on this compiler', { skip: hasCc.status !== 0 ? 'no cc installed' : !x86Linux ? 'not an x86_64 Linux host' : false }, () => {
  const r = preprocess(['-march=x86-64']);
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(aboveBaseline(r.stdout), []);
  assert.deepEqual(checkCBaseline({ root: '/r', env: {}, record, readFile: noFiles, hostTriple: TRIPLE, spawn: spawnSync }), []);
});

// The default level is the compiler's own configuration, not its distribution: read it, never infer
// it from a version string (Fedora's gcc prints the same "Red Hat" marker with a plain x86-64
// default). The AVX2 claim holds only where the default is v3 or above, as RHEL 10's gcc 14 is.
const defaultMarch = /^\s*-march=\s+(\S+)/m.exec(String(spawnSync('cc', ['-Q', '--help=target'], { encoding: 'utf8' }).stdout || ''))?.[1] ?? null;
const defaultAboveV2 = /^x86-64-v[34]$/.test(defaultMarch ?? '');
test('REAL: a compiler whose default -march is x86-64-v3 or above defines __AVX2__ with no flag (the cause of row 29.2)', { skip: hasCc.status !== 0 ? 'no cc installed' : !x86Linux ? 'not an x86_64 Linux host' : !defaultAboveV2 ? `this compiler defaults to -march=${defaultMarch ?? 'unknown'}` : false }, () => {
  const r = preprocess([]);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(aboveBaseline(r.stdout).includes('__AVX2__'), aboveBaseline(r.stdout).join(' '));
  const unflagged = { [TRIPLE]: { ...record[TRIPLE], cflags: '' } };
  const [f] = checkCBaseline({ root: '/r', env: {}, record: unflagged, readFile: noFiles, hostTriple: TRIPLE, spawn: spawnSync });
  assert.equal(f?.check, 'probe');
  assert.match(f.why, /__AVX2__/);
});
