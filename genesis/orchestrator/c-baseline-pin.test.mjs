// Backlog steward-device-arrival-open-items row 29.2: the gate builds C at ONE declared
// level, the Rust target's baseline (-march=x86-64 on x86_64-unknown-linux-gnu). RHEL 10
// gcc defaults to -march=x86-64-v3; cc 1.2.65 passed no -march, so ring's curve25519.c
// carried AVX2 into a drawn epr and a v2 sibling died with SIGILL. cc >= 1.11.0 pushes
// -march=x86-64 itself ("Match rustc's CPU"). This test pins every home of that decision:
// the pool-policy record, the producer recording the record's two variables, the cc floor in each
// native lock, the cc source still carrying the push, and no tracked build config raising
// the level behind the record's back. A drift in any one of them fails here.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const read = rel => readFileSync(resolve(ROOT, rel), 'utf8');

const TRIPLE = 'x86_64-unknown-linux-gnu';
const CFLAGS_VAR = 'CFLAGS_x86_64_unknown_linux_gnu';
const CXXFLAGS_VAR = 'CXXFLAGS_x86_64_unknown_linux_gnu';
const EXPECTED_TARGET = { isa: 'x86-64', cflags: '-march=x86-64', workspaces: ['elohim/eprfs'] };

// ---- pure checks (proved against fixtures below, then applied to the repository) ----

/** Problems with pool-policy's declared_targets record; empty when it has exactly the decided shape. */
export function declaredTargetProblems(policy) {
  const problems = [];
  const overrides = policy?.cargo_env_overrides ?? {};
  if (JSON.stringify(overrides).includes('"declared_targets"') || 'declared_targets' in overrides) {
    problems.push('declared_targets sits inside cargo_env_overrides (hc-mesh.sh reads that block)');
  }
  const record = policy?.declared_targets;
  if (record === undefined) return [...problems, 'pool-policy.json has no top-level declared_targets'];
  if (record === null || typeof record !== 'object' || Array.isArray(record)) {
    return [...problems, 'declared_targets is not an object'];
  }
  const keys = Object.keys(record).sort();
  if (JSON.stringify(keys) !== JSON.stringify(['_comment', TRIPLE].sort())) {
    problems.push(`declared_targets keys are [${keys.join(', ')}], expected [_comment, ${TRIPLE}]`);
  }
  if (typeof record._comment !== 'string' || record._comment.trim() === '') {
    problems.push('declared_targets._comment is not a non-empty paragraph');
  }
  const target = record[TRIPLE];
  try {
    assert.deepStrictEqual(target, EXPECTED_TARGET);
  } catch {
    problems.push(`declared_targets["${TRIPLE}"] is ${JSON.stringify(target)}, expected ${JSON.stringify(EXPECTED_TARGET)}`);
  }
  if (target && typeof target === 'object' && target.cflags !== `-march=${target.isa}`) {
    problems.push(`cflags ${target.cflags} does not name isa ${target.isa}`);
  }
  return problems;
}

const uncommented = text =>
  text
    .split('\n')
    .filter(line => !/^\s*#/.test(line))
    .join('\n');

const globMatches = (pattern, name) =>
  new RegExp(`^${pattern.replace(/[.+^${}()[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.')}$`).test(name);
const RECORDS_NAME = /^recorded\+=\(--env "\$name=\$\{!name\}"\)\s*;;\s*$/;
const recordsExplicitly = (body, name) => body.trim().startsWith(`recorded+=(--env "${name}=\${${name}}")`);

/**
 * Is `name` recorded as a describe --env value? In the recording `case "$name"` block (one
 * whose arms append to `recorded`), the FIRST arm whose patterns match `name` (bash takes
 * the first match) must append `--env "$name=${!name}"` or `--env "<name>=${<name>}"`.
 */
function recordsVariable(code, name) {
  for (const [, block] of code.matchAll(/^\s*case\s+"?\$name"?\s+in\s*$([\s\S]*?)^\s*esac\b/gm)) {
    if (!block.includes('recorded+=')) continue;
    for (const line of block.split('\n')) {
      const arm = line.match(/^\s*([A-Za-z0-9_*?| ]+)\)\s*(.*)$/);
      if (!arm) continue;
      if (!arm[1].split('|').some(p => globMatches(p.trim(), name))) continue;
      return RECORDS_NAME.test(arm[2]) || recordsExplicitly(arm[2], name);
    }
  }
  return false;
}

/** Problems with the producer script: it must record both declared variables as describe --env values and never declare a target to describe. */
export function producerProblems(script) {
  const code = uncommented(script);
  const problems = [];
  for (const name of [CFLAGS_VAR, CXXFLAGS_VAR]) {
    if (!recordsVariable(code, name)) problems.push(`eprfs-produce.sh does not record ${name} as a describe --env value`);
  }
  const describes = code
    .replace(/\\\n/g, ' ')
    .split('\n')
    .filter(line => /\bartifact describe\b/.test(line));
  if (describes.length === 0 || describes.some(line => !line.includes('"${recorded[@]}"'))) {
    problems.push('eprfs-produce.sh: an artifact describe does not pass "${recorded[@]}", so nothing recorded reaches it');
  }
  for (const flag of ['--target-isa', '--target-cflags']) {
    if (code.includes(flag)) problems.push(`eprfs-produce.sh passes ${flag} (put refuses a declared target until L2a, K6)`);
  }
  return problems;
}

/** Every version a Cargo.lock records for the named package. */
export function lockedVersions(lock, name) {
  const versions = [];
  for (const block of lock.split(/^\[\[package\]\]\s*$/m).slice(1)) {
    const n = block.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
    const v = block.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
    if (n === name && v) versions.push(v);
  }
  return versions;
}

/** Numeric semver comparison of the release part: 1.11.0 > 1.2.65. */
export function compareVersions(a, b) {
  const pa = a.split('-')[0].split('.').map(Number);
  const pb = b.split('-')[0].split('.').map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i += 1) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return Math.sign(d);
  }
  return 0;
}

/** Does cc's lib.rs push -march=x86-64 for x86_64 when rustc's target cpu is the baseline? */
export function pushesBaselineMarch(src) {
  return /"x86_64"\s+if\s+[^\n]*rustc_target_cpu\(\)[^\n]*=>\s*\{?\s*cmd\.push_\w+\(\s*&?\[?\s*"-march=x86-64"/.test(src);
}

const BUILD_CONFIG = /^(Dockerfile[^/]*|devfile[^/]*\.yaml)$/;
export const isBuildConfig = path => BUILD_CONFIG.test(basename(path)) || /(^|\/)\.cargo\/config\.toml$/.test(path);

/** Lines (outside full-line comments) that set a CPU level. */
export function cpuLevelLines(text) {
  return text
    .split('\n')
    .map((line, i) => ({ line, n: i + 1 }))
    .filter(({ line }) => !/^\s*#/.test(line) && /-march=|target-cpu/.test(line));
}

// ---- the checks proved against fixtures ----

test('fixture: the decided declared_targets shape passes and every drift is named', () => {
  const good = {
    cargo_env_overrides: { '*': { RUSTUP_TOOLCHAIN: '1.100.0' } },
    declared_targets: { _comment: 'row 29.2: the gate builds C at the Rust baseline.', [TRIPLE]: { ...EXPECTED_TARGET } },
  };
  assert.deepEqual(declaredTargetProblems(good), []);
  assert.match(declaredTargetProblems({ cargo_env_overrides: {} }).join('\n'), /no top-level declared_targets/);
  const nested = { cargo_env_overrides: { declared_targets: good.declared_targets } };
  assert.match(declaredTargetProblems(nested).join('\n'), /inside cargo_env_overrides/);
  const v3 = structuredClone(good);
  v3.declared_targets[TRIPLE] = { isa: 'x86-64-v3', cflags: '-march=x86-64-v3', workspaces: ['elohim/eprfs'] };
  assert.match(declaredTargetProblems(v3).join('\n'), /expected/);
  const extra = structuredClone(good);
  extra.declared_targets['aarch64-unknown-linux-gnu'] = { isa: 'armv8-a' };
  assert.match(declaredTargetProblems(extra).join('\n'), /keys are/);
  const extraField = structuredClone(good);
  extraField.declared_targets[TRIPLE].rustflags = '-C target-cpu=x86-64';
  assert.ok(declaredTargetProblems(extraField).length > 0);
  const noComment = structuredClone(good);
  noComment.declared_targets._comment = '';
  assert.match(declaredTargetProblems(noComment).join('\n'), /_comment/);
});

// The producer's recording code, in the shape eprfs-produce.sh has it: a case arm over the
// environment's names appends `--env "$name=${!name}"`, and the array reaches describe.
const producer = ({ arm = `    ${CFLAGS_VAR} | ${CXXFLAGS_VAR}) recorded+=(--env "$name=\${!name}") ;;`, before = '', describe = '"${recorded[@]}"' } = {}) =>
  [
    '# Of the C family only the declared target\'s flags are recorded; never pass --target-isa here.',
    'recorded=()',
    'while IFS= read -r name; do',
    '  case "$name" in',
    '    RUSTFLAGS) ;;',
    before,
    '    CARGO_* | RUST*) recorded+=(--env "$name=${!name}") ;;',
    arm,
    '  esac',
    'done < <(compgen -e | LC_ALL=C sort)',
    'for bin in "${bins[@]}"; do',
    '  said="$("$epr" artifact describe --project eprfs --bin "$bin" --rustflags "${RUSTFLAGS-}" \\',
    `    --sources "$sources" --out "$method" \\`,
    `    ${describe} 2>&1)" || skip "describe $bin refused"`,
    'done',
    '',
  ]
    .filter(line => line !== '')
    .join('\n');

test('fixture: the producer check wants both variables recorded as describe --env and no declared target', () => {
  assert.deepEqual(producerProblems(producer()), []);
  const explicit = producer({
    arm: `    ${CFLAGS_VAR}) recorded+=(--env "${CFLAGS_VAR}=\${${CFLAGS_VAR}}") ;;\n    ${CXXFLAGS_VAR}) recorded+=(--env "$name=\${!name}") ;;`,
  });
  assert.deepEqual(producerProblems(explicit), [], 'one arm per variable records them too');
  const only = (said, missing, kept) => {
    assert.match(said, new RegExp(`does not record ${missing} `));
    assert.doesNotMatch(said, new RegExp(`does not record ${kept} `));
  };
  // Each variable dropped from the recording arm on its own.
  only(producerProblems(producer({ arm: `    ${CFLAGS_VAR}) recorded+=(--env "$name=\${!name}") ;;` })).join('\n'), CXXFLAGS_VAR, CFLAGS_VAR);
  only(producerProblems(producer({ arm: `    ${CXXFLAGS_VAR}) recorded+=(--env "$name=\${!name}") ;;` })).join('\n'), CFLAGS_VAR, CXXFLAGS_VAR);
  // Named, but not recorded: in a comment, in other code, in an arm that drops them, or
  // shadowed by an earlier arm that matches them first (bash takes the first match).
  for (const [why, script] of [
    ['comment only', producer({ arm: `    # ${CFLAGS_VAR} | ${CXXFLAGS_VAR} are recorded` })],
    ['other code', producer({ arm: '' }) + `\necho "$${CFLAGS_VAR} $${CXXFLAGS_VAR}"\n`],
    ['dropping arm', producer({ arm: `    ${CFLAGS_VAR} | ${CXXFLAGS_VAR}) ;;` })],
    ['wrong value', producer({ arm: `    ${CFLAGS_VAR} | ${CXXFLAGS_VAR}) recorded+=(--env "$name=") ;;` })],
    ['shadowed', producer({ before: '    C*FLAGS_*) ;;' })],
  ]) {
    const said = producerProblems(script).join('\n');
    assert.match(said, new RegExp(`does not record ${CFLAGS_VAR} `), why);
    assert.match(said, new RegExp(`does not record ${CXXFLAGS_VAR} `), why);
  }
  // Recorded, but the array never reaches describe.
  assert.match(producerProblems(producer({ describe: '' })).join('\n'), /describe does not pass "\$\{recorded\[@\]\}"/);
  const declares = `${producer()}\n"$epr" artifact describe --target-isa x86-64 --target-cflags -march=x86-64\n`;
  const said = producerProblems(declares).join('\n');
  assert.match(said, /--target-isa/);
  assert.match(said, /--target-cflags/);
});

test('fixture: lock versions parse per package and compare numerically', () => {
  const lock = [
    'version = 4',
    '[[package]]\nname = "cc"\nversion = "1.2.65"\n',
    '[[package]]\nname = "ccc"\nversion = "9.9.9"\n',
    '[[package]]\nname = "cc"\nversion = "1.11.1"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n',
  ].join('\n');
  assert.deepEqual(lockedVersions(lock, 'cc'), ['1.2.65', '1.11.1']);
  assert.equal(compareVersions('1.11.0', '1.2.65'), 1, '1.11 > 1.2 numerically, not lexically');
  assert.equal(compareVersions('1.11.1', '1.11.1'), 0);
  assert.equal(compareVersions('1.11.0', '1.11.1'), -1);
  assert.equal(compareVersions('1.10.0', '1.9.0'), 1);
});

test('fixture: the cc source check recognises the baseline push and its absence', () => {
  const v1_11 = [
    '                        // Match rustc\'s CPU. A distribution can build GCC with a',
    '                        // higher default, e.g. `x86-64-v2` on RHEL 9.',
    '                        "x86_64" if matches!(self.rustc_target_cpu(), None | Some("x86-64")) => {',
    '                            cmd.push_cc_str("-march=x86-64")',
    '                        }',
  ].join('\n');
  assert.equal(pushesBaselineMarch(v1_11), true);
  assert.equal(pushesBaselineMarch('"i686" => cmd.push_arg_str("-march=i686"),\n_ => {}'), false);
  assert.equal(pushesBaselineMarch('"x86_64" => cmd.push_cc_str("-march=x86-64-v3")'), false);
});

test('fixture: build-config selection and CPU-level lines', () => {
  for (const p of ['Dockerfile', 'a/b/Dockerfile.mesh', 'devfile.yaml', 'devfile_x86_v2.yaml', '.cargo/config.toml', 'x/.cargo/config.toml']) {
    assert.equal(isBuildConfig(p), true, p);
  }
  for (const p of ['devfile.test.mjs', 'docs/Dockerfiles.md/x', 'cargo/config.toml', '.cargo/config']) {
    assert.equal(isBuildConfig(p), false, p);
  }
  assert.deepEqual(cpuLevelLines('# -march=native in a comment\nRUN true\n'), []);
  assert.deepEqual(
    cpuLevelLines('[build]\nrustflags = ["-C", "target-cpu=native"]\nENV CFLAGS=-march=x86-64-v3\n').map(l => l.n),
    [2, 3],
  );
});

// ---- the repository ----

test('pool-policy declares the C baseline as a top-level declared_targets record (row 29.2)', () => {
  const policy = JSON.parse(read('genesis/agentic/pool-policy.json'));
  assert.deepEqual(declaredTargetProblems(policy), [], 'genesis/agentic/pool-policy.json declared_targets');
});

test('the eprfs producer records both declared C flags variables as describe --env and declares no target', () => {
  assert.deepEqual(producerProblems(read('genesis/orchestrator/eprfs-produce.sh')), []);
});

const LOCK_FLOORS = [
  ['elohim/elohim-storage/Cargo.lock', '1.11.0'],
  ['doorway/doorway-service/Cargo.lock', '1.11.0'],
  ['steward/node/Cargo.lock', '1.11.0'],
  ['elohim/eprfs/Cargo.lock', '1.11.1'],
];

for (const [lock, floor] of LOCK_FLOORS) {
  test(`${lock} locks cc >= ${floor}`, () => {
    const versions = lockedVersions(read(lock), 'cc');
    assert.ok(versions.length > 0, `${lock} locks no cc`);
    const below = versions.filter(v => compareVersions(v, floor) < 0);
    assert.deepEqual(below, [], `${lock} locks cc ${below.join(', ')} below ${floor}, which passes no -march`);
  });
}

const cargoHomes = [process.env.CARGO_HOME, '/opt/rust/cargo', join(homedir(), '.cargo')].filter(Boolean);

function ccSource(version) {
  for (const home of cargoHomes) {
    const src = join(home, 'registry', 'src');
    if (!existsSync(src)) continue;
    for (const index of readdirSync(src)) {
      const lib = join(src, index, `cc-${version}`, 'src', 'lib.rs');
      if (existsSync(lib)) return { lib, text: readFileSync(lib, 'utf8') };
    }
  }
  return null;
}

for (const [lock] of LOCK_FLOORS) {
  const versions = existsSync(resolve(ROOT, lock)) ? lockedVersions(read(lock), 'cc') : [];
  for (const version of versions) {
    test(`cc ${version} (${lock}) pushes -march=x86-64 matched to rustc's target cpu`, t => {
      const found = ccSource(version);
      if (!found) {
        t.skip(`cc-${version} source is not in ${cargoHomes.join(' or ')}/registry/src`);
        return;
      }
      assert.ok(pushesBaselineMarch(found.text), `${found.lib} no longer pushes -march=x86-64 for x86_64 at rustc's baseline cpu`);
    });
  }
}

test('no tracked Dockerfile, devfile or .cargo/config.toml sets -march= or target-cpu', () => {
  const pathspecs = [':(glob)**/Dockerfile*', ':(glob)**/devfile*.yaml', ':(glob)**/.cargo/config.toml'];
  const tracked = execFileSync('git', ['ls-files', '-z', '--', ...pathspecs], {
    cwd: ROOT,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  })
    .split('\0')
    .filter(Boolean);
  const scanned = tracked.filter(isBuildConfig);
  assert.ok(scanned.length > 0, 'git ls-files found no build configs to scan');
  const hits = [];
  for (const path of scanned) {
    for (const { line, n } of cpuLevelLines(read(path))) hits.push(`${path}:${n}: ${line.trim()}`);
  }
  assert.deepEqual(
    hits,
    [],
    `a tracked build config sets a CPU level outside declared_targets.\nhits:\n${hits.join('\n')}\nscanned (${scanned.length}):\n${scanned.join('\n')}`,
  );
});
