import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import { buildContentInput } from '../../../seeder/src/content-input.js';
import { runStewardPublish } from '../steward-publish.js';

import type { Conductor, ConductorOptions } from '../lib/steward-conductor.js';
import type { HeadDelegationWire } from '../lib/steward-delegation.js';
import type { PublicationReceipt } from '../lib/steward-publication-receipt.js';

const REMOTE = 'ws://remote';
const LOCAL = 'https://local';
const GRANTS_FILE = 'grants.json';
const GRANTORS_FILE = 'grantors.json';
const STAGE_LOCAL = '--stage-local';
const bytes = (n: number): Uint8Array => new Uint8Array(39).fill(n);
const hash = (n: number): string => encodeHashToBase64(bytes(n));
function fixture() {
  const dir = mkdtempSync(join(tmpdir(), 'steward-stage-'));
  mkdirSync(join(dir, 'content'));
  const item = {
    id: 'lesson',
    title: 'New title',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: 'body',
    reach: 'commons',
    tags: [],
  };
  writeFileSync(join(dir, 'content/lesson.json'), JSON.stringify(item));
  const input = buildContentInput(item as Parameters<typeof buildContentInput>[0]);
  let row: Record<string, unknown> = {
    ...input,
    title: 'Old title',
    metadata: {},
    dhtAnchorHash: hash(3),
  };
  const calls: string[] = [];
  const dials: string[] = [];
  let refuse: string | undefined;
  let offline = true;
  let expired = false;
  let nativeAcceptance: HeadDelegationWire | undefined;
  let loseAcceptanceResponse = false;
  let patches = 0;
  let delayedAuthorHistory = false;
  const profile = {
    adminWs: REMOTE,
    appWs: REMOTE,
    appId: 'app',
    role: 'lamad',
    signingCredentialsDir: dir,
  };
  writeFileSync(
    join(dir, GRANTS_FILE),
    JSON.stringify({
      lesson: {
        grantor: hash(4),
        delegate: hash(1),
        scope: 'lesson',
        validUntil: Date.now() * 1000 + 1e9,
        rootActionHash: hash(3),
        dnaHash: hash(2),
        signature: Buffer.alloc(64).toString('base64'),
      },
    })
  );
  writeFileSync(join(dir, GRANTORS_FILE), JSON.stringify({ [hash(4)]: profile }));
  const argv = [
    'lesson',
    '--data-dir',
    dir,
    '--storage',
    LOCAL,
    '--admin-ws',
    'ws://local',
    '--app-ws',
    'ws://local',
    '--device-agent',
    hash(1),
    '--dna-hash',
    hash(2),
    '--binding',
    hash(5),
    '--receipt-dir',
    join(dir, 'receipts'),
    '--delegations',
    join(dir, GRANTS_FILE),
    '--grantor-connections',
    join(dir, GRANTORS_FILE),
  ];
  const connect = async (options: ConductorOptions): Promise<Conductor> => {
    await Promise.resolve();
    dials.push(options.adminWs);
    if (options.adminWs === REMOTE && offline) throw new Error('remote offline');
    return {
      agent: options.expectedAgent ?? hash(1),
      dna: hash(2),
      close: async () => {
        await Promise.resolve();
      },
      call: async <T>(name: string, payload: unknown): Promise<T> => {
        await Promise.resolve();
        calls.push(name);
        if (name === refuse) throw new Error(`${name}: invalid or unavailable proof`);
        if (name === 'get_content_lineage')
          return {
            referenced_action_hash: (payload as { action_hash: Uint8Array }).action_hash,
            root_action_hash:
              encodeHashToBase64((payload as { action_hash: Uint8Array }).action_hash) === hash(9)
                ? bytes(9)
                : bytes(3),
            root_author: bytes(4),
            content_id: 'lesson',
            truncated: false,
          } as T;
        if (name === 'resolve_content_head_local') return null as T;
        if (name === 'get_accepted_delegated_head') return (nativeAcceptance ?? null) as T;
        if (name === 'accept_delegated_head') {
          assert.deepEqual(
            (payload as { head_action_hash: Uint8Array }).head_action_hash,
            bytes(6)
          );
          if (nativeAcceptance) return nativeAcceptance as T;
          if (expired) throw new Error('expired for new acceptance');
          nativeAcceptance = {
            ...(payload as { delegation: HeadDelegationWire }).delegation,
            acceptance: {
              head_action_hash: bytes(6),
              witness_action_hash: bytes(8),
              accepted_at: Date.now() * 1000,
              signature: new Uint8Array(64),
            },
          };
          if (loseAcceptanceResponse)
            throw new Error('acceptance response lost after native witness');
          return nativeAcceptance as T;
        }
        if (name === 'declare_earned_canonical_head') {
          if (delayedAuthorHistory) {
            delayedAuthorHistory = false;
            throw new Error('acceptance author history not retrievable — PENDING');
          }
          assert.equal((payload as { head_action_hash: string }).head_action_hash, hash(6));
          return {
            head_action_hash: bytes(6),
            author: bytes(1),
            canonical: true,
            canonical_earned: true,
          } as T;
        }
        if (name === 'preflight_head_publication') {
          const input = payload as {
            accepted_head: Uint8Array | null;
            delegation: HeadDelegationWire;
          };
          if (input.accepted_head) {
            assert.deepEqual(input.accepted_head, bytes(6));
            assert.deepEqual(input.delegation.acceptance?.head_action_hash, bytes(6));
          } else if (expired) throw new Error('expired for new publication');
        }
        assert.ok(['verify_device_binding', 'preflight_head_publication'].includes(name));
        return {} as T;
      },
    };
  };
  const fetcher: typeof fetch = async (url, init) => {
    await Promise.resolve();
    assert.ok(String(url).startsWith(`${LOCAL}/`), 'no remote HTTP');
    if (String(url).endsWith('/health'))
      return Response.json({
        conductor: { zomePath: 'live' },
        dhtParticipation: { dnaHashes: { lamad: hash(2) }, agentKeys: { lamad: hash(1) } },
      });
    if (String(url).endsWith('/head')) return new Response('', { status: 404 });
    if (init?.method === 'PATCH') {
      patches++;
      row = {
        ...row,
        ...(JSON.parse(String(init.body)) as Record<string, unknown>),
        dhtAnchorHash: hash(6),
      };
    }
    return Response.json(row);
  };
  return {
    dir,
    competingRow: () => {
      row.dhtAnchorHash = hash(9);
    },
    argv,
    delayAuthorHistory: () => {
      delayedAuthorHistory = true;
    },
    connect,
    fetcher,
    calls,
    dials,
    patches: () => patches,
    refuse: (name: string | undefined) => {
      refuse = name;
    },
    online: () => {
      offline = false;
    },
    expire: () => {
      expired = true;
    },
    loseAcceptanceResponse: () => {
      loseAcceptanceResponse = true;
    },
    expireAndDisconnect: () => {
      expired = true;
      offline = true;
      writeFileSync(join(dir, GRANTORS_FILE), '{}');
      writeFileSync(join(dir, GRANTS_FILE), '{}');
    },
    receipt: () =>
      JSON.parse(
        readFileSync(join(dir, 'receipts', readdirSync(join(dir, 'receipts'))[0]), 'utf8')
      ) as PublicationReceipt,
    cleanup: () => rmSync(dir, { recursive: true, force: true }),
  };
}
void test('offline stage saves pending exact head and connected resume declares without rewriting', async t => {
  const f = fixture();
  t.after(f.cleanup);
  t.mock.method(globalThis, 'fetch', f.fetcher);
  assert.equal(
    await runStewardPublish(
      [
        ...f.argv,
        STAGE_LOCAL,
        '--native-receivers',
        join(f.dir, 'missing.json'),
        '--await-peer',
        'https://offline',
      ],
      f.connect
    ),
    1
  );
  assert.equal(f.patches(), 1);
  assert.equal(f.receipt().head, hash(6));
  assert.equal(f.receipt().declaredAt, undefined);
  assert.ok(f.calls.includes('verify_device_binding'));
  assert.ok(f.calls.includes('preflight_head_publication'));
  assert.ok(!f.dials.includes(REMOTE));
  assert.ok(!f.calls.includes('accept_delegated_head'));
  assert.ok(!f.calls.includes('declare_earned_canonical_head'));
  f.online();
  f.refuse('preflight_head_publication');
  await assert.rejects(runStewardPublish(f.argv, f.connect), /invalid or unavailable proof/);
  assert.equal(f.receipt().declaredAt, undefined);
  assert.equal(f.patches(), 1);
  f.refuse(undefined);
  assert.equal(await runStewardPublish(f.argv, f.connect), 0);
  assert.equal(f.patches(), 1);
  assert.equal(f.receipt().head, hash(6));
  assert.ok(f.receipt().declaredAt);
  assert.equal(f.calls.filter(x => x === 'preflight_head_publication').length, 3);
});
for (const proof of [
  'verify_device_binding',
  'get_content_lineage',
  'preflight_head_publication',
]) {
  void test(`stage-local refuses ${proof} failure before writes`, async t => {
    const f = fixture();
    t.after(f.cleanup);
    t.mock.method(globalThis, 'fetch', f.fetcher);
    f.refuse(proof);
    await assert.rejects(
      runStewardPublish([...f.argv, STAGE_LOCAL], f.connect),
      /invalid or unavailable proof/
    );
    assert.equal(f.patches(), 0);
    assert.ok(!f.dials.includes(REMOTE));
  });
}

void test('accepted exact head survives interrupted declaration then expiry without fresh acceptance', async t => {
  const f = fixture();
  t.after(f.cleanup);
  t.mock.method(globalThis, 'fetch', f.fetcher);
  f.online();
  f.refuse('declare_earned_canonical_head');
  assert.equal(await runStewardPublish(f.argv, f.connect), 1);
  const receipt = f.receipt();
  assert.equal(receipt.acceptedDelegation?.acceptance?.headActionHash, receipt.head);
  assert.equal(receipt.declaredAt, undefined);
  assert.equal(f.patches(), 1);
  f.expireAndDisconnect();
  f.refuse(undefined);
  const priorDials = f.dials.length;
  const acceptances = f.calls.filter(x => x === 'accept_delegated_head').length;
  assert.equal(await runStewardPublish(f.argv, f.connect), 0);
  assert.equal(f.patches(), 1);
  assert.equal(f.calls.filter(x => x === 'accept_delegated_head').length, acceptances);
  assert.ok(!f.dials.slice(priorDials).includes(REMOTE));
  assert.deepEqual(f.receipt().acceptedDelegation, receipt.acceptedDelegation);
  assert.ok(f.receipt().declaredAt);
});

for (const refusal of ['verify_device_binding', 'preflight_head_publication']) {
  void test(`saved acceptance still requires native ${refusal}`, async t => {
    const f = fixture();
    t.after(f.cleanup);
    t.mock.method(globalThis, 'fetch', f.fetcher);
    f.online();
    f.refuse('declare_earned_canonical_head');
    assert.equal(await runStewardPublish(f.argv, f.connect), 1);
    f.expireAndDisconnect();
    f.refuse(refusal);
    const declarations = f.calls.filter(x => x === 'declare_earned_canonical_head').length;
    await assert.rejects(runStewardPublish(f.argv, f.connect), /invalid or unavailable proof/);
    assert.equal(f.patches(), 1);
    assert.equal(f.calls.filter(x => x === 'declare_earned_canonical_head').length, declarations);
    assert.equal(f.receipt().declaredAt, undefined);
  });
}

void test('unaccepted pending head refuses expired grant without another PATCH', async t => {
  const f = fixture();
  t.after(f.cleanup);
  t.mock.method(globalThis, 'fetch', f.fetcher);
  assert.equal(await runStewardPublish([...f.argv, STAGE_LOCAL], f.connect), 1);
  assert.equal(f.receipt().acceptedDelegation, undefined);
  f.expire();
  f.online();
  await assert.rejects(runStewardPublish(f.argv, f.connect), /expired for new publication/);
  assert.equal(f.patches(), 1);
  assert.equal(f.receipt().declaredAt, undefined);
});

void test('receipt acceptance cannot be replayed onto a different pending head', async t => {
  const f = fixture();
  t.after(f.cleanup);
  t.mock.method(globalThis, 'fetch', f.fetcher);
  f.online();
  f.refuse('declare_earned_canonical_head');
  assert.equal(await runStewardPublish(f.argv, f.connect), 1);
  const receipt = f.receipt();
  receipt.head = hash(7);
  const path = join(f.dir, 'receipts', readdirSync(join(f.dir, 'receipts'))[0]);
  writeFileSync(path, JSON.stringify(receipt));
  f.refuse(undefined);
  await assert.rejects(runStewardPublish(f.argv, f.connect), /acceptance names another head/);
  assert.equal(f.patches(), 1);
});

void test('lost native acceptance response recovers the same witness after expiry without a new approval', async t => {
  const f = fixture();
  t.after(f.cleanup);
  t.mock.method(globalThis, 'fetch', f.fetcher);
  f.online();
  f.loseAcceptanceResponse();
  assert.equal(await runStewardPublish(f.argv, f.connect), 1);
  assert.equal(f.receipt().acceptedDelegation, undefined);
  assert.ok(f.receipt().delegation, 'original full grant persists before requesting acceptance');
  const original = f.receipt().delegation;
  writeFileSync(join(f.dir, GRANTS_FILE), '{}');
  f.expire();
  assert.equal(await runStewardPublish(f.argv, f.connect), 0);
  assert.equal(f.patches(), 1);
  assert.equal(f.calls.filter(x => x === 'accept_delegated_head').length, 1);
  assert.equal(f.calls.filter(x => x === 'get_accepted_delegated_head').length, 1);
  assert.deepEqual(f.receipt().delegation, original);
  assert.equal(f.receipt().acceptedDelegation?.acceptance?.witnessActionHash, hash(8));
  assert.ok(f.receipt().declaredAt);
});

void test('delayed native author history retries only the exact accepted version', async t => {
  const f = fixture();
  t.after(f.cleanup);
  t.mock.method(globalThis, 'fetch', f.fetcher);
  f.online();
  f.delayAuthorHistory();
  assert.equal(await runStewardPublish(f.argv, f.connect), 0);
  assert.equal(f.patches(), 1);
  assert.equal(f.calls.filter(call => call === 'accept_delegated_head').length, 1);
  assert.equal(f.calls.filter(call => call === 'declare_earned_canonical_head').length, 2);
  assert.equal(f.receipt().acceptedDelegation?.acceptance?.witnessActionHash, hash(8));
  assert.ok(f.receipt().declaredAt);
});

void test('signed canonical grant root overrides a competing HTTP staging lineage before writes', async t => {
  const f = fixture();
  t.after(f.cleanup);
  f.competingRow();
  const previous = globalThis.fetch;
  globalThis.fetch = f.fetcher;
  t.after(() => {
    globalThis.fetch = previous;
  });
  assert.equal(await runStewardPublish([...f.argv, STAGE_LOCAL], f.connect), 1);
  assert.equal(f.patches(), 1);
  assert.equal(f.receipt().delegation?.rootActionHash, hash(3));
});

void test('canonical grant cannot reparent a previously authored pending head from another root', async t => {
  const f = fixture();
  t.after(f.cleanup);
  const previous = globalThis.fetch;
  globalThis.fetch = f.fetcher;
  t.after(() => {
    globalThis.fetch = previous;
  });
  assert.equal(await runStewardPublish([...f.argv, STAGE_LOCAL], f.connect), 1);
  const receiptPath = join(f.dir, 'receipts', readdirSync(join(f.dir, 'receipts'))[0]);
  const pending = JSON.parse(readFileSync(receiptPath, 'utf8')) as PublicationReceipt;
  pending.head = hash(9);
  writeFileSync(receiptPath, JSON.stringify(pending));
  await assert.rejects(
    runStewardPublish([...f.argv, STAGE_LOCAL], f.connect),
    /pending head is outside canonical root/
  );
  assert.equal(f.patches(), 1, 'no rewrite or reparent of existing pending version');
});
