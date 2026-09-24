import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { judge, observationArgs, readCheck, runAttested } from './gate-attest.mjs';

const attestation = { provider: 'github-checks', repo: 'ethosengine/brit', check: 'Tests pass' };
const SHA = 'b86c5104d3398d89fe56d23199a9393fec534870';

function ghReturning(runs) {
  return (bin, args) => ({ status: 0, stdout: JSON.stringify({ check_runs: runs }), stderr: '', args });
}
const run = (name, status, conclusion, started_at) => ({ name, status, conclusion, started_at });

describe('readCheck — the newest run of the named check', () => {
  test('asks the check-runs endpoint for the SHA and returns the newest named run', () => {
    let seen;
    const spawn = (bin, args) => { seen = args; return ghReturning([run('Tests pass', 'completed', 'failure', '2026-09-01T00:00:00Z'), run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z'), run('lint', 'completed', 'failure', '2026-09-03T00:00:00Z')])(bin, args); };
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn });
    assert.deepEqual(read, { read: 'ok', status: 'completed', conclusion: 'success' });
    assert.deepEqual(seen, ['api', `repos/ethosengine/brit/commits/${SHA}/check-runs?per_page=100`]);
  });
  test('an absent check reads as conclusion "absent"', () => {
    assert.deepEqual(readCheck(attestation, SHA, { ghBin: 'gh', spawn: ghReturning([run('lint', 'completed', 'success', '2026-09-01T00:00:00Z')]) }), { read: 'ok', status: 'absent', conclusion: 'absent' });
  });
  test('a gh failure is read: failed with the stderr as reason', () => {
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn: () => ({ status: 4, stdout: '', stderr: 'gh: not logged in' }) });
    assert.deepEqual(read, { read: 'failed', reason: 'gh: not logged in' });
  });
  test('no gh binary is read: failed', () => {
    assert.deepEqual(readCheck(attestation, SHA, { ghBin: null, spawn: () => { throw new Error('must not spawn'); } }), { read: 'failed', reason: 'gh not found' });
  });
});

describe('judge — four outcomes', () => {
  test('success passes as witnessed', () => {
    const v = judge({ read: 'ok', status: 'completed', conclusion: 'success' }, attestation, SHA);
    assert.equal(v.outcome, 'pass'); assert.equal(v.tier, 'witnessed'); assert.equal(v.conclusion, 'success');
    assert.equal(v.line, 'attested: ethosengine/brit@b86c5104d339 Tests pass success');
  });
  for (const bad of ['failure', 'cancelled', 'timed_out', 'action_required']) {
    test(`${bad} refuses as witnessed`, () => {
      const v = judge({ read: 'ok', status: 'completed', conclusion: bad }, attestation, SHA);
      assert.equal(v.outcome, 'refuse'); assert.equal(v.tier, 'witnessed'); assert.equal(v.conclusion, 'failure');
      assert.match(v.line, new RegExp(`Tests pass ${bad} at b86c5104d339 — a pin without its attestation is not a green pin`));
    });
  }
  test('absent refuses', () => {
    const v = judge({ read: 'ok', status: 'absent', conclusion: 'absent' }, attestation, SHA);
    assert.equal(v.outcome, 'refuse'); assert.equal(v.conclusion, 'absent');
  });
  test('in progress refuses as pending', () => {
    const v = judge({ read: 'ok', status: 'in_progress', conclusion: null }, attestation, SHA);
    assert.equal(v.outcome, 'refuse'); assert.equal(v.conclusion, 'pending');
    assert.match(v.line, /not yet concluded/);
  });
  test('an unreachable read passes as claimed', () => {
    const v = judge({ read: 'failed', reason: 'offline' }, attestation, SHA);
    assert.equal(v.outcome, 'pass'); assert.equal(v.tier, 'claimed'); assert.equal(v.conclusion, 'unreachable');
    assert.equal(v.line, 'attested: claimed — offline');
  });
});

describe('observationArgs', () => {
  test('shapes the epr flow note exactly as measures.yaml declares', () => {
    assert.deepEqual(observationArgs('elohim/brit', attestation, { conclusion: 'success', tier: 'witnessed' }), [
      'flow', 'note', '--kind', 'observation', '--measure', 'pin-attestation@1',
      '--subject', 'elohim/brit', '--value', '1', '--unit', 'reads',
      '--env', 'provider=github-checks', '--env', 'check=Tests pass', '--env', 'conclusion=success', '--env', 'tier=witnessed',
    ]);
  });
});

describe('runAttested — reads the pin, not the checkout', () => {
  const project = { name: 'brit', dir: 'elohim/brit', run: { kind: 'attested', attestation } };
  function fakeGit({ sha = SHA, dirty = false, gitlink = true } = {}) {
    return (bin, args) => {
      if (bin !== 'git') return null;
      if (args.includes('rev-parse')) return gitlink ? { status: 0, stdout: `${sha}\n`, stderr: '' } : { status: 128, stdout: '', stderr: 'fatal: path not in tree' };
      if (args.includes('status')) return { status: 0, stdout: dirty ? ' M Cargo.toml\n' : '', stderr: '' };
      return { status: 1, stdout: '', stderr: `unexpected git ${args.join(' ')}` };
    };
  }
  function harness(gh, git) {
    const lines = []; const epr = [];
    const spawn = (bin, args, o) => bin === 'git' ? git(bin, args, o) : gh(bin, args, o);
    return { lines, epr, deps: { root: '/repo', env: { GH_BIN: process.execPath }, spawn, log: l => lines.push(l), runEpr: a => { epr.push(a); return 0; } } };
  }

  test('green check → exit 0, one attested line, one witnessed observation', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit());
    assert.equal(runAttested(project, h.deps), 0);
    assert.deepEqual(h.lines, ['attested: ethosengine/brit@b86c5104d339 Tests pass success']);
    assert.equal(h.epr.length, 1);
    assert.ok(h.epr[0].includes('tier=witnessed'));
  });
  test('red check → exit 1', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'failure', '2026-09-02T00:00:00Z')]), fakeGit());
    assert.equal(runAttested(project, h.deps), 1);
    assert.ok(h.epr[0].includes('conclusion=failure'));
  });
  test('unreachable → exit 0 as claimed', () => {
    const h = harness(() => ({ status: 1, stdout: '', stderr: 'offline' }), fakeGit());
    assert.equal(runAttested(project, h.deps), 0);
    assert.deepEqual(h.lines, ['attested: claimed — offline']);
    assert.ok(h.epr[0].includes('tier=claimed'));
  });
  test('a dirty worktree is reported on its own line and changes nothing', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit({ dirty: true }));
    assert.equal(runAttested(project, h.deps), 0);
    assert.equal(h.lines[0], 'attested: worktree dirty — the committed pin is what is read; the component’s own gate governs the worktree');
  });
  test('a path that is not a gitlink is a manifest error (exit 2), not a gate red', () => {
    const h = harness(ghReturning([]), fakeGit({ gitlink: false }));
    assert.equal(runAttested(project, h.deps), 2);
    assert.equal(h.epr.length, 0);
  });
  test('the observation is best-effort: a failing epr never changes the exit', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit());
    h.deps.runEpr = () => { throw new Error('epr missing'); };
    assert.equal(runAttested(project, h.deps), 0);
  });
});
