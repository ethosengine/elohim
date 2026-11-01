import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  dismissTargets,
  gitOracle,
  phaseLVerdict,
  pickLandingBase,
  precedesIntent,
} from './gate-landing-base.mjs';

const O = letter => letter.repeat(40);
const DEV = { commit: O('a'), tree: O('b') };
const ME = { change: `I${'1'.repeat(40)}`, device: 'did:key:zme' };
const SETTLEMENT = { max_skew_s: 30, freshness_s: 90 };
let serial = 0;
function row(overrides = {}) {
  serial += 1;
  return {
    intent: `bafy${'a'.repeat(8)}${serial.toString(2).replace(/0/g, 'a').replace(/1/g, 'b')}`,
    change: `I${serial.toString(16).padStart(40, '0')}`,
    tree: O('c'),
    base: DEV.tree,
    class: '0',
    closures: {},
    raisedAt: 1_000,
    device: ME.device,
    own: true,
    state: 'live',
    ...overrides,
  };
}
function oracle(rows, options = {}) {
  const landing = new Map(
    rows.map((r, i) => [r.change, { commit: O(String.fromCharCode(100 + i)), tree: r.tree }]),
  );
  return {
    hasObject: oid => options.missing !== oid,
    landingCommit: change => landing.get(change) || null,
    isAncestor: (a, b) => (options.notAncestor ? false : b !== DEV.commit),
    ownLive: r => options.ownLive?.get(r.intent) ?? null,
    landing,
  };
}
function pick(rows = [], options = {}) {
  const git = options.git || oracle(rows, options);
  return pickLandingBase({
    dev: DEV,
    queue: {
      intents: rows,
      advisory: [],
      siblingsRead: true,
      ...options.queue,
    },
    me: options.me || ME,
    settlement: SETTLEMENT,
    heads: options.heads || new Map(),
    now: options.now || 2_000,
    git,
    cap: options.cap || 4,
  });
}

test('unreadable queue degrades to dev base', async function unreadable_queue_degrades_to_dev_base(t) {
  const result = pickLandingBase({
    dev: DEV,
    queue: { error: 'boom', why: 'boom' },
    me: ME,
    settlement: SETTLEMENT,
    heads: new Map(),
    now: 0,
    git: oracle([]),
  });
  assert.equal(result.degraded, true);
  assert.deepEqual(result.base, DEV);
  assert.match(result.lines[0], /queue unreadable/);
});
test('empty queue gates on dev base', async function empty_queue_gates_on_dev_base(t) {
  const result = pick();
  assert.equal(result.fromDev, true);
  assert.ok(!result.lines.some(line => line.includes('queue unreadable')));
});
test('alone daemon down only own rows chain', async function alone_daemon_down_only_own_rows_chain(t) {
  const sibling = row({ device: 'did:key:zother', own: false });
  const own = row({ tree: O('d') });
  const result = pick([sibling, own]);
  assert.deepEqual(result.chain, [own.intent]);
  assert.ok(result.skipped.some(item => item.why === 'dormant'));
});
test('no change id no chain position', async function no_change_id_no_chain_position(t) {
  const result = pick([], { me: { ...ME, change: null } });
  assert.equal(result.chainLen, 0);
  assert.match(result.lines[0], /no Change-Id/);
});
test('chain follows base pointers', async function chain_follows_base_pointers(t) {
  const a = row({ tree: O('d') });
  const b = row({ base: a.tree, tree: O('e') });
  const git = oracle([a, b]);
  git.landing.set(a.change, { commit: O('f'), tree: a.tree });
  git.landing.set(b.change, { commit: O('1'), tree: b.tree });
  assert.deepEqual(pick([a, b], { git }).chain, [a.intent, b.intent]);
});
test('fork tiebreak is order free', async function fork_tiebreak_is_order_free(t) {
  const a = row({ raisedAt: 1_000, device: 'did:key:za' });
  const b = row({ raisedAt: 2_000, device: 'did:key:zb' });
  assert.equal(precedesIntent(a, b, 30), true);
  assert.deepEqual(
    pick([a, b], {
      heads: new Map([
        [a.device, 1],
        [b.device, 1],
      ]),
    }).chain[0],
    a.intent,
  );
  assert.deepEqual(
    pick([b, a], {
      heads: new Map([
        [a.device, 1],
        [b.device, 1],
      ]),
    }).chain[0],
    a.intent,
  );
});
test('fork_presort_is_code_unit_not_locale', async function fork_presort_is_code_unit_not_locale(t) {
  const upperB = row({ raisedAt: 40_000, device: 'did:key:z6MkB' });
  const lowerA = row({ raisedAt: 0, device: 'did:key:z6Mka' });
  const upperC = row({ raisedAt: 10_000, device: 'did:key:z6MkC' });
  assert.equal(upperB.device < lowerA.device, true);
  assert.equal(upperB.device.localeCompare(lowerA.device) > 0, true);
  const heads = new Map([
    [upperB.device, 1],
    [lowerA.device, 1],
    [upperC.device, 1],
  ]);
  assert.equal(pick([upperB, lowerA, upperC], { heads }).chain[0], lowerA.intent);
});
test('never chains behind governance', async function never_chains_behind_governance(t) {
  for (const value of ['governance', 'high', undefined, 'mystery'])
    assert.equal(pick([row({ class: value })]).chainLen, 0);
});
test('sibling bytes absent stops chain', async function sibling_bytes_absent_stops_chain(t) {
  const r = row({ device: 'did:key:zother', own: false });
  const result = pick([r], {
    heads: new Map([[r.device, 1]]),
    missing: r.tree,
  });
  assert.equal(result.skipped[0].why, 'bytes not on this device');
});
test('landing ref must descend from predecessor', async function landing_ref_must_descend_from_predecessor(t) {
  const result = pick([row()], { notAncestor: true });
  assert.match(result.stopped.why, /does not descend/);
});
test('dormant head skipped by local clock', async function dormant_head_skipped_by_local_clock(t) {
  for (const age of [91, null, undefined]) {
    const r = row({
      device: 'did:key:zother',
      own: false,
      raisedAt: 9_999_999,
    });
    const heads = new Map();
    if (age !== undefined) heads.set(r.device, age);
    assert.equal(pick([r], { heads }).skipped[0].why, 'dormant');
  }
  const live = row({ device: 'did:key:zother', own: false });
  assert.equal(pick([live], { heads: new Map([[live.device, 10]]) }).chainLen, 1);
});
test('own worktree liveness signal', async function own_worktree_liveness_signal(t) {
  const old = row({ raisedAt: 0 });
  const recent = row({ raisedAt: 9_999_000 });
  const unknown = row({ raisedAt: 0 });
  assert.equal(pick([old], { now: 7_200_000, ownLive: new Map([[old.intent, true]]) }).chainLen, 1);
  assert.equal(
    pick([recent], {
      now: 10_000_000,
      ownLive: new Map([[recent.intent, false]]),
    }).chainLen,
    0,
  );
  assert.equal(pick([unknown], { now: 7_200_000 }).chainLen, 0);
});
test('own prior intent superseded never chained', async function own_prior_intent_superseded_never_chained(t) {
  assert.equal(pick([row({ change: ME.change })]).chainLen, 0);
});
test('advisory row from other device skips', async function advisory_row_from_other_device_skips(t) {
  const r = row();
  const result = pick([r], {
    queue: {
      advisory: [{ intent: r.intent, via: 'did:key:zother', kind: 'withdraw' }],
    },
  });
  assert.equal(result.skipped[0].why, 'advisory withdraw by did:key:zother');
});
test('landed by observation', async function landed_by_observation(t) {
  const sameTree = row({ tree: DEV.tree });
  assert.deepEqual(pick([sameTree]).landedObserved, [sameTree.intent]);
  const ancestor = row();
  const git = oracle([ancestor]);
  git.isAncestor = () => true;
  assert.deepEqual(pick([ancestor], { git }).landedObserved, [ancestor.intent]);
});
test('malformed sibling row excluded', async function malformed_sibling_row_excluded(t) {
  let called = false;
  const bad = row({ change: '../../x' });
  const git = oracle([]);
  git.landingCommit = () => {
    called = true;
    return null;
  };
  assert.equal(pick([bad], { git }).chainLen, 0);
  assert.equal(called, false);
});
test('duplicate cid rows fold once', async function duplicate_cid_rows_fold_once(t) {
  const a = row();
  const b = { ...a };
  assert.equal(pick([a, b]).chainLen, 1);
});
test('cap four ends chain', async function cap_four_ends_chain(t) {
  const rows = [];
  let base = DEV.tree;
  for (const tree of ['c', 'd', 'e', 'f', '1']) {
    const r = row({ base, tree: O(tree) });
    rows.push(r);
    base = r.tree;
  }
  const result = pick(rows);
  assert.equal(result.chainLen, 4);
  assert.equal(result.skipped.at(-1).why, 'cap');
});
test('unverified intent included', async function unverified_intent_included(t) {
  assert.equal(pick([row({ verified: false, verifiedAt: null })]).chainLen, 1);
});
test('phase l compares commit then tree', async function phase_l_compares_commit_then_tree(t) {
  assert.equal(phaseLVerdict({ dev: DEV, predicted: DEV }), 'ff');
  assert.equal(
    phaseLVerdict({ dev: DEV, predicted: { commit: O('c'), tree: DEV.tree } }),
    'same-tree',
  );
  assert.equal(phaseLVerdict({ dev: DEV, predicted: { commit: O('c'), tree: O('d') } }), 'moved');
});
test('dismiss targets whole chain', async function dismiss_targets_whole_chain(t) {
  const chain = ['bafya', 'bafyb'];
  const result = dismissTargets({
    chain,
    project: 'app',
    redAtPredicted: true,
    greenAtDev: true,
    keyPredicted: 'kp',
    keyDev: 'kd',
    heads: new Map([['did:key:z', 3]]),
    now: 42,
  });
  assert.deepEqual(result.targets, chain);
  assert.deepEqual(Object.keys(result.capsule), [
    'key_predicted',
    'key_dev',
    'project',
    'chain',
    'heads',
    'now',
  ]);
  for (const value of Object.values(result.capsule)) {
    assert.equal(typeof value, 'string');
    assert.ok(!value.includes('\n'));
  }
});

function cleanEnv(scratch) {
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
  return {
    ...env,
    HOME: scratch,
    GIT_CEILING_DIRECTORIES: dirname(scratch),
    GIT_CONFIG_NOSYSTEM: '1',
    GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_AUTHOR_NAME: 'T',
    GIT_AUTHOR_EMAIL: 't@t',
    GIT_COMMITTER_NAME: 'T',
    GIT_COMMITTER_EMAIL: 't@t',
    GIT_AUTHOR_DATE: '2026-10-31T12:00:00Z',
    GIT_COMMITTER_DATE: '2026-10-31T12:00:00Z',
  };
}
test('git oracle real repo', async function git_oracle_real_repo(t) {
  const dir = mkdtempSync(join(tmpdir(), 'landing-oracle-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const env = cleanEnv(dir);
  const git = (...args) => spawnSync('git', args, { cwd: dir, env, encoding: 'utf8' });
  git('init');
  writeFileSync(join(dir, 'a'), 'a');
  git('add', 'a');
  git('commit', '-m', 'a');
  const commit = git('rev-parse', 'HEAD').stdout.trim();
  const tree = git('rev-parse', 'HEAD^{tree}').stdout.trim();
  git('update-ref', `refs/landing/${ME.change}`, commit);
  const iso = (bin, args, opts) => spawnSync(bin, args, { ...opts, env });
  const o = gitOracle(dir, iso);
  assert.equal(o.hasObject(tree), true);
  assert.deepEqual(o.landingCommit(ME.change), { commit, tree });
  assert.equal(o.isAncestor(commit, commit), true);
  assert.equal(o.ownLive({}), null);
});
test('pick cli reads heads policy and device', async function pick_cli_reads_heads_policy_and_device(t) {
  const dir = mkdtempSync(join(tmpdir(), 'landing-cli-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const env = cleanEnv(dir);
  const git = (...args) => spawnSync('git', args, { cwd: dir, env, encoding: 'utf8' });
  git('init');
  writeFileSync(join(dir, 'a'), 'a');
  git('add', 'a');
  git('commit', '-m', 'a');
  const commit = git('rev-parse', 'HEAD').stdout.trim();
  const tree = git('rev-parse', 'HEAD^{tree}').stdout.trim();
  const heads = join(dir, 'heads.json');
  const policy = join(dir, 'policy.json');
  writeFileSync(heads, JSON.stringify({ devices: [{ device: ME.device, ageS: 1 }] }));
  writeFileSync(policy, JSON.stringify({ berth: { channel: SETTLEMENT } }));
  const module = join(dirname(fileURLToPath(import.meta.url)), 'gate-landing-base.mjs');
  const result = spawnSync(
    process.execPath,
    [
      module,
      'pick',
      '--dev',
      commit,
      '--change',
      ME.change,
      '--device',
      ME.device,
      '--heads',
      heads,
      '--policy',
      policy,
      '--now',
      '1',
    ],
    {
      cwd: dir,
      env,
      input: JSON.stringify({ intents: [], advisory: [], siblingsRead: true }),
      encoding: 'utf8',
    },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).base.tree, tree);
});
test('pick cli rejects an unresolved dev', async function pick_cli_rejects_an_unresolved_dev(t) {
  const dir = mkdtempSync(join(tmpdir(), 'landing-cli-dev-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const env = cleanEnv(dir);
  const git = (...args) => spawnSync('git', args, { cwd: dir, env, encoding: 'utf8' });
  assert.equal(git('init').status, 0);
  const module = join(dirname(fileURLToPath(import.meta.url)), 'gate-landing-base.mjs');
  const result = spawnSync(
    process.execPath,
    [module, 'pick', '--dev', 'missing-dev', '--change', ME.change, '--device', ME.device],
    { cwd: dir, env, input: '{}', encoding: 'utf8' },
  );
  assert.equal(result.status, 2);
  assert.equal(result.stdout, '');
  assert.match(result.stderr, /^\[landing\] --dev missing-dev does not resolve: fatal:/);
});
