import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';

import { runEnrichSample } from './enrich-sample.mjs';

const COMMIT = '0123456789abcdef0123456789abcdef01234567';
const JUST_OID = '1'.repeat(40);
const RUNNER_OID = '2'.repeat(40);

function fixture(t, { gateStatus = 0, enrichStatus = 0, fromDevice = null } = {}) {
  const root = mkdtempSync(resolve(tmpdir(), 'enrich-sample-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const calls = [];
  const spawn = (command, args, options = {}) => {
    calls.push({ command, args, options });
    if (command === 'pgrep') return { status: 1 };
    if (command === 'just') return { status: gateStatus };
    if (command === 'fixture-epr') return { status: enrichStatus };
    if (command === 'pnpm') return { status: 0 };
    if (command === 'git' && args.includes('rev-parse')) {
      return { status: 0, stdout: `${args.at(-1).endsWith(':justfile') ? JUST_OID : RUNNER_OID}\n` };
    }
    if (command === 'git') return { status: 0 };
    throw new Error(`unexpected command ${command}`);
  };
  const ticks = [1_000, 5_500];
  return {
    root,
    calls,
    run: () => runEnrichSample({
      project: 'orchestrator', commit: COMMIT, closure: 'closure-a',
      ...(fromDevice ? { from_device: fromDevice } : {}), root,
    }, {
      env: { EPR_BIN: 'fixture-epr' }, spawn, now: () => ticks.shift(), sleep: async () => {},
    }),
  };
}

function enrichCall(calls) {
  return calls.find(call => call.command === 'fixture-epr');
}

describe('detached enrichment sample', () => {
  for (const [gateStatus, outcome] of [[0, 'green'], [3, 'red']]) {
    test(`gate exit ${gateStatus} records ${outcome} with native verdict selection`, async t => {
      const sample = fixture(t, { gateStatus });
      const result = await sample.run();
      assert.equal(result.outcome, outcome);
      assert.equal(result.seconds, '4.5');
      const call = enrichCall(sample.calls);
      assert.ok(call, 'the native enrichment writer is called');
      assert.deepEqual(call.args.slice(0, 7), [
        'flow', 'enrich', '--project', 'orchestrator', '--closure', 'closure-a', '--kind',
      ]);
      assert.equal(call.args[call.args.indexOf('--outcome') + 1], outcome);
      assert.equal(call.args[call.args.indexOf('--method') + 1],
        `justfile:${JUST_OID},gate-runner:${RUNNER_OID}`);
      assert.equal(call.args[call.args.indexOf('--seconds') + 1], '4.5');
      assert.ok(!call.args.includes('--verdict'));
    });
  }

  test('a sibling source is carried to the native selector', async t => {
    const sibling = 'did:key:sibling';
    const sample = fixture(t, { fromDevice: sibling });
    await sample.run();
    const call = enrichCall(sample.calls);
    assert.equal(call.args[call.args.indexOf('--from-device') + 1], sibling);
  });

  test('a missing native verdict refuses enrichment after the sample without affecting cleanup', async t => {
    const sample = fixture(t, { enrichStatus: 3 });
    const result = await sample.run();
    assert.equal(result.outcome, 'green', 'the sampled gate still ran');
    assert.equal(result.enriched, false);
    assert.ok(sample.calls.some(call => call.command === 'just'));
    assert.ok(enrichCall(sample.calls));
    assert.match(readFileSync(result.logPath, 'utf8'), /enrichment refused \(3\)/);
    assert.ok(sample.calls.some(call => call.command === 'git'
      && call.args.includes('worktree') && call.args.includes('remove')));
  });

  test('the sample worktree is removed even when the gate is red', async t => {
    const sample = fixture(t, { gateStatus: 3 });
    await sample.run();
    const remove = sample.calls.find(call => call.command === 'git'
      && call.args.includes('worktree') && call.args.includes('remove'));
    assert.ok(remove);
    assert.ok(remove.args.includes('--force'));
    assert.match(remove.args.at(-1), /\.worktrees\/enrich-sample-0123456789ab-orchestrator$/);
  });
});
