import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import { captureReceiverOps } from '../lib/steward-op-capture.js';

import type { NativeReceiverProof } from '../lib/steward-native-proof.js';
import type { AdminWebsocket } from '@holochain/client';

const dna = new Uint8Array(39).fill(1);
const agent = new Uint8Array(39).fill(2);
const proof: NativeReceiverProof = {
  agent: encodeHashToBase64(agent),
  dna: encodeHashToBase64(dna),
  target: {
    id: 'lesson',
    head: encodeHashToBase64(agent),
    root: encodeHashToBase64(dna),
    dna: encodeHashToBase64(dna),
    issuanceActionHash: encodeHashToBase64(new Uint8Array(39).fill(8)),
    acceptanceWitnessHash: encodeHashToBase64(new Uint8Array(39).fill(9)),
  },
  accepted: true,
  acceptedAt: 1200,
  deadlineMs: 1234,
  electionLink: encodeHashToBase64(agent),
};
function fixture(wrongCell = false, repeat = false) {
  const requests: { type: string; value: Record<string, unknown> }[] = [];
  let pages = 0;
  const admin = {
    listApps: async () => {
      await Promise.resolve();
      return [
        {
          installed_app_id: 'native',
          cell_info: {
            lamad: [{ type: 'provisioned', value: { cell_id: [dna, wrongCell ? dna : agent] } }],
          },
        },
      ];
    },
    client: {
      url: new URL('ws://localhost:39097'),
      request: async (req: { type: string; value: Record<string, unknown> }) => {
        await Promise.resolve();
        requests.push(req);
        if (req.type === 'dump_full_state') {
          pages++;
          return {
            type: 'full_state_dumped',
            value: {
              source_chain_dump: { secret: 'do-not-persist' },
              integration_dump: {
                integrated: [{ bytes: new Uint8Array([3, 4]) }],
                validation_limbo: [],
                integration_limbo: [],
                dht_ops_cursor: pages === 1 || repeat ? { when_received: 10, hash: agent } : null,
              },
            },
          };
        }
        assert.equal(req.type, 'dump_op_timings');
        return { type: 'op_timings_dumped', value: { timings: [], cursor: null } };
      },
    },
  } as unknown as Pick<AdminWebsocket, 'listApps' | 'client'>;
  return { admin, requests };
}
void test('captures fork pages for exact native cell without source-chain secrets or deadline reset', async t => {
  const directory = mkdtempSync(join(tmpdir(), 'op-capture-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const { admin, requests } = fixture();
  const path = await captureReceiverOps(admin, proof, {
    appId: 'native',
    role: 'lamad',
    directory,
    captureDeadlineMs: Date.now() + 10000,
  });
  const request = JSON.parse(readFileSync(path, 'utf8')) as {
    deadline_micros: number;
    head: number[];
    state_pages: string[];
    issuance_action_hash: number[];
    acceptance_witness_hash: number[];
  };
  assert.equal(request.deadline_micros, 1234000);
  assert.deepEqual(request.head, Array.from(agent));
  assert.deepEqual(request.issuance_action_hash, Array.from(new Uint8Array(39).fill(8)));
  assert.deepEqual(request.acceptance_witness_hash, Array.from(new Uint8Array(39).fill(9)));
  assert.equal(request.state_pages.length, 2);
  const state = readFileSync(request.state_pages[0], 'utf8');
  assert.ok(!state.includes('secret'));
  assert.deepEqual(
    (JSON.parse(state) as { integration_dump: { integrated: { bytes: number[] }[] } })
      .integration_dump.integrated[0].bytes,
    [3, 4]
  );
  assert.deepEqual(
    requests.map(r => r.type),
    ['dump_full_state', 'dump_full_state', 'dump_op_timings']
  );
  assert.ok(requests.every(r => r.value.limit === 256));
  assert.deepEqual(requests[1].value.dht_ops_cursor, { when_received: 10, hash: agent });
});
void test('refuses another hosted cell before any dump', async () => {
  const { admin, requests } = fixture(true);
  await assert.rejects(
    captureReceiverOps(admin, proof, {
      appId: 'native',
      role: 'lamad',
      directory: '/unused',
      captureDeadlineMs: Date.now() + 10000,
    }),
    /exact expected native cell/
  );
  assert.equal(requests.length, 0);
});
void test('repeated cursor cannot silently certify an incomplete capture', async t => {
  const directory = mkdtempSync(join(tmpdir(), 'op-capture-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const { admin } = fixture(false, true);
  await assert.rejects(
    captureReceiverOps(admin, proof, {
      appId: 'native',
      role: 'lamad',
      directory,
      captureDeadlineMs: Date.now() + 10000,
    }),
    /cursor repeated/
  );
});

void test('unselected pooled admin route cannot mix another conductor timing page', async () => {
  const { admin, requests } = fixture();
  admin.client.url = new URL('wss://doorway.example/hc/admin');
  await assert.rejects(
    captureReceiverOps(admin, proof, {
      appId: 'native',
      role: 'lamad',
      directory: '/unused',
      captureDeadlineMs: Date.now() + 10000,
    }),
    /explicit conductor_id/
  );
  assert.equal(requests.length, 0);
});

void test('bounded larger pages retain complete evidence and the original acceptance deadline', async t => {
  for (const capturePageSize of [1024, 4096]) {
    const directory = mkdtempSync(join(tmpdir(), 'op-capture-sized-'));
    t.after(() => rmSync(directory, { recursive: true, force: true }));
    const { admin, requests } = fixture();
    const path = await captureReceiverOps(admin, proof, {
      appId: 'native',
      role: 'lamad',
      directory,
      captureDeadlineMs: Date.now() + 10000,
      capturePageSize,
    });
    const captured = JSON.parse(readFileSync(path, 'utf8')) as {
      content_id: string;
      dna: number[];
      deadline_micros: number;
      state_pages: string[];
      timing_pages: string[];
    };
    assert.equal(captured.content_id, proof.target.id);
    assert.deepEqual(captured.dna, Array.from(dna));
    assert.equal(captured.deadline_micros, proof.deadlineMs * 1000);
    assert.equal(captured.state_pages.length, 2);
    assert.equal(captured.timing_pages.length, 1);
    assert.ok(requests.every(r => r.value.limit === capturePageSize));
    for (const page of captured.state_pages) {
      const bytes = readFileSync(page, 'utf8');
      assert.ok(!bytes.includes('do-not-persist'));
      assert.deepEqual(JSON.parse(bytes), {
        integration_dump: {
          integrated: [{ bytes: [3, 4] }],
          validation_limbo: [],
          integration_limbo: [],
          dht_ops_cursor:
            page === captured.state_pages[0]
              ? { when_received: 10, hash: Array.from(agent) }
              : null,
        },
      });
    }
  }
});
void test('invalid page sizes refuse before any native request', async () => {
  for (const capturePageSize of [0, -1, 1.5, 4097, Number.NaN, Number.POSITIVE_INFINITY]) {
    const { admin, requests } = fixture();
    admin.listApps = () => {
      throw new Error('must refuse before native inspection');
    };
    await assert.rejects(
      captureReceiverOps(admin, proof, {
        appId: 'native',
        role: 'lamad',
        directory: '/unused',
        captureDeadlineMs: Date.now() + 10000,
        capturePageSize,
      }),
      /Invalid capture page size/
    );
    assert.equal(requests.length, 0);
  }
});
void test('larger pages cannot hide exhaustion of the completeness budget', async t => {
  const directory = mkdtempSync(join(tmpdir(), 'op-capture-budget-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const { admin, requests } = fixture();
  await assert.rejects(
    captureReceiverOps(admin, proof, {
      appId: 'native',
      role: 'lamad',
      directory,
      captureDeadlineMs: Date.now() + 10000,
      capturePageSize: 4096,
      maxPages: 1,
    }),
    /Incomplete state capture: page budget exhausted/
  );
  assert.equal(requests.length, 1);
  assert.throws(() => readFileSync(join(directory, 'request.json')));
});
