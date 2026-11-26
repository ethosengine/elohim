import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const SCRIPT = fileURLToPath(new URL('./push-changed-files.sh', import.meta.url));
const ZERO_SHA = '0000000000000000000000000000000000000000';
const REMOTE_REF = 'refs/heads/fixture';

// This is the former pre-push block verbatim. The fixture wrapper only supplies its variables
// and emits the values which the surrounding hook observes.
const OLD_BLOCK = String.raw`
if [ "$REMOTE_SHA" = "$ZERO_SHA" ]; then
  GATE_NARROW_BASE=none
  if ! NEW_CHANGED=$(git diff --name-only "origin/dev...$LOCAL_SHA" 2>/dev/null); then
    CHANGE_UNREADABLE="$CHANGE_UNREADABLE $REMOTE_REF"
    NEW_CHANGED=""
  fi
else
  if ! NEW_CHANGED=$(git diff --name-only "$REMOTE_SHA" "$LOCAL_SHA" 2>/dev/null); then
    GATE_NARROW_BASE=none
    if ! NEW_CHANGED=$(git diff --name-only "origin/dev...$LOCAL_SHA" 2>/dev/null); then
      CHANGE_UNREADABLE="$CHANGE_UNREADABLE $REMOTE_REF"
      NEW_CHANGED=""
    fi
  fi
fi
`;

function git(repo, ...args) {
  const result = spawnSync('git', args, {
    cwd: repo,
    encoding: 'utf8',
    env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' },
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}

function commit(repo, message, files) {
  for (const [path, content] of Object.entries(files)) {
    const full = join(repo, path);
    mkdirSync(join(full, '..'), { recursive: true });
    writeFileSync(full, content);
  }
  git(repo, 'add', ...Object.keys(files));
  git(repo, 'commit', '-q', '-m', message);
  return git(repo, 'rev-parse', 'HEAD');
}

function fixture() {
  const repo = mkdtempSync(join(tmpdir(), 'push-changed-files-'));
  git(repo, 'init', '-q');
  git(repo, 'config', 'user.name', 'Range Fixture');
  git(repo, 'config', 'user.email', 'range@example.invalid');
  const base = commit(repo, 'base', { 'base.txt': 'base\n' });
  git(repo, 'update-ref', 'refs/remotes/origin/dev', base);
  return { repo, base };
}

function oldRange(repo, destination, tip, remoteRef = REMOTE_REF) {
  const wrapper = `
REMOTE_SHA=$1
LOCAL_SHA=$2
REMOTE_REF=$3
ZERO_SHA=${ZERO_SHA}
GATE_NARROW_BASE=ok
CHANGE_UNREADABLE=""
NEW_CHANGED=""
${OLD_BLOCK}
printf '%s' "$NEW_CHANGED"
printf '%s\\0%s' "$GATE_NARROW_BASE" "$CHANGE_UNREADABLE" >&2
`;
  const result = spawnSync('bash', ['-c', wrapper, 'old-range', destination, tip, remoteRef], {
    cwd: repo,
    encoding: 'buffer',
  });
  assert.equal(result.status, 0, result.stderr.toString());
  const [gateNarrowBase, changeUnreadable] = result.stderr.toString().split('\0');
  return { paths: result.stdout.toString(), gateNarrowBase, changeUnreadable };
}

function sharedRange(repo, destination, tip, remoteRef = REMOTE_REF) {
  const result = spawnSync('bash', [SCRIPT, destination, tip], {
    cwd: repo,
    encoding: 'utf8',
  });
  const gateNarrowBase = result.status === 0 ? 'ok' : 'none';
  const changeUnreadable = result.status === 4 ? ` ${remoteRef}` : '';
  return { paths: result.stdout.replace(/\n$/, ''), gateNarrowBase, changeUnreadable, status: result.status };
}

function pushAccumulation(repo, refs, range) {
  let changed = '';
  let gateNarrowBase = 'ok';
  let changeUnreadable = '';
  let calls = 0;
  for (const { localSha, remoteRef, remoteSha } of refs) {
    if (localSha === ZERO_SHA) continue;
    calls += 1;
    const result = range(repo, remoteSha, localSha, remoteRef);
    if (result.gateNarrowBase === 'none') gateNarrowBase = 'none';
    changeUnreadable += result.changeUnreadable;
    if (result.paths) changed = `${changed}\n${result.paths}`;
  }
  return { changed, gateNarrowBase, changeUnreadable, calls };
}

function assertParity(repo, destination, tip, expectedStatus) {
  const old = oldRange(repo, destination, tip);
  const shared = sharedRange(repo, destination, tip);
  assert.equal(shared.status, expectedStatus, shared.paths);
  assert.deepEqual(
    { paths: shared.paths, gateNarrowBase: shared.gateNarrowBase, changeUnreadable: shared.changeUnreadable },
    old,
  );
}

test('destination_to_tip_matches_the_former_hook_block', () => {
  const { repo, base } = fixture();
  const tip = commit(repo, 'tip', { 'z-last.txt': 'z\n', 'a-first.txt': 'a\n' });
  assertParity(repo, base, tip, 0);
});

test('missing_destination_falls_back_to_origin_dev_merge_base', () => {
  const { repo } = fixture();
  const tip = commit(repo, 'tip', { 'fallback.txt': 'fallback\n' });
  assertParity(repo, 'destination-this-clone-lacks', tip, 3);
});

// A malformed pre-push line (three fields) leaves the destination empty. The former block's direct
// diff on an empty name failed and fell to the merge-base range: the ordinary path, never unreadable.
test('empty_destination_reads_the_merge_base_range_as_the_former_hook_block_did', () => {
  const { repo } = fixture();
  const tip = commit(repo, 'tip', { 'malformed.txt': 'malformed\n' });
  assertParity(repo, '', tip, 3);
});

test('new_ref_uses_origin_dev_merge_base', () => {
  const { repo } = fixture();
  const tip = commit(repo, 'tip', { 'new-ref.txt': 'new\n' });
  assertParity(repo, ZERO_SHA, tip, 3);
});

test('merge_commit_range_preserves_the_former_path_order', () => {
  const { repo, base } = fixture();
  git(repo, 'checkout', '-q', '-b', 'side');
  commit(repo, 'side', { 'side.txt': 'side\n' });
  git(repo, 'checkout', '-q', '-b', 'tip', base);
  commit(repo, 'tip', { 'tip.txt': 'tip\n' });
  git(repo, 'merge', '-q', '--no-ff', '-m', 'merge side', 'side');
  assertParity(repo, base, git(repo, 'rev-parse', 'HEAD'), 0);
});

test('unreadable_ranges_match_the_former_empty_refusal_effects', () => {
  const { repo } = fixture();
  git(repo, 'update-ref', '-d', 'refs/remotes/origin/dev');
  assertParity(repo, 'missing-destination', 'missing-tip', 4);
});

test('two_refs_accumulate_the_same_changed_path_union_as_the_former_hook_block', () => {
  const { repo, base } = fixture();
  const first = commit(repo, 'first ref', { 'first.txt': 'first\n' });
  const second = commit(repo, 'second ref', { 'second.txt': 'second\n' });
  const refs = [
    { localSha: first, remoteRef: 'refs/heads/first', remoteSha: base },
    { localSha: second, remoteRef: 'refs/heads/second', remoteSha: first },
  ];
  const former = pushAccumulation(repo, refs, oldRange);
  const shared = pushAccumulation(repo, refs, sharedRange);
  assert.deepEqual(shared, former);
  assert.equal(shared.changed, '\nfirst.txt\nsecond.txt');
  assert.equal(shared.calls, 2);
});

test('tag_push_matches_the_former_hook_accumulation', () => {
  const { repo, base } = fixture();
  const tip = commit(repo, 'tagged tip', { 'tagged.txt': 'tagged\n' });
  const refs = [{ localSha: tip, remoteRef: 'refs/tags/v1', remoteSha: base }];
  const former = pushAccumulation(repo, refs, oldRange);
  const shared = pushAccumulation(repo, refs, sharedRange);
  assert.deepEqual(shared, former);
  assert.equal(shared.changed, '\ntagged.txt');
  assert.equal(shared.calls, 1);
});

test('delete_push_skips_the_range_script_and_leaves_accumulation_unchanged', () => {
  const { repo, base } = fixture();
  const refs = [{ localSha: ZERO_SHA, remoteRef: 'refs/heads/deleted', remoteSha: base }];
  const former = pushAccumulation(repo, refs, oldRange);
  const shared = pushAccumulation(repo, refs, sharedRange);
  assert.deepEqual(shared, former);
  assert.deepEqual(shared, {
    changed: '',
    gateNarrowBase: 'ok',
    changeUnreadable: '',
    calls: 0,
  });
});
