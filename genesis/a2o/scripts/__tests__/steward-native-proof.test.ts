import assert from 'node:assert/strict';
import { test } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import { awaitNativeReceiver } from '../lib/steward-native-proof.js';

import type { Conductor } from '../lib/steward-conductor.js';

const raw = new Uint8Array(39).fill(1);
const rootRaw = new Uint8Array(39).fill(2);
const target = {
  id: 'fct',
  head: encodeHashToBase64(raw),
  root: encodeHashToBase64(rootRaw),
  dna: 'dna',
};
function fixture() {
  let now = 0;
  const calls: string[] = [];
  const timing = {
    now: () => now,
    sleep: async (ms: number) => {
      now += ms;
      await Promise.resolve();
    },
  };
  const c: Conductor = {
    agent: 'receiver',
    dna: 'dna',
    close: async () => {
      await Promise.resolve();
    },
    call: async <T>(name: string, input: unknown): Promise<T> => {
      await Promise.resolve();
      calls.push(name);
      if (name === 'resolve_content_head_local')
        return { head_action_hash: raw, canonical: true, canonical_earned: true } as T;
      if (name === 'resolve_canonical_election')
        return {
          winner_target: target.head,
          canonical_earned: true,
          canonical_link_hash: 'actual-link',
        } as T;
      assert.equal(name, 'get_content_lineage');
      assert.equal((input as { local: boolean }).local, true);
      if (now < 1000) throw new Error('lineage-not-held');
      return {
        referenced_action_hash: raw,
        root_action_hash: rootRaw,
        content_id: 'fct',
        truncated: false,
      } as T;
    },
  };
  return { c, calls, timing };
}
void test('waits for receiver-local ancestry even when head and election already match', async () => {
  const { c, calls, timing } = fixture();
  const proof = await awaitNativeReceiver(c, target, 2000, timing);
  assert.equal(proof.accepted, true);
  assert.equal(proof.acceptedAt, 1000);
  assert.equal(proof.electionLink, 'actual-link');
  assert.equal(calls.filter(name => name === 'get_content_lineage').length, 2);
});
void test('does not reset shared deadline for missing ancestry or late receivers', async () => {
  const { c, calls, timing } = fixture();
  assert.equal((await awaitNativeReceiver(c, target, 500, timing)).accepted, false);
  const before = calls.length;
  assert.equal((await awaitNativeReceiver(c, target, 500, timing)).accepted, false);
  assert.equal(calls.length, before);
});
void test('refuses another DNA before any native read', async () => {
  const { c, calls, timing } = fixture();
  await assert.rejects(
    awaitNativeReceiver(c, { ...target, dna: 'wrong' }, 2000, timing),
    /DNA mismatch/
  );
  assert.equal(calls.length, 0);
});
void test('a v3 native grant proof carries both its issuance and acceptance actions', async () => {
  const { c, calls, timing } = fixture();
  const grantTarget = {
    ...target,
    issuanceActionHash: encodeHashToBase64(new Uint8Array(39).fill(8)),
  };
  await assert.rejects(awaitNativeReceiver(c, grantTarget, 2000, timing), /acceptance witness/);
  assert.equal(calls.length, 0);
});
