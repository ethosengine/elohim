import assert from 'node:assert/strict';
import { test } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import {
  scopedCapability,
  MANDATE_TAG,
  type InvocationMandate,
} from '../lib/steward-credential.js';

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
