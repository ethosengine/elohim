/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns test promises. */
import { strict as assert } from 'node:assert';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { it } from 'node:test';

import { publicationOrder, type RepoItem } from '../lib/steward-items.js';
import { pendingPublication, savePublication } from '../lib/steward-publication-receipt.js';

it('resumes the exact authored version and refuses changed input, device or network', () => {
  const dir = mkdtempSync(join(tmpdir(), 'steward-publication-'));
  const receipt = {
    id: 'lesson',
    seedHash: 'seed-v2',
    agent: 'che',
    dna: 'network-a',
    storage: 'https://storage.example',
    head: 'authored-v2',
    authoredAt: '2026-09-30T00:00:00Z',
  };
  try {
    assert.equal(pendingPublication(dir, receipt), undefined);
    savePublication(dir, receipt);
    assert.deepEqual(pendingPublication(dir, receipt), receipt);
    for (const key of ['seedHash', 'agent', 'dna', 'storage'] as const) {
      assert.throws(
        () => pendingPublication(dir, { ...receipt, [key]: 'different' }),
        /resume the original/
      );
    }
    savePublication(dir, { ...receipt, declaredAt: '2026-09-30T00:01:00Z' });
    assert.equal(pendingPublication(dir, receipt), undefined);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

function item(id: string, refs?: string[]): RepoItem {
  return {
    id,
    kind: refs ? 'path' : 'content',
    file: `${id}.json`,
    json: {
      id,
      title: id,
      chapters: refs?.map(resourceId => ({ steps: [{ resourceId }] })),
    } as RepoItem['json'],
  };
}

it('publishes leaves and nested paths before their composition, refusing composition cycles', () => {
  const course = item('course', ['module']);
  const module = item('module', ['lesson']);
  const lesson = item('lesson');
  assert.deepEqual(
    publicationOrder([course, module, lesson]).map(i => i.id),
    ['lesson', 'module', 'course']
  );
  assert.throws(() => publicationOrder([item('a', ['b']), item('b', ['a'])]), /cyclic course/);
});
