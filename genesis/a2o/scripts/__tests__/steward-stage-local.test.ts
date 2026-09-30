import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import { buildContentInput } from '../../../seeder/src/content-input.js';
import { runStewardPublish } from '../steward-publish.js';

import type { Conductor, ConductorOptions } from '../lib/steward-conductor.js';
import type { PublicationReceipt } from '../lib/steward-publication-receipt.js';

const REMOTE = 'ws://remote';
const LOCAL = 'https://local';
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
  let patches = 0;
  const profile = {
    adminWs: REMOTE,
    appWs: REMOTE,
    appId: 'app',
    role: 'lamad',
    signingCredentialsDir: dir,
  };
  writeFileSync(
    join(dir, 'grants.json'),
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
  writeFileSync(join(dir, 'grantors.json'), JSON.stringify({ [hash(4)]: profile }));
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
    join(dir, 'grants.json'),
    '--grantor-connections',
    join(dir, 'grantors.json'),
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
            root_action_hash: bytes(3),
            root_author: bytes(4),
            content_id: 'lesson',
            truncated: false,
          } as T;
        if (name === 'resolve_content_head_local') return null as T;
        if (name === 'accept_delegated_head') {
          assert.deepEqual(
            (payload as { head_action_hash: Uint8Array }).head_action_hash,
            bytes(6)
          );
          return (payload as { delegation: unknown }).delegation as T;
        }
        if (name === 'declare_earned_canonical_head') {
          assert.equal((payload as { head_action_hash: string }).head_action_hash, hash(6));
          return {
            head_action_hash: bytes(6),
            author: bytes(1),
            canonical: true,
            canonical_earned: true,
          } as T;
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
    argv,
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
        '--stage-local',
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
      runStewardPublish([...f.argv, '--stage-local'], f.connect),
      /invalid or unavailable proof/
    );
    assert.equal(f.patches(), 0);
    assert.ok(!f.dials.includes(REMOTE));
  });
}
