import assert from 'node:assert/strict';

import { After, Before, Given, Status, Then, When } from '@cucumber/cucumber';

import {
  WorkspaceHalf,
  T8_TAG,
  timeoutSeconds,
} from './devices-share-one-berth-workspace.fixture.js';
import { holdWithoutEpr } from './epr-binary.js';

const halves = new WeakMap<object, WorkspaceHalf>();
const HELD =
  'HELD (@requires:shem): A2O_T8_ROLE is unset — run one holder and one contender in their own workspaces';

// Cucumber does not execute hooks in --dry-run. Show the same configuration hold there;
// the skipped dry-run steps themselves prove definition coverage, not a live protocol pass.
if (process.argv.includes('--dry-run') && !process.env.A2O_T8_ROLE) {
  // eslint-disable-next-line no-console -- A held dry-run must be visible like the epr guard.
  console.log(HELD);
}

Before({ tags: T8_TAG }, function (scenario): 'skipped' | undefined {
  if (!process.env.A2O_T8_ROLE) {
    // eslint-disable-next-line no-console -- Same HELD convention as the epr guard.
    console.log(HELD);
    return 'skipped';
  }
  const held = holdWithoutEpr(scenario);
  if (held) return held;
  assert.ok(
    ['holder', 'contender'].includes(process.env.A2O_T8_ROLE),
    'A2O_T8_ROLE must be holder or contender'
  );
  return undefined;
});

function half(world: object): WorkspaceHalf {
  const result = halves.get(world);
  assert.ok(result, 'workspace half has not been initialized');
  return result;
}

Given(
  'device A is the workspace on {string} and device B the workspace on {string}, both devices of {string}',
  function (first: string, second: string, handle: string) {
    assert.deepEqual([first, second, handle], ['ethosengine', 'shem', 'matthew']);
  }
);

Given(
  "device A takes the holder's half and device B the contender's half, each running these steps on its own workspace",
  function () {
    const role = process.env.A2O_T8_ROLE;
    assert.ok(role === 'holder' || role === 'contender');
    halves.set(this, new WorkspaceHalf(role));
  }
);

Given(
  "this workspace's peer daemon has folded a fresh head from the other admitted device over their own link",
  { timeout: timeoutSeconds() * 1000 + 90_000 },
  async function () {
    await half(this).prepare();
  }
);

When(
  "the holder claims the lane named for this run and the contender then claims that lane after folding the holder's lease",
  { timeout: timeoutSeconds() * 5000 + 180_000 },
  async function () {
    const r = half(this).receipt;
    assert.equal(r.resource, `lane:t8-receipt-${r.runId}`);
    await half(this).exchange();
  }
);

Then(
  "the holder's claim exits 0 and the contender's claim exits 3 naming the holder's device and session",
  function () {
    // Each half asserts only its local command. t8-receipt.ts joins both independent witnesses.
    const r = half(this).receipt;
    const phase = r.role === 'holder' ? 'holder-claim' : 'contender-claim';
    assert.equal(r.commands.find(c => c.phase === phase)?.exitCode, r.role === 'holder' ? 0 : 3);
  }
);

Then("the holder releases the lane after seeing the contender's refusal mooring", function () {
  const r = half(this).receipt;
  if (r.role === 'holder') {
    assert.ok(r.folds.some(f => f.phase === 'refusal-crossed'));
    assert.ok(r.release && r.folds.some(f => f.phase === 'release-crossed'));
  } else {
    assert.ok(r.folds.some(f => f.phase === 'holder-released'));
  }
});

Then("the contender's fresh claim after that release exits 0 and is released", function () {
  const r = half(this).receipt;
  if (r.role === 'contender') {
    assert.equal(r.commands.find(c => c.phase === 'contender-post-release-claim')?.exitCode, 0);
    assert.ok(r.release && r.folds.some(f => f.phase === 'release-crossed'));
  } else {
    assert.ok(r.folds.some(f => f.phase === 'contender-released'));
  }
});

Then(
  "the berth status each half captured while the other's lease was held lists the other's moorings and leases, read from its own fold",
  function () {
    // Each half captured its own local status while the other lease was live.
    // The joiner requires both outputs and their corresponding folded heads.
    const r = half(this).receipt;
    assert.ok(r.commands.some(c => c.phase === 'berth-status-peer' && c.exitCode === 0));
    assert.ok(r.folds.some(f => f.phase === 'peer-status-visible'));
  }
);

Then(
  "any relay on the path between the two peer daemons saw only encrypted traffic, inferred from the daemon's iroh QUIC guarantee and recorded with the declared relay URL in the receipt",
  function () {
    // eprfs-peer binds iroh::Endpoint: QUIC encryption is end to end, including relayed paths.
    // This records the transport contract and our declared relay, not packet inspection or
    // proof that this exchange traversed the relay rather than a direct path.
    const r = half(this).receipt;
    assert.equal(r.env.relay, 'https://relay.alpha.elohim.host');
    assert.equal(r.env.transport, 'iroh QUIC');
    assert.ok(r.env.encryptionEvidence);
  }
);

Then(
  "this workspace's receipt records that every head and acknowledgement from the other device arrived through the peer daemon's link, and lists only the peer link as a coordination channel",
  function () {
    const r = half(this).receipt;
    assert.deepEqual(r.env.coordinationChannels, ['peer link']);
    assert.ok(r.folds.length > 0 && r.folds.every(f => f.peer.via === 'peer link'));
    assert.ok(
      r.folds.some(f => f.peer.seq > 0 && f.peer.root && f.peer.ackVector[r.device.did] > 0)
    );
    // The only git command records checkout provenance. Neither receipt nor remote commands
    // participate in the protocol; only the steward channel moves the moorings and lease records.
    assert.ok(
      r.commands
        .filter(c => c.argv[0] === 'git')
        .every(c => c.argv.join(' ') === 'git rev-parse HEAD')
    );
  }
);

After({ tags: T8_TAG, timeout: 120_000 }, async function (scenario) {
  const run = halves.get(this);
  if (!run) return;
  const passed = scenario.result?.status === Status.PASSED;
  await run.finish(
    passed,
    passed
      ? 'This workspace half completed; join both receipts for the T8 verdict.'
      : (scenario.result?.message ?? 'Workspace half did not complete.')
  );
});
