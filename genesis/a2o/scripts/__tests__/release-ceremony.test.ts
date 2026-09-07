/* eslint-disable @typescript-eslint/no-floating-promises -- node:test describe/it
   return promises that the test runner itself consumes; awaiting them is wrong. */
/**
 * T4 (2026-09-08) — "I have adopted" means "I run the target bytes" (D-A).
 *
 * `readAdoptedRelease` / `assertAdmissibleOverEarnedHead` read
 * `GET /admin/adoption`'s `channels[].verdict.runsTarget` /
 * `.alreadyCurrent` — the Rust side's `already_runs_target` exposed on the
 * wire (`elohim-storage/src/services/release_adoption/{state,watch}.rs`).
 * Before T4, an `observe`-mode peer that already ran a release's target
 * coordinator bytes (because the release was cut FOR the fleet, not for
 * this peer's own controller) never recorded `appliedRelease`, so `publish`
 * refused it `not_adopted` forever — the exact wedge
 * `runtime-steward-adoption-of-fleet-cut-release.md` describes and the it8
 * fleet-cut candidate below reproduces.
 *
 * Fixture grounding: `earnedCid` and `channelId` are lifted verbatim from
 * `genesis/a2o/reports/workspace-release/2026-09-06/
 * workspace-release-it8-fleet-cut.json` (gitignored; read from the shared
 * checkout, not the worktree) — `envelope.lineageParentCid` is the standing
 * earned head that manifest was built to publish over, and `channelId` is
 * the real channel the workspace-to-fleet crossing used. Only these two
 * string fields are copied in; no other repo/fleet fact is assumed.
 */
import { strict as assert } from 'node:assert';
import { createServer } from 'node:http';
import { describe, it } from 'node:test';

import {
  ADOPTION_PATH,
  assertAdmissibleOverEarnedHead,
  readAdoptedRelease,
  type Flags,
  type PeerConfig,
} from '../release-ceremony.js';

// From workspace-release-it8-fleet-cut.json (2026-09-06) — see module doc.
const IT8_CHANNEL_ID = 'runtime:coordinators:elohim:workspace';
const IT8_EARNED_CID = 'uhCkkU-TS7pi8E38KYo_p092bJLlObEkVpib4LAnBm3x4THNU0qiH';

const ACTING_PEER: PeerConfig = { name: 'workspace', admin: 0, app: 0 };

/** A manifest whose declared lineage parent is the it8 earned head — the
 * "candidate to publish next" half of `assertAdmissibleOverEarnedHead`'s
 * two-condition admission. Only `envelope.lineageParentCid` is read by the
 * function under test. */
function manifestOver(earnedCid: string) {
  return { envelope: { lineageParentCid: earnedCid } };
}

/** Starts a hermetic `GET /admin/adoption` stand-in serving exactly the body
 * given, and returns its base url + a stop() to close it — same pattern
 * `look.test.ts` uses for its localhost 404 fixture. */
async function withAdoptionServer(
  body: unknown,
  run: (adoptionUrl: string) => Promise<void>
): Promise<void> {
  const server = createServer((req, res) => {
    if (req.url === ADOPTION_PATH) {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify(body));
    } else {
      res.writeHead(404);
      res.end();
    }
  });
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address() as { port: number };
  try {
    await run(`http://127.0.0.1:${port}`);
  } finally {
    await new Promise<void>((resolve, reject) =>
      server.close(err => (err ? reject(err) : resolve()))
    );
  }
}

describe('readAdoptedRelease — runsTarget wire parsing', () => {
  it('reads verdict.runsTarget + verdict.releaseCid off an "ok" row (observe-mode by-bytes exit)', async () => {
    await withAdoptionServer(
      {
        channels: [
          {
            channelId: IT8_CHANNEL_ID,
            appliedRelease: null, // observe mode never records an apply
            resolvedHead: { cid: IT8_EARNED_CID, tier: 'earned' },
            verdict: { state: 'ok', ok: true, releaseCid: IT8_EARNED_CID, runsTarget: true },
          },
        ],
      },
      async adoptionUrl => {
        const result = await readAdoptedRelease(adoptionUrl, IT8_CHANNEL_ID, 5_000);
        assert.equal(result.error, null);
        assert.equal(result.appliedCid, null, 'observe mode never sets appliedRelease');
        assert.equal(result.runsTarget, true);
        assert.equal(result.verdictReleaseCid, IT8_EARNED_CID);
      }
    );
  });

  it('reads verdict.alreadyCurrent off an "applied" row (apply-mode by-bytes exit)', async () => {
    await withAdoptionServer(
      {
        channels: [
          {
            channelId: IT8_CHANNEL_ID,
            appliedRelease: { cid: IT8_EARNED_CID },
            resolvedHead: { cid: IT8_EARNED_CID, tier: 'earned' },
            verdict: {
              state: 'applied',
              ok: true,
              releaseCid: IT8_EARNED_CID,
              vehicle: 'sync_coordinators',
              alreadyCurrent: true,
            },
          },
        ],
      },
      async adoptionUrl => {
        const result = await readAdoptedRelease(adoptionUrl, IT8_CHANNEL_ID, 5_000);
        assert.equal(result.runsTarget, true);
        assert.equal(result.verdictReleaseCid, IT8_EARNED_CID);
      }
    );
  });

  it('reports runsTarget=false and no verdictReleaseCid when the channel has no row yet', async () => {
    await withAdoptionServer({ channels: [] }, async adoptionUrl => {
      const result = await readAdoptedRelease(adoptionUrl, IT8_CHANNEL_ID, 5_000);
      assert.equal(result.error, null);
      assert.equal(result.appliedCid, null);
      assert.equal(result.runsTarget, false);
      assert.equal(result.verdictReleaseCid, null);
    });
  });

  it('never trusts runsTarget=true recorded for a DIFFERENT (stale) release', async () => {
    // The exact false-positive shape a reviewer should look for: a row whose
    // bit is true, but for a release that is not the one being asked about.
    await withAdoptionServer(
      {
        channels: [
          {
            channelId: IT8_CHANNEL_ID,
            appliedRelease: null,
            resolvedHead: { cid: 'uhCkkSomeOlderHead', tier: 'earned' },
            verdict: { state: 'ok', ok: true, releaseCid: 'uhCkkSomeOlderHead', runsTarget: true },
          },
        ],
      },
      async adoptionUrl => {
        const result = await readAdoptedRelease(adoptionUrl, IT8_CHANNEL_ID, 5_000);
        assert.equal(result.runsTarget, true, 'the row itself is honestly reported');
        assert.equal(
          result.verdictReleaseCid,
          'uhCkkSomeOlderHead',
          'the caller must compare this to the cid it cares about — see the next describe block'
        );
      }
    );
  });
});

describe('assertAdmissibleOverEarnedHead — D-A: runsTarget counts as adopted', () => {
  it('ADMITS a publish when appliedRelease is null but runsTarget is true for the earned head (the it8 fleet-cut shape)', async () => {
    await withAdoptionServer(
      {
        channels: [
          {
            channelId: IT8_CHANNEL_ID,
            appliedRelease: null,
            resolvedHead: { cid: IT8_EARNED_CID, tier: 'earned' },
            verdict: { state: 'ok', ok: true, releaseCid: IT8_EARNED_CID, runsTarget: true },
          },
        ],
      },
      async adoptionUrl => {
        const flags: Flags = { 'adoption-url': adoptionUrl };
        // Must not throw: this is the exact fleet-cut-release wedge the
        // backlog atom names — coordinator_lineage_mismatch on the peers
        // this release WASN'T cut for is a separate refusal on a different
        // release; here the peer already runs these bytes.
        await assertAdmissibleOverEarnedHead(
          IT8_CHANNEL_ID,
          IT8_EARNED_CID,
          manifestOver(IT8_EARNED_CID),
          flags,
          [ACTING_PEER],
          ACTING_PEER,
          5_000
        );
      }
    );
  });

  it('still REFUSES not_adopted when runsTarget is true but for a different release than the earned head', async () => {
    await withAdoptionServer(
      {
        channels: [
          {
            channelId: IT8_CHANNEL_ID,
            appliedRelease: null,
            resolvedHead: { cid: 'uhCkkOlderHead', tier: 'earned' },
            verdict: { state: 'ok', ok: true, releaseCid: 'uhCkkOlderHead', runsTarget: true },
          },
        ],
      },
      async adoptionUrl => {
        const flags: Flags = { 'adoption-url': adoptionUrl };
        await assert.rejects(
          async () =>
            assertAdmissibleOverEarnedHead(
              IT8_CHANNEL_ID,
              IT8_EARNED_CID,
              manifestOver(IT8_EARNED_CID),
              flags,
              [ACTING_PEER],
              ACTING_PEER,
              5_000
            ),
          /not_adopted/,
          'a stale runsTarget bit for a different release must never count as adoption of THIS one'
        );
      }
    );
  });

  it('REFUSES not_adopted when neither appliedRelease nor runsTarget hold, and names BOTH facts in the message', async () => {
    await withAdoptionServer(
      {
        channels: [
          {
            channelId: IT8_CHANNEL_ID,
            appliedRelease: null,
            resolvedHead: { cid: IT8_EARNED_CID, tier: 'earned' },
            verdict: { state: 'ok', ok: true, releaseCid: IT8_EARNED_CID, runsTarget: false },
          },
        ],
      },
      async adoptionUrl => {
        const flags: Flags = { 'adoption-url': adoptionUrl };
        await assert.rejects(
          async () =>
            assertAdmissibleOverEarnedHead(
              IT8_CHANNEL_ID,
              IT8_EARNED_CID,
              manifestOver(IT8_EARNED_CID),
              flags,
              [ACTING_PEER],
              ACTING_PEER,
              5_000
            ),
          (err: Error) => {
            assert.match(err.message, /not_adopted/);
            // The controller row (appliedRelease) and the by-bytes bit
            // (runsTarget) must BOTH be visible in the refusal so a reader
            // can tell "never applied" apart from "applied nothing, and
            // does not run the target bytes either" — the task's explicit
            // ask, and the axis an Opus reviewer checks for a
            // false-positive-adoption path.
            assert.match(err.message, /appliedRelease=none/);
            assert.match(err.message, /runsTarget=false/);
            return true;
          }
        );
      }
    );
  });

  it('ADMITS via the legacy appliedRelease.cid path unchanged (T4 is additive, not a replacement)', async () => {
    await withAdoptionServer(
      {
        channels: [
          {
            channelId: IT8_CHANNEL_ID,
            appliedRelease: { cid: IT8_EARNED_CID },
            resolvedHead: { cid: IT8_EARNED_CID, tier: 'earned' },
            verdict: {
              state: 'applied',
              ok: true,
              releaseCid: IT8_EARNED_CID,
              vehicle: 'sync_coordinators',
              alreadyCurrent: false,
            },
          },
        ],
      },
      async adoptionUrl => {
        const flags: Flags = { 'adoption-url': adoptionUrl };
        await assertAdmissibleOverEarnedHead(
          IT8_CHANNEL_ID,
          IT8_EARNED_CID,
          manifestOver(IT8_EARNED_CID),
          flags,
          [ACTING_PEER],
          ACTING_PEER,
          5_000
        );
      }
    );
  });
});
