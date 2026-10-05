import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import {
  scopedCapability,
  writeCustodyJson,
  MANDATE_TAG,
  type InvocationMandate,
} from '../lib/steward-credential.js';
import { main } from '../steward-credential.js';

import type { connectConductor } from '../lib/steward-conductor.js';

const hash = (prefix: number, value: number) =>
  encodeHashToBase64(new Uint8Array([132, prefix, 36, ...new Uint8Array(36).fill(value)]));
const owner = hash(32, 1),
  requester = hash(32, 2),
  device = hash(32, 3);
const dna = hash(45, 4),
  root = hash(41, 5),
  binding = hash(41, 6);
const LESSON = 'fct-lesson';
function mandate(): InvocationMandate {
  return {
    issuer: owner,
    requester,
    dna,
    delegate: device,
    subjects: [{ id: LESSON, root }],
    operations: ['grant_head_delegation', 'accept_delegated_head', 'get_accepted_delegated_head'],
    valid_until: 100,
    binding,
    policy: 'fct-commons-v1',
    exact_payload_json: null,
  };
}

void test('the issuer credential is assigned, listed and carries exact payload bounds with no secret in its tag', () => {
  const secret = new Uint8Array(64).fill(7);
  const cap = scopedCapability(mandate(), 'content_store', secret, 99);
  assert.equal(cap.access.type, 'assigned');
  assert.equal(cap.functions.type, 'listed');
  assert.deepEqual(JSON.parse(cap.tag.slice(MANDATE_TAG.length)), mandate());
  assert.equal(cap.tag.includes(Buffer.from(secret).toString('hex')), false);
  if (cap.functions.type === 'listed') {
    assert.equal(
      cap.functions.value.some(([, fn]) => fn === 'revoke_identity_device'),
      false
    );
    assert.equal(
      cap.functions.value.some(([, fn]) => fn === 'revoke_head_delegation'),
      false
    );
  }
});

void test('a device consent and a whole approval are identity ceremonies, each bound to its exact payload', () => {
  for (const operation of ['sign_device_consent', 'sign_device_approval']) {
    const exact = {
      ...mandate(),
      delegate: null,
      subjects: [],
      binding: null,
      operations: [operation],
      exact_payload_json: '{"consent":1}',
    };
    const cap = scopedCapability(exact, 'mishpat', new Uint8Array(64), 99);
    assert.equal(cap.functions.type, 'listed');
    if (cap.functions.type === 'listed')
      assert.equal(
        cap.functions.value.some(([zome, fn]) => zome === 'mishpat' && fn === operation),
        true
      );
    // Never without the exact payload, and never on the content surface.
    assert.throws(() =>
      scopedCapability({ ...exact, exact_payload_json: null }, 'mishpat', new Uint8Array(64), 99)
    );
    assert.throws(() => scopedCapability(exact, 'content_store', new Uint8Array(64), 99));
  }
});

void test('the ceremony refuses wildcards, duplicates, missing bindings, expired grants and mixed authority surfaces', () => {
  for (const changed of [
    { subjects: [{ id: '*', root }] },
    {
      subjects: [
        { id: LESSON, root },
        { id: LESSON, root },
      ],
    },
    { binding: null },
    { valid_until: 99 },
    { policy: '' },
    { operations: ['grant_head_delegation', 'sign_device_enrollment'] },
  ])
    assert.throws(() =>
      scopedCapability({ ...mandate(), ...changed }, 'content_store', new Uint8Array(64), 99)
    );
});

void test('identity signing requires an exact payload and never inherits content privileges', () => {
  const identity = {
    ...mandate(),
    subjects: [],
    operations: ['sign_device_enrollment'],
    exact_payload_json: '[]',
  };
  const cap = scopedCapability(identity, 'mishpat', new Uint8Array(64), 99);
  assert.equal(cap.functions.type, 'listed');
  assert.throws(() =>
    scopedCapability({ ...identity, exact_payload_json: null }, 'mishpat', new Uint8Array(64), 99)
  );
  assert.throws(() => scopedCapability(identity, 'content_store', new Uint8Array(64), 99));
});

void test('the ceremony CLI persists Buffer hashes as complete native byte arrays', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'ceremony-output-'));
  const descriptor = join(directory, 'descriptor.json');
  const bytes = new Uint8Array([132, 41, 36, ...new Uint8Array(36).fill(9)]);
  let closed = false;
  writeCustodyJson(descriptor, {
    connection: {},
    mandate: { ...mandate(), operations: ['witness_device_publication'], exact_payload_json: '{}' },
  });
  const connect = (async () => {
    await Promise.resolve();
    return {
      agent: owner,
      dna,
      requester,
      call: async () => {
        await Promise.resolve();
        return { action_hash: Buffer.from(bytes), entry_hash: bytes };
      },
      close: async () => {
        await Promise.resolve();
        closed = true;
      },
    };
  }) as typeof connectConductor;
  try {
    await main(['exercise-ceremony', descriptor], connect);
    const result = JSON.parse(readFileSync(join(directory, 'ceremony-result.json'), 'utf8')) as {
      action_hash: number[];
      entry_hash: number[];
    };
    assert.deepEqual(result.action_hash, Array.from(bytes));
    assert.deepEqual(result.entry_hash, Array.from(bytes));
    assert.equal(encodeHashToBase64(new Uint8Array(result.action_hash)), encodeHashToBase64(bytes));
    assert.equal(closed, true);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
