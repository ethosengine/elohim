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
import { execFile } from 'node:child_process';
import { createServer } from 'node:http';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import {
  ADOPTION_PATH,
  assertAdmissibleOverEarnedHead,
  channelRootRecord,
  doorwayBindPatch,
  doorwayPublishRefusal,
  doorwayTransportRefusal,
  mergeReleaseBinding,
  readAdoptedRelease,
  RELEASE_CHANNEL_KEY,
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

// ---------------------------------------------------------------------------
// Slice 2 (native-delivery Lane N) — channel bind and the doorway transport
// ---------------------------------------------------------------------------

const ALPHA_DEV_CHANNEL = 'runtime:app-bundle:alpha:dev';
const LANDING = 'elohim-host-landing';
const TRANSPORT_FLAG = '--transport';

describe('channel bind — the slug elects itself by its release channel', () => {
  const CHANNEL = ALPHA_DEV_CHANNEL;

  it('sets the binding and carries every other key unchanged', () => {
    const merged = JSON.parse(
      mergeReleaseBinding(JSON.stringify({ serverBlobHash: 'sha256-s', title: 'x' }), CHANNEL)
    );
    assert.deepEqual(merged, {
      serverBlobHash: 'sha256-s',
      title: 'x',
      [RELEASE_CHANNEL_KEY]: CHANNEL,
    });
  });

  it('unbind removes only the binding', () => {
    const merged = JSON.parse(
      mergeReleaseBinding(
        JSON.stringify({ serverBlobHash: 'sha256-s', releaseChannel: CHANNEL }),
        null
      )
    );
    assert.deepEqual(merged, { serverBlobHash: 'sha256-s' });
  });

  it('binds an empty metadata, and refuses a non-channel id or non-object metadata', () => {
    assert.deepEqual(JSON.parse(mergeReleaseBinding(null, CHANNEL)), { releaseChannel: CHANNEL });
    assert.throws(() => mergeReleaseBinding('{}', 'app-bundle-dev'));
    assert.throws(() => mergeReleaseBinding('[1,2]', CHANNEL));
  });
});

describe('publish --transport doorway — the lineage pre-flight', () => {
  const HEAD = 'uhCkkCurrentHead';
  it('admits a first release, and a candidate naming the current head or staged candidate', () => {
    assert.equal(doorwayPublishRefusal(null, { envelope: { lineageParentCid: null } }), '');
    assert.equal(
      doorwayPublishRefusal({ headActionHash: HEAD }, { envelope: { lineageParentCid: null } }),
      ''
    );
    assert.equal(
      doorwayPublishRefusal({ headActionHash: HEAD }, { envelope: { lineageParentCid: HEAD } }),
      ''
    );
    assert.equal(
      doorwayPublishRefusal(
        { headActionHash: HEAD, stagingCandidate: 'uhCkkStaged' },
        { envelope: { lineageParentCid: 'uhCkkStaged' } }
      ),
      ''
    );
  });

  it('refuses a candidate that builds on a release the channel no longer stands on', () => {
    const refusal = doorwayPublishRefusal(
      { headActionHash: HEAD },
      { envelope: { lineageParentCid: 'uhCkkSomethingElse' } }
    );
    assert.ok(refusal.startsWith('lineage_parent_mismatch'), refusal);
  });
});

// ---------------------------------------------------------------------------
// channel create / bind --transport doorway (native-delivery N6 fleet leg): the
// one-time steward acts CI carries on an [app:channel-create] push.
// ---------------------------------------------------------------------------

describe('--transport doorway — which verbs carry it', () => {
  it('admits the verbs with a doorway path, and the default transport', () => {
    for (const [verb, sub] of [
      ['channel', 'create'],
      ['channel', 'bind'],
      ['channel', 'unbind'],
      ['publish', undefined],
    ] as const) {
      assert.equal(doorwayTransportRefusal(verb, sub, 'doorway'), '');
      assert.equal(doorwayTransportRefusal(verb, sub, 'admin-ws'), '');
      assert.equal(doorwayTransportRefusal(verb, sub, undefined), '');
    }
  });

  it('refuses a verb with no doorway path instead of falling back to a conductor admin port', () => {
    for (const verb of ['promote', 'revert', 'status', 'attestations']) {
      const refusal = doorwayTransportRefusal(verb, undefined, 'doorway');
      assert.ok(refusal.startsWith(`${verb} has no doorway transport`), refusal);
    }
    assert.match(
      doorwayTransportRefusal('channel', 'create', 'carrier-pigeon'),
      /expects admin-ws or doorway/
    );
  });
});

describe('channel bind --transport doorway — the PATCH body', () => {
  const CHANNEL = ALPHA_DEV_CHANNEL;

  it("binds with the slug's own reach riding along (the notarizing PATCH)", () => {
    assert.deepEqual(
      doorwayBindPatch({ reach: 'public', metadata: { serverBlobHash: 's' } }, CHANNEL),
      {
        reach: 'public',
        metadata: { [RELEASE_CHANNEL_KEY]: CHANNEL },
      }
    );
    assert.deepEqual(doorwayBindPatch({ metadata: null }, CHANNEL), {
      reach: 'commons',
      metadata: { [RELEASE_CHANNEL_KEY]: CHANNEL },
    });
  });

  it('is idempotent: an already-bound slug needs no write, an unbound slug no unbind', () => {
    assert.equal(
      doorwayBindPatch({ reach: 'commons', metadata: { releaseChannel: CHANNEL } }, CHANNEL),
      null
    );
    assert.equal(doorwayBindPatch({ reach: 'commons', metadata: {} }, null), null);
  });

  it('unbinds by writing the key null (the storage merges metadata key by key), and refuses a non-channel id', () => {
    assert.deepEqual(
      doorwayBindPatch({ reach: 'commons', metadata: { releaseChannel: CHANNEL } }, null),
      {
        reach: 'commons',
        metadata: { [RELEASE_CHANNEL_KEY]: null },
      }
    );
    assert.throws(() => doorwayBindPatch({ metadata: {} }, 'app-bundle-dev'));
  });
});

/** A fake doorway: records every request, answers from a small content table. */
function fakeDoorway(rows: Record<string, Record<string, unknown>>, patchStatus = 200) {
  const requests: { method: string; url: string; key: string; body: string }[] = [];
  const server = createServer((req, res) => {
    let body = '';
    req.on('data', chunk => (body += chunk));
    req.on('end', () => {
      const url = decodeURIComponent(req.url ?? '');
      requests.push({
        method: req.method ?? '',
        url,
        key: String(req.headers['x-api-key'] ?? ''),
        body,
      });
      const reply = (status: number, value: unknown) => {
        res.writeHead(status, { 'content-type': 'application/json' });
        res.end(JSON.stringify(value));
      };
      if (req.method === 'POST' && url === '/db/content/bulk') {
        for (const row of JSON.parse(body) as Record<string, unknown>[]) rows[String(row.id)] = row;
        return reply(200, { created: 1 });
      }
      const m = /^\/db\/content\/([^/]+)(\/head|\/canonical-head)?$/.exec(url);
      if (!m || !rows[m[1]]) return reply(404, { error: 'not found' });
      if (m[2] === '/head') return reply(200, { headActionHash: `uhCkkHead-${m[1]}` });
      if (m[2] === '/canonical-head') return reply(200, { canonical: true });
      if (req.method === 'PATCH' && patchStatus >= 300) {
        return reply(patchStatus, { error: 'Conductor bridge unavailable' });
      }
      if (req.method === 'PATCH') {
        const patch = JSON.parse(body) as { metadata?: Record<string, unknown> };
        rows[m[1]].metadata = { ...(rows[m[1]].metadata as object), ...(patch.metadata ?? {}) };
      }
      return reply(200, rows[m[1]]);
    });
  });
  return { server, requests };
}

const CEREMONY = fileURLToPath(new URL('../release-ceremony.ts', import.meta.url));
const TSX = fileURLToPath(new URL('../../node_modules/.bin/tsx', import.meta.url));

function ceremony(args: string[]): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise(resolve => {
    execFile(
      TSX,
      [CEREMONY, ...args],
      { env: { ...process.env, STORAGE_API_KEY_ADMIN: 'k-admin' } },
      (err, stdout, stderr) => {
        resolve({ code: err ? Number((err as { code?: number }).code ?? 1) : 0, stdout, stderr });
      }
    );
  });
}

describe('channel create / bind over the doorway — the writes, end to end', () => {
  const CHANNEL = ALPHA_DEV_CHANNEL;

  it('create POSTs the channel root, notarizes it with a reach PATCH, then bind PATCHes and declares staging', async () => {
    const rows: Record<string, Record<string, unknown>> = {
      [LANDING]: {
        id: LANDING,
        reach: 'commons',
        metadata: { serverBlobHash: 's' },
      },
    };
    const { server, requests } = fakeDoorway(rows);
    await new Promise<void>(r => server.listen(0, '127.0.0.1', () => r()));
    const doorway = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
    try {
      const created = await ceremony([
        'channel',
        'create',
        CHANNEL,
        TRANSPORT_FLAG,
        'doorway',
        '--doorway',
        doorway,
      ]);
      assert.equal(created.code, 0, created.stderr);
      const writes = requests.filter(r => r.method !== 'GET');
      assert.deepEqual(
        writes.map(r => `${r.method} ${r.url}`),
        ['POST /db/content/bulk', `PATCH /db/content/${CHANNEL}`]
      );
      const [root] = JSON.parse(writes[0].body);
      assert.deepEqual(root, {
        ...channelRootRecord(CHANNEL, 'commons', {}, root.metadata.createdAt),
      });
      assert.deepEqual(JSON.parse(writes[1].body), { reach: 'commons' });
      assert.ok(
        requests.every(r => r.key === 'k-admin'),
        'every request carries STORAGE_API_KEY_ADMIN'
      );

      requests.length = 0;
      const bound = await ceremony([
        'channel',
        'bind',
        LANDING,
        CHANNEL,
        TRANSPORT_FLAG,
        'doorway',
        '--doorway',
        doorway,
      ]);
      assert.equal(bound.code, 0, bound.stderr);
      assert.deepEqual(
        requests.filter(r => r.method !== 'GET').map(r => `${r.method} ${r.url}`),
        [
          'PATCH /db/content/elohim-host-landing',
          'POST /db/content/elohim-host-landing/canonical-head',
        ]
      );
      assert.equal(JSON.parse(bound.stdout).tier, 'staging');
      assert.equal(
        rows[LANDING].metadata &&
          (rows[LANDING].metadata as Record<string, unknown>).serverBlobHash,
        's'
      );

      requests.length = 0;
      const again = await ceremony([
        'channel',
        'bind',
        LANDING,
        CHANNEL,
        TRANSPORT_FLAG,
        'doorway',
        '--doorway',
        doorway,
      ]);
      assert.equal(again.code, 0, again.stderr);
      assert.equal(JSON.parse(again.stdout).alreadyBound, true);
      assert.ok(
        requests.every(r => r.method === 'GET'),
        'an already-bound slug is not re-written'
      );
    } finally {
      server.close();
    }
  });

  it('a doorway 5xx on the bind PATCH exits non-zero, names the reason last, and declares no head', async () => {
    const rows: Record<string, Record<string, unknown>> = {
      [LANDING]: { id: LANDING, reach: 'commons', metadata: {} },
    };
    const { server, requests } = fakeDoorway(rows, 503);
    await new Promise<void>(r => server.listen(0, '127.0.0.1', () => r()));
    const doorway = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
    try {
      const bound = await ceremony([
        'channel',
        'bind',
        LANDING,
        CHANNEL,
        TRANSPORT_FLAG,
        'doorway',
        '--doorway',
        doorway,
      ]);
      assert.notEqual(bound.code, 0);
      assert.ok(
        !requests.some(r => r.url.endsWith('/canonical-head')),
        'no canonical-head is declared after a refused PATCH'
      );
      const last = bound.stderr.trim().split('\n').pop() ?? '';
      assert.match(last, /^release-ceremony: channel bind: PATCH .* returned 503/);
    } finally {
      server.close();
    }
  });

  it('a verb with no doorway path refuses --transport doorway with exit 2 and one line', async () => {
    const refused = await ceremony([
      'promote',
      CHANNEL,
      'uhCkkX',
      TRANSPORT_FLAG,
      'doorway',
      '--doorway',
      'http://127.0.0.1:9',
    ]);
    assert.equal(refused.code, 2);
    assert.equal(refused.stderr.trim().split('\n').length, 1, refused.stderr);
    assert.match(refused.stderr, /promote has no doorway transport/);
  });
});
