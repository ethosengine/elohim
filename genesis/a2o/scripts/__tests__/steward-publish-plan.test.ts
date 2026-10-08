/* eslint-disable @typescript-eslint/no-floating-promises -- node:test describe/it
   return promises that the test runner itself consumes; awaiting them is wrong. */
/**
 * Steward publish decisions (scripts/lib/steward-publish-plan.ts): the commons fence,
 * the seed-hash identity shared with the pipeline seeder, and carry-or-refuse.
 */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { buildContentInput, buildPathInput } from '../../../seeder/src/content-input.js';
import { seedHashFor } from '../../../seeder/src/seed-idempotency.js';
import {
  commonsFenceRefusal,
  landedMismatches,
  planItem,
  publishPatch,
  type ExistingRow,
} from '../lib/steward-publish-plan.js';

const content = buildContentInput({
  id: 'c1',
  title: 'A lesson',
  content: '# body',
  contentFormat: 'markdown',
  reach: 'commons',
  tags: ['b', 'a'],
});

const path = buildPathInput({
  id: 'p1',
  title: 'A path',
  reach: 'commons',
  chapters: [{ id: 'ch1', title: 'One', steps: [{ resourceId: 'c1', stepTitle: 'Lesson' }] }],
});

function anchoredRow(over: Partial<ExistingRow> = {}): ExistingRow {
  return {
    id: 'c1',
    title: 'A lesson',
    contentType: 'concept',
    contentFormat: 'markdown',
    contentBody: '# body',
    reach: 'commons',
    tags: ['a', 'b'],
    metadata: {},
    dhtAnchorHash: 'uhCkkOLD',
    ...over,
  };
}

describe('commonsFenceRefusal', () => {
  it('admits authored commons only', () => {
    assert.equal(commonsFenceRefusal('x', { reach: 'commons' }), undefined);
    assert.equal(commonsFenceRefusal('x', { visibility: 'commons' }), undefined);
  });
  it('refuses an intimate love map, naming its reach', () => {
    const why = commonsFenceRefusal('love-map-matthew-jessica', {
      reach: 'intimate',
      visibility: 'intimate',
    });
    assert.match(why ?? '', /love-map-matthew-jessica.*"intimate"/);
  });
  it('refuses public and ungraded rows', () => {
    assert.match(commonsFenceRefusal('p', { visibility: 'public' }) ?? '', /"public"/);
    assert.match(commonsFenceRefusal('u', {}) ?? '', /no authored reach/);
  });
  it('prefers reach over the legacy visibility key', () => {
    assert.match(
      commonsFenceRefusal('x', { reach: 'intimate', visibility: 'commons' }) ?? '',
      /"intimate"/
    );
  });
});

describe('planItem', () => {
  it('creates when the steward has no row', () => {
    assert.equal(planItem(content, undefined).action, 'create');
  });

  it('is unchanged only when the seed hash matches AND the head is declared', () => {
    const h = seedHashFor(content);
    const row = anchoredRow({ metadata: { seedHash: h } });
    assert.equal(planItem(content, row, 'uhCkkOLD').action, 'unchanged');
    assert.equal(planItem(content, row).action, 'declare');
    assert.equal(planItem(content, row, 'uhCkkOTHER').action, 'declare');
  });

  it('updates an anchored row whose only changes ride the conductor (reach, metadata)', () => {
    const plan = planItem(content, anchoredRow());
    assert.equal(plan.action, 'update');
    assert.deepEqual(plan.uncarried, []);
  });

  it('bootstraps a seeded SQL row even when its seed hash already matches', () => {
    const row = anchoredRow({ dhtAnchorHash: null, metadata: { seedHash: seedHashFor(content) } });
    const plan = planItem(content, row);
    assert.equal(plan.action, 'update');
    assert.deepEqual(plan.uncarried, []);
    assert.equal(planItem(content, { ...row, contentType: 'path' }).action, 'blocked');
  });

  it('updates an anchored row whose body changed — update_content carries the body', () => {
    const plan = planItem(content, anchoredRow({ contentBody: '# older body' }));
    assert.equal(plan.action, 'update');
    assert.deepEqual(plan.uncarried, []);
  });

  it('updates an anchored path whose sections tree changed', () => {
    const row = anchoredRow({
      id: 'p1',
      contentType: 'path',
      contentFormat: 'epr-composite',
      contentBody: '{"schemaVersion":1,"sections":[]}',
      tags: [],
    });
    assert.deepEqual(planItem(path, row).uncarried, []);
  });

  it('updates an anchored row whose tags changed — the signed version carries tags (F17)', () => {
    const plan = planItem(content, anchoredRow({ tags: ['older'] }));
    assert.equal(plan.action, 'update');
    assert.deepEqual(plan.uncarried, []);
  });

  it('updates an anchored row whose content type changed — the signed version carries it (F17)', () => {
    const plan = planItem(content, anchoredRow({ contentType: 'reference' }));
    assert.equal(plan.action, 'update');
    assert.deepEqual(plan.uncarried, []);
  });

  it('treats public and commons as one grade, and lets an anchored row widen to it', () => {
    assert.deepEqual(planItem(content, anchoredRow({ reach: 'public' })).uncarried, []);
    assert.deepEqual(planItem(content, anchoredRow({ reach: 'private' })).uncarried, []);
    const narrowed = buildContentInput({
      id: 'c1',
      title: 'A lesson',
      content: '# body',
      contentFormat: 'markdown',
      reach: 'community',
      tags: ['b', 'a'],
    });
    assert.deepEqual(planItem(narrowed, anchoredRow()).uncarried, [
      'reach (other peers do not adopt a reach change)',
    ]);
    assert.deepEqual(
      landedMismatches(content, seedHashFor(content), {
        ...anchoredRow({ reach: 'public', metadata: { seedHash: seedHashFor(content) } }),
      }),
      []
    );
  });

  it('lets an UNANCHORED row take a new body (create_content bootstrap carries it)', () => {
    const plan = planItem(
      content,
      anchoredRow({ dhtAnchorHash: null, contentBody: '# older', tags: [] })
    );
    assert.equal(plan.action, 'update');
  });

  it('still blocks a content-type change on an UNANCHORED row — create_content keeps the stored type', () => {
    assert.deepEqual(
      planItem(content, anchoredRow({ dhtAnchorHash: null, contentType: 'path' })).uncarried,
      ['contentType (create_content keeps the stored type)']
    );
  });
});

describe('publishPatch', () => {
  it('always carries reach (so it re-notarizes) and the seed hash in metadata', () => {
    const h = seedHashFor(content);
    const patch = publishPatch(content, h);
    assert.equal(patch.reach, 'commons');
    assert.equal(patch.contentType, 'concept');
    assert.deepEqual((patch.metadata as Record<string, unknown>).seedHash, h);
    assert.equal(patch.contentBody, '# body');
    assert.equal('blobHash' in patch, false);
  });
});

describe('landedMismatches', () => {
  it('is empty when the re-read row holds the publish', () => {
    const h = seedHashFor(content);
    const row = anchoredRow({
      reach: 'commons',
      metadata: { seedHash: h },
      dhtAnchorHash: 'uhCkkNEW',
    });
    assert.deepEqual(landedMismatches(content, h, row), []);
  });
  it('names a body the PATCH did not land', () => {
    const h = seedHashFor(content);
    const row = anchoredRow({ reach: 'commons', metadata: { seedHash: h }, contentBody: 'stale' });
    assert.deepEqual(landedMismatches(content, h, row), ['contentBody']);
  });
});

describe('reach on an anchored row', () => {
  it('is refused when it would move to a grade adoption does not carry (anything but a widening to the open grade)', () => {
    const toCommunity = buildContentInput({
      id: 'c1',
      title: 'A lesson',
      content: '# body',
      contentFormat: 'markdown',
      reach: 'community',
      tags: ['b', 'a'],
    });
    const plan = planItem(toCommunity, anchoredRow({ reach: 'commons' }));
    assert.equal(plan.action, 'blocked');
    assert.deepEqual(plan.uncarried, ['reach (other peers do not adopt a reach change)']);
    // public -> commons is the same grade; private -> commons is the F19 widening.
    assert.equal(planItem(content, anchoredRow({ reach: 'public' })).action, 'update');
    assert.equal(planItem(content, anchoredRow({ reach: 'private' })).action, 'update');
  });
});
