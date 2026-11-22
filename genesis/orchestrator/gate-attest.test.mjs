import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import {
  CLAIMED_EXIT_CODE,
  judge,
  judgePin,
  observationArgs,
  readCheck,
  runAttested,
} from './gate-attest.mjs';

const attestation = { provider: 'github-checks', repo: 'ethosengine/brit', check: 'Tests pass' };
const SHA = 'b86c5104d3398d89fe56d23199a9393fec534870';

const SUITE = 4242;
const WF = 7;
/** One gh fake for both reads: check-runs for the commit, then the commit's workflow runs (default: the one run that made the check). */
function ghReturning(runs, workflowRuns = [wfRun(1, 'ci', 'completed', 'success', SUITE, '2026-09-02T00:00:00Z')]) {
  return (bin, args) => {
    const body = String(args[1]).includes('/actions/runs') ? { workflow_runs: workflowRuns } : { check_runs: runs };
    return { status: 0, stdout: JSON.stringify(body), stderr: '', args };
  };
}
const run = (name, status, conclusion, started_at, suite = SUITE) => ({ name, status, conclusion, started_at, check_suite: { id: suite } });
const wfRun = (id, name, status, conclusion, check_suite_id, run_started_at, workflow_id = WF) => ({ id, name, workflow_id, status, conclusion, check_suite_id, run_started_at, created_at: run_started_at });

describe('readCheck — the newest run of the named check', () => {
  test('asks the check-runs endpoint for the SHA and returns the newest named run', () => {
    let seen;
    const spawn = (bin, args) => { seen ??= args; return ghReturning([run('Tests pass', 'completed', 'failure', '2026-09-01T00:00:00Z', 1), run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z'), run('lint', 'completed', 'failure', '2026-09-03T00:00:00Z')])(bin, args); };
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn });
    assert.deepEqual(read, { read: 'ok', status: 'completed', conclusion: 'success' });
    assert.deepEqual(seen, ['api', `repos/ethosengine/brit/commits/${SHA}/check-runs?check_name=Tests%20pass&per_page=100`]);
  });
  test('an absent check reads as conclusion "absent"', () => {
    assert.deepEqual(readCheck(attestation, SHA, { ghBin: 'gh', spawn: ghReturning([run('lint', 'completed', 'success', '2026-09-01T00:00:00Z')]) }), { read: 'ok', status: 'absent', conclusion: 'absent' });
  });
  test('a gh failure is read: failed with the stderr as reason', () => {
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn: () => ({ status: 4, stdout: '', stderr: 'gh: not logged in' }) });
    assert.deepEqual(read, { read: 'failed', reason: 'gh: not logged in' });
  });
  test('the forge answering "no such commit" (404/422) is a definitive absent, not an unreachable read', () => {
    for (const code of [404, 422]) {
      const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn: () => ({ status: 1, stdout: '', stderr: `gh: Not Found (HTTP ${code})` }) });
      assert.deepEqual(read, { read: 'ok', status: 'absent', conclusion: 'absent' }, `HTTP ${code}`);
    }
  });
  test('auth, rate-limit, server and network failures stay unreachable reads', () => {
    for (const stderr of ['gh: Forbidden (HTTP 403)', 'gh: rate limit exceeded (HTTP 429)', 'gh: Server Error (HTTP 502)', 'dial tcp: could not resolve host']) {
      const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn: () => ({ status: 1, stdout: '', stderr }) });
      assert.equal(read.read, 'failed', stderr);
    }
  });
  test('the read is bounded by a timeout, and a timed-out read is unreachable, naming the timeout', () => {
    let opts;
    const spawn = (bin, args, o) => { opts = o; return { status: null, stdout: '', stderr: '', error: Object.assign(new Error('spawnSync gh ETIMEDOUT'), { code: 'ETIMEDOUT' }) }; };
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn });
    assert.ok(opts.timeout >= 30000, 'gh spawn carries a timeout');
    assert.equal(read.read, 'failed');
    assert.match(read.reason, /timed out/);
  });
  test('no gh binary is read: failed', () => {
    assert.deepEqual(readCheck(attestation, SHA, { ghBin: null, spawn: () => { throw new Error('must not spawn'); } }), { read: 'failed', reason: 'gh not found' });
  });
});

describe('readCheck and judge — a newer run of the same workflow (backlog row 71, 2026-11-22)', () => {
  const green = run('Tests pass', 'completed', 'success', '2026-11-21T23:23:17Z');
  const judged = wfRun(10, 'ci', 'completed', 'success', SUITE, '2026-11-21T22:06:44Z');
  const verdictFor = (workflowRuns, spawn = ghReturning([green], workflowRuns)) => {
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn });
    return { read, verdict: judge(read, attestation, SHA) };
  };

  test('(a) the judged run alone: pass, the line is today\'s', () => {
    const { read, verdict } = verdictFor([judged]);
    assert.deepEqual(read, { read: 'ok', status: 'completed', conclusion: 'success' });
    assert.equal(verdict.line, 'attested: ethosengine/brit@b86c5104d339 Tests pass success');
  });
  test('the second read asks for the commit\'s workflow runs', () => {
    const calls = [];
    verdictFor([judged], (bin, args, o) => { calls.push({ args, o }); return ghReturning([green], [judged])(bin, args); });
    assert.equal(calls.length, 2);
    assert.deepEqual(calls[1].args, ['api', `repos/ethosengine/brit/actions/runs?head_sha=${SHA}&per_page=100`]);
    assert.ok(calls[1].o.timeout >= 30000);
  });
  test('(b) a newer in-progress run of the same workflow with no named check yet refuses as pending, naming the run', () => {
    const { read, verdict } = verdictFor([judged, wfRun(11, 'ci', 'in_progress', null, 9999, '2026-11-21T23:27:00Z')]);
    assert.notEqual(read.status, 'completed');
    assert.equal(verdict.outcome, 'refuse'); assert.equal(verdict.tier, 'witnessed'); assert.equal(verdict.conclusion, 'pending');
    assert.equal(verdict.line, 'attested: REFUSED — ethosengine/brit@b86c5104d339 Tests pass not yet concluded at b86c5104d339: a newer run of the same workflow (ci, run 11, in_progress) has not produced this check yet');
  });
  test('(c) a newer in-progress run of a different workflow never matters', () => {
    const { verdict } = verdictFor([judged, wfRun(11, 'brit build artifacts', 'in_progress', null, 9999, '2026-11-21T23:27:00Z', WF + 1)]);
    assert.equal(verdict.outcome, 'pass'); assert.equal(verdict.line, 'attested: ethosengine/brit@b86c5104d339 Tests pass success');
  });
  test('(d) a newer queued run of the same workflow refuses', () => {
    const { verdict } = verdictFor([judged, wfRun(12, 'ci', 'queued', null, 9999, '2026-11-21T23:30:00Z')]);
    assert.equal(verdict.outcome, 'refuse'); assert.equal(verdict.conclusion, 'pending');
    assert.match(verdict.line, /\(ci, run 12, queued\)/);
  });
  test('(e) an in-progress run of the same workflow that started earlier never matters', () => {
    const { verdict } = verdictFor([judged, wfRun(13, 'ci', 'in_progress', null, 9999, '2026-11-21T21:00:00Z')]);
    assert.equal(verdict.outcome, 'pass');
  });
  test('a newer completed run of the same workflow never matters here', () => {
    const { verdict } = verdictFor([judged, wfRun(14, 'ci', 'completed', 'failure', 9999, '2026-11-21T23:27:00Z')]);
    assert.equal(verdict.outcome, 'pass');
  });
  test('(f) a failing second read keeps the verdict and says newer runs were not checked', () => {
    const spawn = (bin, args) => String(args[1]).includes('/actions/runs') ? { status: 1, stdout: '', stderr: 'gh: Server Error (HTTP 502)' } : ghReturning([green])(bin, args);
    const { verdict } = verdictFor([], spawn);
    assert.equal(verdict.outcome, 'pass'); assert.equal(verdict.tier, 'witnessed'); assert.equal(verdict.conclusion, 'success');
    assert.equal(verdict.line, 'attested: ethosengine/brit@b86c5104d339 Tests pass success (newer runs not checked: gh: Server Error (HTTP 502))');
  });
  test('a timed-out or unparsable second read is the same', () => {
    const timedOut = (bin, args) => String(args[1]).includes('/actions/runs') ? { status: null, stdout: '', stderr: '', error: Object.assign(new Error('x'), { code: 'ETIMEDOUT' }) } : ghReturning([green])(bin, args);
    assert.match(verdictFor([], timedOut).verdict.line, /success \(newer runs not checked: gh timed out/);
    const junk = (bin, args) => String(args[1]).includes('/actions/runs') ? { status: 0, stdout: 'not json', stderr: '' } : ghReturning([green])(bin, args);
    assert.match(verdictFor([], junk).verdict.line, /success \(newer runs not checked: unparsable/);
  });
  test('(g) no workflow run carries the judged suite: pass with the clause, never a refusal', () => {
    const { verdict } = verdictFor([wfRun(11, 'ci', 'in_progress', null, 9999, '2026-11-21T23:27:00Z')]);
    assert.equal(verdict.outcome, 'pass'); assert.equal(verdict.tier, 'witnessed');
    assert.match(verdict.line, /Tests pass success \(newer runs not checked: .*not in the workflow-run list/);
  });
  test('the clause rides a witnessed red too, and a red stays a refusal', () => {
    const red = run('Tests pass', 'completed', 'failure', '2026-11-21T23:23:17Z');
    const spawn = (bin, args) => String(args[1]).includes('/actions/runs') ? { status: 1, stdout: '', stderr: 'offline' } : ghReturning([red])(bin, args);
    const v = judge(readCheck(attestation, SHA, { ghBin: 'gh', spawn }), attestation, SHA);
    assert.equal(v.outcome, 'refuse'); assert.match(v.line, /newer runs not checked: offline\)$/);
  });
  test('(h) a pending named check refuses as today and the second read is never spawned', () => {
    const calls = [];
    const spawn = (bin, args) => { calls.push(args); return ghReturning([run('Tests pass', 'in_progress', null, '2026-11-21T23:23:17Z')])(bin, args); };
    const { read, verdict } = verdictFor([], spawn);
    assert.equal(calls.length, 1);
    assert.deepEqual(read, { read: 'ok', status: 'in_progress', conclusion: null });
    assert.equal(verdict.line, 'attested: REFUSED — ethosengine/brit@b86c5104d339 Tests pass not yet concluded (in_progress) at b86c5104d339');
  });
  test('an absent check and a failed first read make no second read either', () => {
    for (const spawn of [() => ({ status: 1, stdout: '', stderr: 'offline' }), ghReturning([])]) {
      let n = 0;
      readCheck(attestation, SHA, { ghBin: 'gh', spawn: (...a) => { n++; return spawn(...a); } });
      assert.equal(n, 1);
    }
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

test('judgePin resolves the registered gitlink attestation and reuses the same judge', () => {
  const verdict = judgePin('elohim/brit', SHA, {
    root: new URL('../..', import.meta.url).pathname,
    env: { GH_BIN: process.execPath },
    spawn: ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]),
  });
  assert.equal(verdict.tier, 'witnessed');
  assert.equal(verdict.conclusion, 'success');
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
  function fakeGit({ sha = SHA, dirty = false, gitlink = true, mode = '160000' } = {}) {
    return (bin, args) => {
      if (bin !== 'git') return null;
      if (args.includes('ls-tree')) {
        return gitlink
          ? { status: 0, stdout: `${mode} ${mode === '160000' ? 'commit' : 'tree'} ${sha}\telohim/brit\n`, stderr: '' }
          : { status: 0, stdout: '', stderr: '' };
      }
      if (args.includes('status')) return { status: 0, stdout: dirty ? ' M elohim/brit\n' : '', stderr: '' };
      return { status: 1, stdout: '', stderr: `unexpected git ${args.join(' ')}` };
    };
  }
  function harness(gh, git) {
    const lines = []; const epr = []; const gitCalls = [];
    const spawn = (bin, args, o) => { if (bin === 'git') { gitCalls.push({ args, opts: o }); return git(bin, args, o); } return gh(bin, args, o); };
    return { lines, epr, gitCalls, deps: { root: '/repo', env: { GH_BIN: process.execPath, GIT_DIR: '/repo/.git', GIT_INDEX_FILE: '/repo/.git/index' }, spawn, log: l => lines.push(l), runEpr: a => { epr.push(a); return 0; } } };
  }

  test('the pin is read with ls-tree and must be a gitlink (mode 160000); a tree object is a manifest error', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit({ mode: '040000' }));
    assert.equal(runAttested(project, h.deps), 2);
    assert.match(h.lines[0], /not a gitlink/);
    assert.equal(h.epr.length, 0);
    assert.deepEqual(h.gitCalls[0].args, ['-C', '/repo', 'ls-tree', 'HEAD', '--', 'elohim/brit']);
  });

  test('dirtiness is the superproject’s view of the pin path, asked with the hook’s GIT_* pins scrubbed', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit());
    assert.equal(runAttested(project, h.deps), 0);
    const status = h.gitCalls.find(c => c.args.includes('status'));
    assert.deepEqual(status.args, ['-C', '/repo', 'status', '--porcelain', '--', 'elohim/brit']);
    for (const call of h.gitCalls) {
      assert.equal(call.opts.env.GIT_DIR, undefined, 'GIT_DIR scrubbed');
      assert.equal(call.opts.env.GIT_INDEX_FILE, undefined, 'GIT_INDEX_FILE scrubbed');
    }
  });

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
  test('unreachable → its own non-green exit as claimed', () => {
    const h = harness(() => ({ status: 1, stdout: '', stderr: 'offline' }), fakeGit());
    assert.equal(runAttested(project, h.deps), CLAIMED_EXIT_CODE);
    assert.notEqual(CLAIMED_EXIT_CODE, 0);
    assert.notEqual(CLAIMED_EXIT_CODE, 1);
    assert.notEqual(CLAIMED_EXIT_CODE, 2);
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
