import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import { main } from '../steward-credential.js';

import type { Conductor } from '../lib/steward-conductor.js';
import type { HeadDelegationWire } from '../lib/steward-delegation.js';

const DESCRIPTOR = 'descriptor.json';
const GRANT_HEADS = 'grant-heads';
const hash = (value: number) =>
  encodeHashToBase64(new Uint8Array([132, 41, 36, ...new Uint8Array(36).fill(value)]));

void test('selects one issued root, preserves descriptor and sibling evidence, and resumes without another grant', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'credential-selection-'));
  try {
    const path = join(dir, DESCRIPTOR);
    const mandate = {
      issuer: hash(1),
      requester: hash(2),
      dna: hash(3),
      delegate: hash(4),
      binding: hash(5),
      valid_until: 100,
      operations: ['grant_head_delegation'],
      subjects: [
        { id: 'course', root: hash(6) },
        { id: 'sibling', root: hash(7) },
      ],
    };
    const descriptor = JSON.stringify({ connection: {}, mandate });
    writeFileSync(path, descriptor);
    const sibling = { evidence: 'preserve' };
    writeFileSync(join(dir, 'delegations.json'), JSON.stringify({ sibling }));
    const calls: unknown[] = [];
    const c: Conductor = {
      agent: mandate.issuer,
      requester: mandate.requester,
      dna: mandate.dna,
      async call<T>(fn: string, payload: unknown): Promise<T> {
        assert.equal(fn, 'grant_head_delegation');
        calls.push(payload);
        const wire: HeadDelegationWire = {
          payload: {
            grantor: decodeHashFromBase64(mandate.issuer),
            delegate: decodeHashFromBase64(mandate.delegate),
            scope: 'course',
            valid_until: 100,
            root_action_hash: decodeHashFromBase64(hash(6)),
            dna_hash: decodeHashFromBase64(mandate.dna),
            device_binding: decodeHashFromBase64(mandate.binding),
          },
          signature: new Uint8Array(64),
          acceptance: null,
        };
        return await Promise.resolve(wire as T);
      },
      close: async () => {
        await Promise.resolve();
      },
    };
    await main([GRANT_HEADS, path, 'course'], async () => await Promise.resolve(c));
    assert.deepEqual(calls, [
      {
        scope: 'course',
        root_action_hash: decodeHashFromBase64(hash(6)),
        delegate: decodeHashFromBase64(mandate.delegate),
        valid_until: 100,
        device_binding: decodeHashFromBase64(mandate.binding),
      },
    ]);
    assert.equal(readFileSync(path, 'utf8'), descriptor);
    const saved = JSON.parse(readFileSync(join(dir, 'delegations.json'), 'utf8')) as Record<
      string,
      unknown
    >;
    assert.deepEqual(saved.sibling, sibling);
    await main([GRANT_HEADS, path, 'course'], async () => await Promise.resolve(c));
    assert.equal(calls.length, 1);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

void test('unknown or ambiguous selected IDs refuse before connecting or writing custody', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'credential-refusal-'));
  try {
    const path = join(dir, DESCRIPTOR);
    let connected = false;
    const connect = async (): Promise<Conductor> => {
      connected = true;
      return await Promise.reject(new Error('must never connect'));
    };
    for (const subjects of [
      [],
      [
        { id: 'course', root: hash(6) },
        { id: 'course', root: hash(7) },
      ],
    ]) {
      const descriptor = JSON.stringify({ connection: {}, mandate: { subjects } });
      writeFileSync(path, descriptor);
      await assert.rejects(main([GRANT_HEADS, path, 'course'], connect), /one exact root/);
      assert.equal(connected, false);
      assert.equal(readFileSync(path, 'utf8'), descriptor);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

void test('omitting the selector retains the existing all-subject ceremony', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'credential-all-subjects-'));
  try {
    const path = join(dir, DESCRIPTOR);
    const mandate = {
      issuer: hash(1),
      requester: hash(2),
      dna: hash(3),
      delegate: hash(4),
      binding: hash(5),
      valid_until: 100,
      operations: ['grant_head_delegation'],
      subjects: [
        { id: 'first', root: hash(6) },
        { id: 'second', root: hash(7) },
      ],
    };
    writeFileSync(path, JSON.stringify({ connection: {}, mandate }));
    const calls: string[] = [];
    const c: Conductor = {
      agent: mandate.issuer,
      requester: mandate.requester,
      dna: mandate.dna,
      async call<T>(_fn: string, payload: unknown): Promise<T> {
        const input = payload as { scope: string; root_action_hash: Uint8Array };
        calls.push(input.scope);
        const wire: HeadDelegationWire = {
          payload: {
            grantor: decodeHashFromBase64(mandate.issuer),
            delegate: decodeHashFromBase64(mandate.delegate),
            scope: input.scope,
            root_action_hash: input.root_action_hash,
            valid_until: 100,
            dna_hash: decodeHashFromBase64(mandate.dna),
            device_binding: decodeHashFromBase64(mandate.binding),
          },
          signature: new Uint8Array(64),
          acceptance: null,
        };
        return await Promise.resolve(wire as T);
      },
      close: async () => {
        await Promise.resolve();
      },
    };
    await main([GRANT_HEADS, path], async () => await Promise.resolve(c));
    assert.deepEqual(calls, ['first', 'second']);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
