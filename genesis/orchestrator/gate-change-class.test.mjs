import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  CHANGE_CLASSES,
  GOVERNANCE_PATHS,
  changeClass,
  classifyChange,
  classifyPath,
  classifyPoolPolicy,
} from './gate-change-class.mjs';
import { SIBLING_REUSE_GOVERNANCE_PATHS } from './gate-evidence.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
function fakeSpawn(table) {
  const calls = [];
  const spawn = (bin, args, opts) => {
    calls.push({ bin, args, opts });
    const found = table.find(([match]) => match(bin, args));
    return found ? found[1] : { status: 1, stdout: '', stderr: 'unmatched' };
  };
  return { spawn, calls };
}
const classified = paths => classifyChange({ paths });

test('classifier change is governance', async function classifier_change_is_governance(t) {
  for (const path of [
    'genesis/orchestrator/gate-change-class.mjs',
    'genesis/orchestrator/gate-landing-base.mjs',
    'genesis/orchestrator/gate-change-id.mjs',
    'genesis/orchestrator/commit-tag-parser.mjs',
    '.claude/epr-meta/measures.yaml',
    'elohim/eprfs/epr-cli/src/peer.rs',
    'genesis/orchestrator/scripts/brit-helper.sh',
    'genesis/orchestrator/scripts/t2-receipt.sh',
    'genesis/agentic/bin/pool-lib.sh',
    'genesis/orchestrator/rust-style/src/lib.rs',
    '.epr-meta/manifest.md',
    '.claude/scripts/ci-ignore-projector.py',
  ])
    assert.equal(classified([path]).class, 'governance', path);
});

test('governance covers the runner import closure', async function governance_covers_the_runner_import_closure(t) {
  const expected = [
    'genesis/orchestrator/gate-runner.mjs',
    'genesis/orchestrator/gate-evidence.mjs',
    'genesis/orchestrator/ci-ignore.mjs',
    'genesis/orchestrator/manifest-utils.mjs',
    'genesis/orchestrator/pipeline-registry.mjs',
    'genesis/orchestrator/graph-walker.mjs',
    'genesis/orchestrator/cargo-affected.mjs',
    'genesis/orchestrator/run-local-gate.sh',
    'genesis/orchestrator/rust-style.sh',
    'genesis/orchestrator/cargo-lint.mjs',
    'genesis/orchestrator/gate-steps.sh',
    'genesis/agentic/bin/cargo-pool',
    'genesis/agentic/bin/berth',
  ];
  for (const path of expected) assert.equal(classified([path]).reexec, true, path);
});

test('ci ignored paths still classify', async function ci_ignored_paths_still_classify(t) {
  assert.equal(classified(['.husky/pre-push.bash']).class, 'governance');
});

test('rename out of glob is governance', async function rename_out_of_glob_is_governance(t) {
  const fake = fakeSpawn([
    [(_b, a) => a[0] === 'merge-base', { status: 0, stdout: `${'a'.repeat(40)}\n`, stderr: '' }],
    [
      (_b, a) => a[0] === 'diff',
      {
        status: 0,
        stdout: 'genesis/orchestrator/gate-runner.mjs\ngenesis/orchestrator/lib/runner.mjs\n',
        stderr: '',
      },
    ],
  ]);
  assert.equal(changeClass({ root: ROOT, spawn: fake.spawn }).class, 'governance');
  assert.ok(fake.calls.find(call => call.args[0] === 'diff').args.includes('--no-renames'));
});

test('channel vetoes siblings without reexec', async function channel_vetoes_siblings_without_reexec(t) {
  const result = classified(['elohim/eprfs/eprfs-peer/src/fold.rs']);
  assert.equal(result.class, 'channel');
  assert.equal(result.siblingVeto, true);
  assert.equal(result.reexec, false);
});

test('sibling fence is a subset', async function sibling_fence_is_a_subset(t) {
  const samples = SIBLING_REUSE_GOVERNANCE_PATHS.filter(
    path => path !== 'genesis/agentic/pool-policy.json',
  ).map(path => path.replace('/**', '/x').replace('*', 'x'));
  for (const path of samples) assert.equal(classified([path]).siblingVeto, true, path);
});

test('pool policy classified by key', async function pool_policy_classified_by_key(t) {
  const pair = (before, after) => classifyPoolPolicy(JSON.stringify(before), JSON.stringify(after));
  assert.equal(
    pair({ berth: { channel: { crosses: 1 } } }, { berth: { channel: { crosses: 2 } } })[0].class,
    'channel',
  );
  assert.deepEqual(
    pair({ cargo_env_overrides: { eprfs: 1 } }, { cargo_env_overrides: { eprfs: 2 } })[0].reuseOff,
    ['eprfs'],
  );
  assert.deepEqual(
    pair({ cargo_env_overrides: { '*': 1 } }, { cargo_env_overrides: { '*': 2 } })[0].reuseOff,
    ['*'],
  );
  assert.equal(pair({ gate_reuse: 1 }, { gate_reuse: 2 })[0].class, 'governance');
  assert.equal(pair({ _comment: 1 }, { _comment: 2 })[0].class, '0');
  assert.equal(classifyPoolPolicy('{', '{}')[0].class, 'governance');
});

test('dna hash paths are high', async function dna_hash_paths_are_high(t) {
  assert.equal(classified(['elohim/holochain/dna/imagodei/dna.yaml']).class, 'high');
  assert.equal(
    classified(['elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs']).class,
    'high',
  );
  assert.equal(classified(['elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs']).class, '0');
});

test('flags union not top class', async function flags_union_not_top_class(t) {
  const result = classified(['elohim/holochain/dna/imagodei/dna.yaml', '.husky/pre-push']);
  assert.equal(result.class, 'high');
  assert.equal(result.reexec, true);
});

test('merge base unavailable fails closed', async function merge_base_unavailable_fails_closed(t) {
  const fake = fakeSpawn([]);
  const result = changeClass({ root: ROOT, spawn: fake.spawn });
  assert.equal(result.class, 'governance');
  assert.equal(result.base, null);
  assert.equal(result.cure, 'git fetch origin');
});

test('only gate test files are excluded from governance', async function only_gate_test_files_are_excluded_from_governance(t) {
  assert.equal(classified(['genesis/orchestrator/gate-evidence.test.mjs']).class, 'channel');
  assert.equal(classified(['.husky/hooks-path.test.mjs']).class, 'governance');
});

test('cli discovers the repository root from cwd', async function cli_discovers_the_repository_root_from_cwd(t) {
  const dir = mkdtempSync(join(tmpdir(), 'change-class-cli-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const elsewhere = join(dir, 'elsewhere');
  mkdirSync(elsewhere);
  const env = { ...process.env };
  for (const key of [
    'GIT_DIR',
    'GIT_WORK_TREE',
    'GIT_INDEX_FILE',
    'GIT_PREFIX',
    'GIT_COMMON_DIR',
    'NODE_TEST_CONTEXT',
  ])
    delete env[key];
  Object.assign(env, {
    HOME: dir,
    GIT_CONFIG_NOSYSTEM: '1',
    GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_AUTHOR_NAME: 'T',
    GIT_AUTHOR_EMAIL: 't@t',
    GIT_COMMITTER_NAME: 'T',
    GIT_COMMITTER_EMAIL: 't@t',
  });
  const git = (...args) => spawnSync('git', args, { cwd: dir, env, encoding: 'utf8' });
  assert.equal(git('init', '-b', 'dev').status, 0);
  writeFileSync(join(dir, 'ordinary'), 'base\n');
  git('add', 'ordinary');
  assert.equal(git('commit', '-m', 'base').status, 0);
  assert.equal(git('checkout', '-b', 'topic').status, 0);
  writeFileSync(join(dir, 'ordinary'), 'topic\n');
  git('add', 'ordinary');
  assert.equal(git('commit', '-m', 'topic').status, 0);
  const module = resolve(dirname(fileURLToPath(import.meta.url)), 'gate-change-class.mjs');
  const result = spawnSync(process.execPath, [module, '--json'], {
    cwd: elsewhere,
    env,
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).class, '0');
});

test('classes match the rust literal', async function classes_match_the_rust_literal(t) {
  const text = readFileSync(resolve(ROOT, 'elohim/eprfs/epr-cli/src/flow/intend.rs'), 'utf8');
  const match = text.match(/CLASSES_LITERAL[^=]*=\s*\[([^\]]*)\]/);
  assert.ok(match, 'CLASSES_LITERAL must be a single-line array');
  const values = [...match[1].matchAll(/"([^"]+)"/g)].map(row => row[1]);
  assert.deepEqual(values, CHANGE_CLASSES);
});
