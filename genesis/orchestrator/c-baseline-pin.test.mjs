// Backlog steward-device-arrival-open-items row 29.2: the gate builds C at ONE declared
// level, the Rust target's baseline (-march=x86-64 on x86_64-unknown-linux-gnu). RHEL 10
// gcc defaults to -march=x86-64-v3; cc 1.2.65 passed no -march, so ring's curve25519.c
// carried AVX2 into a drawn epr and a v2 sibling died with SIGILL. cc >= 1.11.0 pushes
// -march=x86-64 itself ("Match rustc's CPU"). This test pins every home of that decision:
// the pool-policy record, the producer recording the record's two variables and declaring the
// record's target to describe, the cc floor in each native lock, the cc source still carrying the
// push, no tracked build config raising the level behind the record's back, and every cc-built
// crate in epr's lock closure accounted for. A drift in any one of them fails here.
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
// `env`: the gate exports these beside the two flags (E12 L2a rework, ruling 1): cc's command log,
// the evidence `epr artifact put` reads before it stamps env:isa-c.
const EXPECTED_TARGET = {
  isa: 'x86-64', cflags: '-march=x86-64', env: { CC_ENABLE_DEBUG_OUTPUT: '1' }, workspaces: ['elohim/eprfs'],
};

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

const TARGET_FLAGS = ['--target-isa', '--target-cflags'];
const escape = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
/** `<flag> "$var"` or `<flag> "${var}"`: the value is an expansion, never a literal word. */
const passesExpansion = (text, flag) =>
  new RegExp(`${escape(flag)}\\s+"\\$(?:[A-Za-z_][A-Za-z0-9_]*|\\{[A-Za-z_][A-Za-z0-9_]*(?:\\[[0-9]+\\])?\\})"`).test(text);

/**
 * Problems with the producer script. It must record both declared variables as describe --env
 * values, and declare the record's target to every describe: `--target-isa "$v" --target-cflags
 * "$w"` on the describe line, or in an array `NAME=(...)` the describe line passes as
 * "${NAME[@]}", with no literal of the record's isa or cflags anywhere in the code and the values
 * read from pool-policy.json declared_targets. (That the values passed ARE the record's is proved
 * by eprfs-produce.test.mjs against a fixture record whose values no script could carry.)
 */
export function producerProblems(script, target = EXPECTED_TARGET) {
  const code = uncommented(script);
  const problems = [];
  for (const name of [CFLAGS_VAR, CXXFLAGS_VAR]) {
    if (!recordsVariable(code, name)) problems.push(`eprfs-produce.sh does not record ${name} as a describe --env value`);
  }
  const joined = code.replace(/\\\n/g, ' ');
  const describes = joined.split('\n').filter(line => /\bartifact describe\b/.test(line));
  if (describes.length === 0 || describes.some(line => !line.includes('"${recorded[@]}"'))) {
    problems.push('eprfs-produce.sh: an artifact describe does not pass "${recorded[@]}", so nothing recorded reaches it');
  }
  const arrays = [...joined.matchAll(/(?:^|[\s;&|])([A-Za-z_][A-Za-z0-9_]*)=\(([^()]*)\)/gm)]
    .filter(([, , body]) => TARGET_FLAGS.every(flag => passesExpansion(body, flag)))
    .map(([, name]) => name);
  const declares = line =>
    TARGET_FLAGS.every(flag => passesExpansion(line, flag)) || arrays.some(name => line.includes(`"\${${name}[@]}"`));
  if (describes.length === 0 || describes.some(line => !declares(line))) {
    problems.push(`eprfs-produce.sh: an artifact describe is not given ${TARGET_FLAGS.join(' and ')} as expansions of the record's values`);
  }
  for (const value of [target.cflags, target.isa]) {
    if (new RegExp(`(^|[^\\w.-])${escape(value)}([^\\w.-]|$)`).test(code)) {
      problems.push(`eprfs-produce.sh carries the literal ${value}; the declared target is read from the record, never written in the script`);
    }
  }
  if (!/pool-policy\.json/.test(code) || !/\bdeclared_targets\b/.test(code)) {
    problems.push('eprfs-produce.sh does not read pool-policy.json declared_targets, so its declared target comes from somewhere else');
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
  v3.declared_targets[TRIPLE] = { ...EXPECTED_TARGET, isa: 'x86-64-v3', cflags: '-march=x86-64-v3' };
  assert.match(declaredTargetProblems(v3).join('\n'), /expected/);
  const noLog = structuredClone(good);
  delete noLog.declared_targets[TRIPLE].env;
  assert.match(declaredTargetProblems(noLog).join('\n'), /expected/, 'the record declares the command log put reads');
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
// environment's names appends `--env "$name=${!name}"`, and the array reaches describe; the
// declared target is read from the record into an array that reaches describe too.
const DECLARES = 'if [[ -n "$isa" ]]; then declared=(--target-isa "$isa" --target-cflags "$cflags"); fi';
const producer = ({
  arm = `    ${CFLAGS_VAR} | ${CXXFLAGS_VAR}) recorded+=(--env "$name=\${!name}") ;;`,
  before = '',
  describe = '"${declared[@]}" "${recorded[@]}"',
  declares = DECLARES,
  reads = 'mapfile -t target < <(python3 "$read_target" "$repo/genesis/agentic/pool-policy.json" declared_targets)',
} = {}) =>
  [
    '# Of the C family only the declared target\'s flags are recorded; its -march=x86-64 is the record\'s.',
    'declared=()',
    reads,
    'isa="${target[0]-}" cflags="${target[1]-}"',
    declares,
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

test('fixture: the producer check wants both variables recorded as describe --env and the record\'s target declared', () => {
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
  assert.match(producerProblems(producer({ describe: '"${declared[@]}"' })).join('\n'), /describe does not pass "\$\{recorded\[@\]\}"/);
  // The declared target: on the describe line itself is as good as through an array.
  const inline = producer({ declares: '', describe: '--target-isa "${target[0]}" --target-cflags "$cflags" "${recorded[@]}"' });
  assert.deepEqual(producerProblems(inline), [], 'flags on the describe line, values expanded');
  const undeclared = /describe is not given --target-isa and --target-cflags as expansions/;
  for (const [why, script] of [
    ['the array never reaches describe', producer({ describe: '"${recorded[@]}"' })],
    ['no declared array', producer({ declares: '' })],
    ['one flag only', producer({ declares: 'declared=(--target-isa "$isa")' })],
    ['a literal value', producer({ declares: 'declared=(--target-isa "$isa" --target-cflags -march="$isa")' })],
  ]) {
    assert.match(producerProblems(script).join('\n'), undeclared, why);
  }
  // Literal values: the record's isa or cflags written into the script, in any form, are named,
  // whether or not the flags also look like expansions; a level above it is not this record's.
  for (const literal of [
    'declared=(--target-isa x86-64 --target-cflags -march=x86-64)',
    'declared=(--target-isa "$isa" --target-cflags "$cflags"); cflags=-march=x86-64',
    '"$epr" artifact describe --target-isa x86-64 --target-cflags=-march=x86-64',
  ]) {
    assert.match(producerProblems(`${producer()}\n${literal}\n`).join('\n'), /carries the literal (-march=)?x86-64;/, literal);
  }
  assert.deepEqual(producerProblems(`${producer()}\n# -march=x86-64 in a comment\nv=x86-64-v3\n`), [], 'a comment or another level is not the literal');
  const elsewhere = producer({ reads: 'mapfile -t target < <(cat "$here/target.txt")' });
  assert.match(producerProblems(elsewhere).join('\n'), /does not read pool-policy\.json declared_targets/);
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
  // The guard alone is not enough: with rustc's baseline guard in place, a push of a higher
  // level is still not the baseline push (this case tests the closing quote, not the guard).
  const v3 = v1_11.replace('"-march=x86-64"', '"-march=x86-64-v3"');
  assert.match(v3, /rustc_target_cpu\(\)/);
  assert.equal(pushesBaselineMarch(v3), false);
  assert.equal(pushesBaselineMarch('"x86_64" => cmd.push_cc_str("-march=x86-64")'), false, 'no guard');
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

// Hardening 6: put reads the build's own compile commands (isa_c_evidence.rs); the retired
// string scan of the bytes' debug section must not be described as what decides the marker.
export const RETIRED_EVIDENCE = /producer strings? in the bytes|GNU C producer string|\.debug_str/i;

test('fixture: a comment describing the retired producer-string check is named', () => {
  assert.match('# put checks the C producer strings in the bytes', RETIRED_EVIDENCE);
  assert.doesNotMatch('# put reads the compile commands of the build that produced the bytes', RETIRED_EVIDENCE);
});

test('neither the producer nor this pin test describes the retired producer-string check', () => {
  const pin = read('genesis/orchestrator/c-baseline-pin.test.mjs').split('\n')
    .filter(line => !line.includes('RETIRED_EVIDENCE') && !line.includes('producer strings in the bytes\''));
  for (const [file, lines] of [
    ['genesis/orchestrator/eprfs-produce.sh', read('genesis/orchestrator/eprfs-produce.sh').split('\n')],
    ['genesis/orchestrator/c-baseline-pin.test.mjs', pin],
  ]) {
    const stale = lines.map((line, i) => [i + 1, line]).filter(([, line]) => RETIRED_EVIDENCE.test(line));
    assert.deepEqual(stale.map(([n, line]) => `${file}:${n}: ${line.trim()}`), []);
  }
});

test('the eprfs producer records both declared C flags variables as describe --env and declares the record\'s target', () => {
  const policy = JSON.parse(read('genesis/agentic/pool-policy.json'));
  assert.deepEqual(producerProblems(read('genesis/orchestrator/eprfs-produce.sh'), policy.declared_targets[TRIPLE]), []);
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

// ── cc pushes -march=x86-64 only when rustc's target cpu is unset or x86-64 (cc src/lib.rs ~3350;
// brit sprint 3 measured no -march at all under -C target-cpu=x86-64-v2). So a declared workspace's
// builds hold their C baseline only while nothing sets a target cpu for them: the gate's rustflags,
// the policy's cargo env, and the bin store's own builds. Cargo configs are the test above. ─────────
const CPU_FLAG = /target-cpu|target-feature|-C\s*target|-Ctarget/;
const RUSTFLAG_KEYS = /^(RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|CARGO_BUILD_RUSTFLAGS|CARGO_TARGET_[A-Z0-9_]+_RUSTFLAGS)$/;
const BIN_STORE_EMPTY_RUSTFLAGS = /insert\("RUSTFLAGS"\.into\(\),\s*Some\(String::new\(\)\)\)/g;

export function targetCpuProblems({ projects, overrides, workspaces, binStoreSource }) {
  const problems = [];
  const declared = projects.filter(p => workspaces.includes(p.run?.cargo?.workspace));
  if (!declared.length) problems.push('no gate project builds a declared workspace');
  for (const p of declared) {
    const rustflags = p.run.cargo.rustflags ?? '';
    if (CPU_FLAG.test(rustflags)) problems.push(`${p.name}: run.cargo.rustflags sets a target cpu (${rustflags})`);
    for (const scope of ['*', p.name]) {
      for (const [key, value] of Object.entries(overrides?.[scope] ?? {})) {
        if (RUSTFLAG_KEYS.test(key) && CPU_FLAG.test(String(value))) problems.push(`${p.name}: cargo_env_overrides["${scope}"].${key} sets a target cpu`);
      }
      for (const [key, value] of Object.entries(p.run.cargo.env ?? {})) {
        if (scope === p.name && RUSTFLAG_KEYS.test(key) && CPU_FLAG.test(String(value))) problems.push(`${p.name}: run.cargo.env.${key} sets a target cpu`);
      }
    }
  }
  const empties = (binStoreSource.match(BIN_STORE_EMPTY_RUSTFLAGS) ?? []).length;
  if (empties < 2) problems.push(`bin_store/source.rs clears RUSTFLAGS for ${empties} of its 2 builds (the bin store builds epr outside the gate)`);
  return problems;
}

test('fixture: a target cpu anywhere an eprfs-workspace build reads it is named', () => {
  const ok = { name: 'eprfs', run: { cargo: { workspace: 'elohim/eprfs', rustflags: '' } } };
  const source = 'insert("RUSTFLAGS".into(), Some(String::new()));\ninsert("RUSTFLAGS".into(), Some(String::new()));';
  const base = { projects: [ok], overrides: {}, workspaces: ['elohim/eprfs'], binStoreSource: source };
  assert.deepEqual(targetCpuProblems(base), []);
  assert.match(targetCpuProblems({ ...base, projects: [{ ...ok, run: { cargo: { ...ok.run.cargo, rustflags: '-C target-cpu=x86-64-v2' } } }] })[0], /rustflags sets a target cpu/);
  assert.match(targetCpuProblems({ ...base, overrides: { '*': { RUSTFLAGS: '-Ctarget-cpu=native' } } })[0], /cargo_env_overrides\["\*"\]\.RUSTFLAGS/);
  assert.match(targetCpuProblems({ ...base, overrides: { eprfs: { CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS: '-C target-feature=+avx2' } } })[0], /CARGO_TARGET_/);
  assert.match(targetCpuProblems({ ...base, projects: [{ ...ok, run: { cargo: { ...ok.run.cargo, env: { RUSTFLAGS: '-C target-cpu=haswell' } } } }] })[0], /run\.cargo\.env\.RUSTFLAGS/);
  assert.match(targetCpuProblems({ ...base, binStoreSource: source.split('\n')[0] })[0], /1 of its 2 builds/);
  assert.deepEqual(targetCpuProblems({ ...base, projects: [{ ...ok, run: { cargo: { workspace: 'elohim', rustflags: '-C target-cpu=native' } } }] }),
    ['no gate project builds a declared workspace'], 'another workspace is not this record\'s business');
});

test('no build of a declared workspace sets a target cpu, so cc keeps pushing the baseline', async () => {
  const { loadGateRegistry } = await import('./pipeline-registry.mjs');
  const policy = JSON.parse(read('genesis/agentic/pool-policy.json'));
  const workspaces = Object.entries(policy.declared_targets).filter(([k]) => !k.startsWith('_')).flatMap(([, t]) => t.workspaces);
  const projects = [...loadGateRegistry(ROOT).values()];
  assert.deepEqual(targetCpuProblems({ projects, overrides: policy.cargo_env_overrides, workspaces,
    binStoreSource: read('elohim/eprfs/device-runtime/src/bin_store/source.rs') }), []);
});

// ── Row 29.2 memo, PIN TEST, as reworked at E12 L2a (ruling 1): every crate in epr's and
// bin-launch's lock closures that builds with cc is accounted for. `epr artifact put` reads the
// compile commands of the build that produced the bytes (cc's `running:` log, enabled by the
// record's CC_ENABLE_DEBUG_OUTPUT) and stamps env:isa-c only when each one carries the declared
// -march and no other ISA selection, or the crate is a named exception in
// elohim/eprfs/epr-cli/src/artifact/isa_c_evidence.rs EXCEPTIONS. So each cc-built crate here is
// JUDGED (put judges its commands) or EXCEPTED (on that Rust list, with its reason held below
// against the crate's own build.rs), and the two lists agree. An entry whose crate left the
// closures fails too. The lock is read as text: its dependency edges are the union over features
// and platforms, so the closure is a superset of any one build's, the safe side for a list of what
// must be accounted for. ────────────────────────────────────────────────────────────────────────
const JUDGED = 'judged by put from its compile commands';
const EXCEPTED = 'a named exception in isa_c_evidence.rs';
// Seeded from elohim/eprfs/Cargo.lock at 71069b1f5, each reason read from the crate's build.rs and
// from its compile commands in a gate build (CC_ENABLE_DEBUG_OUTPUT=1, 2026-11-19).
export const CC_BUILT_IN_EPR = {
  ring: {
    kind: JUDGED,
    exercises: 'x25519',
    reason: 'curve25519.c, its other C units and its assembly each compile at the declared -march, which put reads ' +
      'from each command; the exercise also runs an x25519 agreement through it',
  },
  'libsqlite3-sys': {
    kind: JUDGED,
    exercises: 'sqlite',
    reason: 'rusqlite "bundled" compiles sqlite3.c in one command carrying the declared -march, which put reads; the ' +
      'exercise also runs a SQLite round trip',
  },
  psm: {
    kind: JUDGED,
    reason: 'on x86_64 Linux its build.rs compiles src/arch/x86_64.s (-xassembler-with-cpp) in one command carrying ' +
      'the declared -march, which put reads; it is not a crate that compiles nothing',
  },
  blake3: { kind: EXCEPTED },
  stacker: { kind: EXCEPTED },
  generator: { kind: EXCEPTED },
  'iana-time-zone-haiku': { kind: EXCEPTED },
};

const EVIDENCE_RS = 'elohim/eprfs/epr-cli/src/artifact/isa_c_evidence.rs';

/**
 * The EXCEPTIONS list in isa_c_evidence.rs: `{ package, kind, reason }` per entry, the reason's
 * Rust string continuations (`\` and the next line's indentation) joined as rustc joins them.
 */
export function rustExceptions(source) {
  const list = source.match(/pub\(super\) const EXCEPTIONS: &\[Exception\] = &\[([\s\S]*?)\n\];/)?.[1];
  if (!list) throw new Error(`${EVIDENCE_RS} declares no EXCEPTIONS list`);
  const entries = [];
  for (const [, body] of list.matchAll(/Exception \{([\s\S]*?)\n {4}\},/g)) {
    const package_ = body.match(/package: "([^"]+)"/)?.[1];
    const version = body.match(/version: "([^"]+)"/)?.[1];
    const kind = body.match(/kind: ExceptionKind::(\w+)/)?.[1];
    const sources = [...(body.match(/sources: &\[([\s\S]*?)\]/)?.[1] ?? '').matchAll(/"([^"]+)"/g)].map(m => m[1]);
    const raw = body.match(/reason:\s*"((?:[^"\\]|\\[\s\S])*)"/)?.[1];
    const reason = raw?.replace(/\\\n\s*/g, '').replace(/\\"/g, '"');
    entries.push({ package: package_, version, kind, sources, reason });
  }
  return entries;
}

function crateSource(name, version) {
  for (const home of cargoHomes) {
    const src = join(home, 'registry', 'src');
    if (!existsSync(src)) continue;
    for (const index of readdirSync(src)) {
      const dir = join(src, index, `${name}-${version}`);
      if (existsSync(dir)) return dir;
    }
  }
  return null;
}

/** The body of `fn <name>(` in a Rust source, up to the next top-level `fn`. */
const fnBody = (source, name) => source.match(new RegExp(`\\nfn ${name}\\([\\s\\S]*?(?=\\nfn |$)`))?.[0] ?? '';

/**
 * Each exception's reason held against what its crate's source does, per crate: problems, empty
 * when the source still says what the reason says. `read(relative)` reads the crate's file;
 * `lockedCrate(name)` the source dir of another locked crate (iana-time-zone).
 */
export const EXCEPTION_HOLDS = {
  blake3: {
    kind: 'DispatchedAssembly',
    holds: ({ read, exception }) => {
      const build = read('build.rs');
      const problems = [];
      const main = fnBody(build, 'main');
      // Re-review ruling 3: the exception names exactly the unix assembly build.rs compiles.
      const unixAssembly = ['build_sse2_sse41_avx2_assembly', 'build_avx512_assembly']
        .flatMap(name => [...fnBody(build, name).matchAll(/build\.file\("([^"]+)"\)/g)].map(m => m[1]))
        .filter(f => !/windows/.test(f))
        .sort();
      const named = [...(exception?.sources ?? [])].sort();
      if (JSON.stringify(named) !== JSON.stringify(unixAssembly)) {
        problems.push(`the exception reviews ${named.join(', ') || 'no sources'}, and build.rs compiles ${unixAssembly.join(', ')} on x86_64 unix`);
      }
      if (!/should_prefer_intrinsics\(\) \|\| is_pure\(\)[\s\S]*?build_sse2_sse41_avx2_rust_intrinsics\(\);\s*\} else \{[\s\S]*?build_sse2_sse41_avx2_assembly\(\);/.test(main)) {
        problems.push('build.rs main no longer builds the SSE2/SSE4.1/AVX2 assembly unless pure or prefer_intrinsics');
      }
      if (!/else if is_x86_32\(\) \|\| should_prefer_intrinsics\(\) \{\s*build_avx512_c_intrinsics\(\);\s*\} else \{\s*build_avx512_assembly\(\);/.test(main)) {
        problems.push('build.rs main no longer builds the AVX-512 assembly, not C, unless prefer_intrinsics');
      }
      for (const name of ['build_sse2_sse41_avx2_assembly', 'build_avx512_assembly']) {
        const files = [...fnBody(build, name).matchAll(/build\.file\("([^"]+)"\)/g)].map(m => m[1]);
        const unix = files.filter(f => !/windows/.test(f));
        if (!unix.length || unix.some(f => !f.endsWith('_unix.S'))) problems.push(`${name} compiles ${unix.join(', ') || 'nothing'} on unix, not only _unix.S assembly`);
      }
      const cFiles = [...build.matchAll(/build\.file\("([^"]+\.c)"\)/g)].map(m => m[1]);
      const cHomes = cFiles.filter(f => !['build_avx512_c_intrinsics', 'build_neon_c_intrinsics'].some(fn => fnBody(build, fn).includes(f)));
      if (cHomes.length) problems.push(`build.rs compiles C outside the intrinsics paths: ${cHomes.join(', ')}`);
      const platform = read('src/platform.rs');
      if (!/fn avx512_detected\(\)/.test(platform) || !/cpufeatures::new!\(has_avx512, "avx512f", "avx512vl"\)/.test(platform)) {
        problems.push('src/platform.rs no longer chooses the AVX-512 path by a run-time CPUID check');
      }
      return problems;
    },
  },
  stacker: {
    kind: 'CompilesNothing',
    holds: ({ read }) => {
      const build = read('build.rs');
      const guarded = build.match(/if target\.contains\("windows"\) \{([\s\S]*?)\n {4}\}/)?.[1] ?? '';
      const files = [...build.matchAll(/\.file\(/g)].length;
      return files > 0 && [...guarded.matchAll(/\.file\(/g)].length === files && /\.compile\(/.test(guarded)
        ? [] : ['build.rs runs cc outside its Windows branch'];
    },
  },
  generator: {
    kind: 'CompilesNothing',
    holds: ({ read }) => {
      const build = read('build.rs');
      const problems = [];
      if (!/let external_assembly_required = target_arch == "powerpc64";/.test(build)) problems.push('build.rs no longer limits its assembly to powerpc64');
      const guarded = build.match(/if external_assembly_required \{([\s\S]*?)\n {4}\}/)?.[1] ?? '';
      if ([...build.matchAll(/\.file\(/g)].length !== [...guarded.matchAll(/\.file\(/g)].length) problems.push('build.rs runs cc outside its powerpc64 branch');
      return problems;
    },
  },
  'iana-time-zone-haiku': {
    kind: 'CompilesNothing',
    holds: ({ lockedCrate }) => {
      const dir = lockedCrate('iana-time-zone');
      const manifest = readFileSync(join(dir, 'Cargo.toml'), 'utf8');
      const headers = manifest.split('\n').filter(line => /^\[.*iana-time-zone-haiku/.test(line));
      return headers.length > 0 && headers.every(h => h === '[target.\'cfg(target_os = "haiku")\'.dependencies.iana-time-zone-haiku]')
        ? [] : [`iana-time-zone depends on iana-time-zone-haiku outside cfg(target_os = "haiku"): ${headers.join(' ')}`];
    },
  },
};

/** The lock's packages, each with its dependency edges as the lock writes them. */
export function lockPackages(lock) {
  return lock.split(/^\[\[package\]\]\s*$/m).slice(1).map(block => ({
    name: block.match(/^name\s*=\s*"([^"]+)"/m)?.[1],
    version: block.match(/^version\s*=\s*"([^"]+)"/m)?.[1],
    deps: (block.match(/^dependencies\s*=\s*\[([\s\S]*?)\]/m)?.[1]?.match(/"[^"]+"/g) ?? []).map(d => d.slice(1, -1)),
  }));
}

/** Names (sorted, one per crate) of the packages reachable from `root` that depend on cc. */
export function ccBuiltCrates(packages, root) {
  const byName = new Map();
  for (const p of packages) byName.set(p.name, [...(byName.get(p.name) ?? []), p]);
  // An edge is `name`, or `name version` when the lock holds more than one version, and may carry
  // ` (source)` after that.
  const targets = edge => {
    const [name, version] = edge.split(' ');
    return (byName.get(name) ?? []).filter(p => !version || p.version === version);
  };
  const seen = new Set();
  const stack = [...(byName.get(root) ?? [])];
  if (stack.length === 0) throw new Error(`the lock has no package ${root}`);
  const built = new Set();
  while (stack.length) {
    const p = stack.pop();
    const key = `${p.name} ${p.version}`;
    if (seen.has(key)) continue;
    seen.add(key);
    if (p.deps.some(edge => edge.split(' ')[0] === 'cc')) built.add(p.name);
    for (const edge of p.deps) stack.push(...targets(edge));
  }
  return [...built].sort();
}

/**
 * Problems with the list against the closures' cc-built crates, the Rust EXCEPTIONS list
 * (`exceptions`, from rustExceptions) and exercise.rs's source. A JUDGED crate gives its reason and
 * is not excepted; an EXCEPTED one is on the Rust list, whose reason is one sentence.
 */
export function ccAccountProblems(built, list, exerciseSource, exceptions) {
  const problems = [];
  const excepted = new Map(exceptions.map(e => [e.package, e]));
  for (const name of built) {
    if (!Object.hasOwn(list, name)) {
      problems.push(`${name} builds with cc in a declared bin's lock closure and is not on CC_BUILT_IN_EPR: say whether it is ${JUDGED}, or ${EXCEPTED} (and add it there with its reason)`);
    }
  }
  for (const [name, entry] of Object.entries(list)) {
    if (!built.includes(name)) problems.push(`CC_BUILT_IN_EPR lists ${name}, which no longer builds with cc in a declared bin's lock closure`);
    if (![JUDGED, EXCEPTED].includes(entry?.kind)) problems.push(`CC_BUILT_IN_EPR.${name}.kind is ${JSON.stringify(entry?.kind)}, not "${JUDGED}" or "${EXCEPTED}"`);
    if (entry?.kind === JUDGED) {
      if (typeof entry.reason !== 'string' || entry.reason.trim().length < 20) problems.push(`CC_BUILT_IN_EPR.${name} gives no reason`);
      if (excepted.has(name)) problems.push(`${name} is judged here and excepted in ${EVIDENCE_RS}; it is one or the other`);
    }
    if (entry?.kind === EXCEPTED && !excepted.has(name)) problems.push(`CC_BUILT_IN_EPR.${name} is excepted here and not on ${EVIDENCE_RS} EXCEPTIONS`);
    if (entry?.exercises !== undefined && !(typeof entry.exercises === 'string' && entry.exercises && exerciseSource.includes(entry.exercises))) {
      problems.push(`CC_BUILT_IN_EPR.${name} is exercised through ${JSON.stringify(entry.exercises)}, which epr-cli/src/exercise.rs does not run`);
    }
  }
  for (const exception of exceptions) {
    if (list[exception.package]?.kind !== EXCEPTED) problems.push(`${EVIDENCE_RS} excepts ${exception.package}, which CC_BUILT_IN_EPR does not list as ${EXCEPTED}`);
    const reason = exception.reason ?? '';
    if (reason.length < 40 || !reason.endsWith('.') || reason.includes('. ')) problems.push(`the exception for ${exception.package} gives no one-sentence reason: ${JSON.stringify(reason)}`);
    if (EXCEPTION_HOLDS[exception.package]?.kind !== exception.kind) {
      problems.push(`the exception for ${exception.package} (${exception.kind}) has no check here holding it against its build.rs`);
    }
  }
  return problems;
}

test('fixture: a cc-built crate reached from the root is found; one reached from nowhere is not', () => {
  const lock = [
    'version = 4',
    '[[package]]\nname = "cc"\nversion = "1.2.65"\n',
    '[[package]]\nname = "cc"\nversion = "1.11.1"\n',
    '[[package]]\nname = "root"\nversion = "0.1.0"\ndependencies = [\n "mid",\n "ring",\n]\n',
    '[[package]]\nname = "mid"\nversion = "1.0.0"\ndependencies = [\n "zlib-sys 2.0.0",\n]\n',
    '[[package]]\nname = "zlib-sys"\nversion = "2.0.0"\ndependencies = [\n "cc 1.11.1",\n]\n',
    '[[package]]\nname = "zlib-sys"\nversion = "1.0.0"\ndependencies = [\n "cc 1.2.65",\n]\n',
    '[[package]]\nname = "ring"\nversion = "0.17.14"\ndependencies = [\n "cc 1.11.1 (registry+https://github.com/rust-lang/crates.io-index)",\n]\n',
    '[[package]]\nname = "ccache-wrapper"\nversion = "1.0.0"\ndependencies = [\n "ccc",\n]\n',
    '[[package]]\nname = "orphan-sys"\nversion = "1.0.0"\ndependencies = [\n "cc 1.11.1",\n]\n',
  ].join('\n');
  assert.deepEqual(ccBuiltCrates(lockPackages(lock), 'root'), ['ring', 'zlib-sys']);
  assert.throws(() => ccBuiltCrates(lockPackages(lock), 'absent'), /no package absent/);
  const exercise = 'exercise: ok (x25519 secret, sqlite rows)';
  const list = {
    ring: { kind: JUDGED, exercises: 'x25519', reason: 'curve25519.c compiles at the declared -march' },
    stacker: { kind: EXCEPTED },
  };
  const exceptions = [{ package: 'stacker', kind: 'CompilesNothing', reason: 'stacker runs cc only on Windows, so it compiles nothing on Linux.' }];
  const said = (...args) => ccAccountProblems(...args).join('\n');
  assert.deepEqual(ccAccountProblems(['ring', 'stacker'], list, exercise, exceptions), []);
  const { stacker: _, ...short } = list;
  assert.match(said(['ring', 'stacker'], short, exercise, exceptions), /stacker builds with cc .* not on CC_BUILT_IN_EPR/);
  assert.match(said(['ring'], list, exercise, exceptions), /lists stacker, which no longer builds/);
  assert.match(said(['ring', 'stacker'], { ...list, ring: { ...list.ring, exercises: 'ed448' } }, exercise, exceptions), /exercised through "ed448"/);
  assert.match(said(['ring', 'stacker'], { ...list, ring: { ...list.ring, kind: 'fine' } }, exercise, exceptions), /kind is "fine"/);
  assert.match(said(['ring', 'stacker'], { ...list, ring: { kind: JUDGED, reason: '' } }, exercise, exceptions), /ring gives no reason/);
  assert.match(said(['ring', 'stacker'], list, exercise, []), /stacker is excepted here and not on/);
  assert.match(said(['ring', 'stacker'], list, exercise, [...exceptions, { package: 'ring', kind: 'CompilesNothing', reason: exceptions[0].reason }]),
    /ring is judged here and excepted/);
  assert.match(said(['ring', 'stacker'], list, exercise, [{ ...exceptions[0], reason: 'too short.' }]), /no one-sentence reason/);
  assert.match(said(['ring', 'stacker'], list, exercise, [{ ...exceptions[0], reason: `${exceptions[0].reason} Then more.` }]), /no one-sentence reason/);
  assert.match(said(['ring', 'stacker'], list, exercise, [{ ...exceptions[0], kind: 'DispatchedAssembly' }]), /no check here holding it/);
});

test('fixture: the Rust EXCEPTIONS list is read with its continued reason strings joined', () => {
  const source = [
    'pub(super) const EXCEPTIONS: &[Exception] = &[',
    '    Exception {',
    '        package: "blake3",',
    '        version: "1.8.5",',
    '        kind: ExceptionKind::DispatchedAssembly,',
    '        sources: &[',
    '            "c/a_unix.S",',
    '            "c/b_unix.S",',
    '        ],',
    '        reason:',
    '            "On x86_64 its build.rs \\',
    '                 compiles only \\"assembly\\".",',
    '    },',
    '];',
  ].join('\n');
  assert.deepEqual(rustExceptions(source), [{
    package: 'blake3', version: '1.8.5', kind: 'DispatchedAssembly', sources: ['c/a_unix.S', 'c/b_unix.S'],
    reason: 'On x86_64 its build.rs compiles only "assembly".',
  }]);
  // The blake3 hold names a source list that differs from what build.rs compiles.
  const build = [
    'fn main() {}',
    'fn build_sse2_sse41_avx2_assembly() {',
    '    build.file("c/a_unix.S");',
    '}',
    'fn build_avx512_assembly() {',
    '    build.file("c/b_unix.S");',
    '}',
  ].join('\n');
  const read = rel => (rel === 'build.rs' ? build : '');
  const said = EXCEPTION_HOLDS.blake3.holds({ read, exception: { sources: ['c/a_unix.S'] } }).join('\n');
  assert.match(said, /reviews c\/a_unix\.S, and build\.rs compiles c\/a_unix\.S, c\/b_unix\.S/);
  assert.throws(() => rustExceptions('const OTHER: u8 = 1;'), /declares no EXCEPTIONS list/);
});

/**
 * The PREBUILT_MEMBERS list in isa_c_evidence.rs (re-review ruling 2): archive members put allows
 * that no judged compile produced, `{ package, version, member, reason }` each.
 */
export function rustPrebuiltMembers(source) {
  const list = source.match(/pub\(super\) const PREBUILT_MEMBERS: &\[PrebuiltMember\] = &\[([\s\S]*?)\];/)?.[1];
  if (list === undefined) throw new Error(`${EVIDENCE_RS} declares no PREBUILT_MEMBERS list`);
  return [...list.matchAll(/PrebuiltMember \{([\s\S]*?)\n {4}\},/g)].map(([, body]) => ({
    package: body.match(/package: "([^"]+)"/)?.[1],
    version: body.match(/version: "([^"]+)"/)?.[1],
    member: body.match(/member: "([^"]+)"/)?.[1],
    reason: body.match(/reason:\s*"((?:[^"\\]|\\[\s\S])*)"/)?.[1]?.replace(/\\\n\s*/g, '').replace(/\\"/g, '"'),
  }));
}

/** Problems with the prebuilt-member list against the lock: each entry locked at its version, with a one-sentence reason. */
export function prebuiltMemberProblems(entries, lock) {
  const problems = [];
  for (const e of entries) {
    if (!e.package || !e.member) problems.push(`a PREBUILT_MEMBERS entry names no package or member: ${JSON.stringify(e)}`);
    if (!lockedVersions(lock, e.package ?? '').includes(e.version)) problems.push(`PREBUILT_MEMBERS ${e.package} ${e.version} is not a version elohim/eprfs/Cargo.lock locks`);
    const reason = e.reason ?? '';
    if (reason.length < 40 || !reason.endsWith('.') || reason.includes('. ')) problems.push(`PREBUILT_MEMBERS ${e.package}/${e.member} gives no one-sentence reason`);
  }
  return problems;
}

test('fixture: a prebuilt archive member is allowed only by name, at a locked version, with its reason', () => {
  assert.deepEqual(rustPrebuiltMembers('pub(super) const PREBUILT_MEMBERS: &[PrebuiltMember] = &[];'), []);
  const one = [
    'pub(super) const PREBUILT_MEMBERS: &[PrebuiltMember] = &[',
    '    PrebuiltMember {',
    '        package: "zlib-sys",',
    '        version: "1.0.0",',
    '        member: "crc.o",',
    '        reason: "zlib-sys ships crc.o prebuilt for its CPUID-dispatched CRC path, reviewed.",',
    '    },',
    '];',
  ].join('\n');
  const parsed = rustPrebuiltMembers(one);
  assert.deepEqual(parsed.map(e => [e.package, e.version, e.member]), [['zlib-sys', '1.0.0', 'crc.o']]);
  const lock = '[[package]]\nname = "zlib-sys"\nversion = "1.0.0"\n';
  assert.deepEqual(prebuiltMemberProblems(parsed, lock), []);
  assert.match(prebuiltMemberProblems(parsed, lock.replace('1.0.0', '2.0.0')).join('\n'), /not a version/);
  assert.match(prebuiltMemberProblems([{ ...parsed[0], reason: 'short.' }], lock).join('\n'), /one-sentence/);
  assert.throws(() => rustPrebuiltMembers('const X: u8 = 0;'), /no PREBUILT_MEMBERS list/);
});

test('every prebuilt archive member put allows is locked and gives its reason', () => {
  assert.deepEqual(prebuiltMemberProblems(rustPrebuiltMembers(read(EVIDENCE_RS)), read('elohim/eprfs/Cargo.lock')), []);
});

const rustExceptionList = () => rustExceptions(read(EVIDENCE_RS));
const declaredClosures = () => {
  const packages = lockPackages(read('elohim/eprfs/Cargo.lock'));
  return [...new Set([...ccBuiltCrates(packages, 'elohim-epr-cli'), ...ccBuiltCrates(packages, 'elohim-device-runtime')])].sort();
};

// bin-launch and epr are marked env:isa-c by the same rule: put reads the compile commands of the
// build that produced their bytes, so a cc-built crate in either closure is judged or a named
// exception, and nothing passes for want of a string (E12 L2a rework, ruling 1(e)).
test('every crate cc builds in epr\'s or bin-launch\'s lock closure is judged or a named exception', () => {
  const built = declaredClosures();
  assert.ok(built.includes('ring'), 'ring builds with cc in epr\'s closure (the crate that trapped on shem)');
  assert.deepEqual(ccAccountProblems(built, CC_BUILT_IN_EPR, read('elohim/eprfs/epr-cli/src/exercise.rs'), rustExceptionList()), []);
});

// Ruling 1(d): each exception's reason is held against the locked crate's own source in the cargo
// registry, not by its name alone. A named test may not skip (the orchestrator-tree half reds an
// undeclared skip), so a source absent from the registry fails, naming the crate and version.
for (const exception of existsSync(resolve(ROOT, EVIDENCE_RS)) ? rustExceptionList() : []) {
  test(`the ${exception.package} exception holds against its locked build.rs`, () => {
    const holder = EXCEPTION_HOLDS[exception.package];
    assert.ok(holder, `no check holds the ${exception.package} exception`);
    const lock = read('elohim/eprfs/Cargo.lock');
    const versions = lockedVersions(lock, exception.package);
    assert.ok(versions.length > 0, `elohim/eprfs/Cargo.lock locks no ${exception.package}`);
    // Re-review ruling 3: the exception names the version it reviewed, and the lock holds it.
    assert.ok(versions.includes(exception.version),
      `the ${exception.package} exception reviews ${exception.version}, and elohim/eprfs/Cargo.lock locks ${versions.join(', ')}`);
    const located = (name, version) => {
      const dir = crateSource(name, version);
      assert.ok(dir, `${name}-${version} source is not in ${cargoHomes.join(' or ')}/registry/src, so the ${exception.package} exception cannot be held against it`);
      return dir;
    };
    const lockedCrate = name => {
      const [version] = lockedVersions(lock, name);
      assert.ok(version, `elohim/eprfs/Cargo.lock locks no ${name}`);
      return located(name, version);
    };
    for (const version of [exception.version]) {
      const dir = located(exception.package, version);
      const problems = holder.holds({ read: rel => readFileSync(join(dir, rel), 'utf8'), lockedCrate, exception });
      assert.deepEqual(problems, [], `${exception.package}-${version}: ${exception.reason}`);
    }
  });
}

test('fixture: an exception check names a build.rs that no longer says what its reason says', () => {
  const stacker = EXCEPTION_HOLDS.stacker.holds;
  const guarded = 'fn main() {\n    let mut cfg = cc::Build::new();\n    if target.contains("windows") {\n        cfg.file("src/arch/windows.c");\n        cfg.compile("libstacker.a");\n    }\n}\n';
  assert.deepEqual(stacker({ read: () => guarded }), []);
  assert.ok(stacker({ read: () => `${guarded}fn extra() { cc::Build::new().file("x.c").compile("x"); }\n` }).length > 0);
  const generator = EXCEPTION_HOLDS.generator.holds;
  const ppc = 'fn main() {\n    let external_assembly_required = target_arch == "powerpc64";\n    if external_assembly_required {\n        cc::Build::new().file("a.S").compile("a");\n    }\n}\n';
  assert.deepEqual(generator({ read: () => ppc }), []);
  assert.ok(generator({ read: () => ppc.replace('"powerpc64"', '"x86_64"') }).length > 0);
});

// ── Row 29.2 ruling: the only producer of a release for sibling draw is eprfs-produce, for a
// workspace declared_targets lists. A new producer for another workspace fails here until that
// workspace has a record. (hc-mesh.sh also crosses `artifact put`, without --release: mesh
// artifacts for storage, doorway and the beacon, whose locks the floors above hold at cc >= 1.11.) ─
const SCRIPT = /(^|\/)(justfile|Jenkinsfile[^/]*|[^/]+\.(sh|bash|mjs|js|py))$/;

/** The `artifact put` commands in a script text that put a release (`--release`), with line numbers. */
export function releasePuts(text) {
  const lines = text.split('\n');
  const found = [];
  lines.forEach((line, i) => {
    if (!/\bartifact put\b/.test(line) || /^\s*(#|\/\/)/.test(line)) return;
    let span = line;
    for (let j = i; j < lines.length - 1 && j < i + 8 && /\\\s*$/.test(lines[j]); j++) span += `\n${lines[j + 1]}`;
    if (/(^|\s)--release(\s|=)/.test(span)) found.push(i + 1);
  });
  return found;
}

test('fixture: a release put is found across continuations; a comment or a plain crossing put is not', () => {
  assert.deepEqual(releasePuts('x=$(epr artifact put --method m \\\n  --release "$r" --cross)'), [1]);
  assert.deepEqual(releasePuts('epr artifact put --method m --bytes b --cross'), []);
  assert.deepEqual(releasePuts('# then `epr artifact put --release <r>` of the gate\'s bytes'), []);
  assert.deepEqual(releasePuts('a\nepr artifact put --release=r'), [2]);
});

test('the only script that puts a release for sibling draw is eprfs-produce, for a declared workspace', () => {
  const tracked = execFileSync('git', ['ls-files', '-z'], { cwd: ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
    .split('\0').filter(path => path && SCRIPT.test(path) && !/\.test\.(mjs|js|sh)$|(^|\/)(__tests__|tests?)\//.test(path));
  const producers = tracked.filter(path => existsSync(resolve(ROOT, path)) && releasePuts(read(path)).length > 0);
  assert.deepEqual(producers, ['genesis/orchestrator/eprfs-produce.sh'],
    'a new release producer needs its workspace in pool-policy declared_targets (row 29.2) before it may offer to siblings');
  const policy = JSON.parse(read('genesis/agentic/pool-policy.json'));
  const workspaces = Object.entries(policy.declared_targets).filter(([k]) => !k.startsWith('_')).flatMap(([, t]) => t.workspaces);
  assert.match(read('genesis/orchestrator/eprfs-produce.sh'), /--project eprfs\b/);
  assert.ok(workspaces.includes('elohim/eprfs'), 'eprfs-produce offers the elohim/eprfs workspace, which must be declared');
});
