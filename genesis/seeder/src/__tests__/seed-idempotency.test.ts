import { describe, expect, it } from 'vitest';

import type { CreateContentInput } from '../generated/create-content-input.js';
import { SEED_HASH_KEY } from '../seed-hash.js';
import {
  decideSeedAction,
  deferToSteward,
  reachReconcileTargets,
  seedHashFor,
  withSeedHash,
  type SeedDecision,
  type StoredContentRow,
} from '../seed-idempotency.js';

const input: CreateContentInput = {
  id: 'fct-module-01-church-dilemma',
  title: 'The Church Dilemma',
  schemaVersion: 1,
  contentType: 'lesson',
  contentFormat: 'markdown',
  contentBody: '# Lament',
  contentSizeBytes: 8,
  reach: 'commons',
  tags: ['fct', 'lament'],
  metadata: { moduleNumber: 1, summary: 'Lament' },
  dhtAnchorHash: 'bafkreiexample',
};

/** What GET /db/content/{id} returns for a row the seeder inserted from `input` (legacy: no seedHash). */
function storedFrom(i: CreateContentInput, extra: Partial<StoredContentRow> = {}): StoredContentRow {
  return {
    id: i.id,
    title: i.title,
    description: i.description ?? null,
    contentType: i.contentType!,
    contentFormat: i.contentFormat ?? null,
    contentBody: i.contentBody ?? null,
    blobHash: i.blobHash ?? null,
    reach: i.reach ?? null,
    tags: [...(i.tags ?? [])].reverse(), // storage returns tags in its own order
    metadata: i.metadata ?? null,
    ...extra,
  };
}

const found = (row: StoredContentRow) => ({ kind: 'found' as const, row });

describe('decideSeedAction', () => {
  const h = seedHashFor(input);

  it('missing → insert, and the insert carries metadata.seedHash', () => {
    const d = decideSeedAction(input, { kind: 'missing' });
    expect(d).toEqual({ kind: 'insert', id: input.id, seedHash: h });
    expect(withSeedHash(input, h).metadata).toEqual({ ...input.metadata, [SEED_HASH_KEY]: h });
  });

  it('the hash ignores volatile fields (anchor, size, schemaVersion)', () => {
    expect(seedHashFor({ ...input, dhtAnchorHash: 'other', contentSizeBytes: 1, schemaVersion: 2 })).toBe(h);
  });

  it('stored seedHash equal → unchanged, regardless of other stored drift', () => {
    const row = storedFrom(input, { metadata: { ...input.metadata, [SEED_HASH_KEY]: h }, title: 'drifted' });
    expect(decideSeedAction(input, found(row))).toEqual({ kind: 'unchanged', id: input.id, seedHash: h, via: 'seedHash' });
  });

  it('stored seedHash different → update with a diesel-only patch carrying the new seedHash', () => {
    const changed = { ...input, contentBody: '# Lament, revised' };
    const row = storedFrom(input, { metadata: { ...input.metadata, [SEED_HASH_KEY]: h } });
    const d = decideSeedAction(changed, found(row));
    expect(d.kind).toBe('update');
    if (d.kind !== 'update') return;
    expect(d.via).toBe('seedHash');
    expect(d.patch.contentBody).toBe('# Lament, revised');
    expect(d.patch.metadata[SEED_HASH_KEY]).toBe(seedHashFor(changed));
    expect(d.patch).not.toHaveProperty('reach');
    expect(d.patch).not.toHaveProperty('blobHash');
    expect(d.reachPatch).toBeUndefined();
    expect(d.unpatchable).toEqual([]);
  });

  it('legacy row (no seedHash) that reads back equal → unchanged, and is NOT stamped', () => {
    const d = decideSeedAction(input, found(storedFrom(input)));
    expect(d).toEqual({ kind: 'unchanged', id: input.id, seedHash: h, via: 'legacy' });
  });

  it('legacy compare normalizes what the seeder could not have authored', () => {
    const noDesc = { ...input, description: undefined };
    const row = storedFrom(noDesc, {
      description: '', // conductor bootstrap stores "" for an absent description
      blobHash: 'sha256-deploy-staged', // deploy-owned: repository authors none
      metadata: { ...input.metadata, releaseChannel: 'dev', serverBlobHash: 'sha256-ssr' }, // other writers' keys
    });
    expect(decideSeedAction(noDesc, found(row)).kind).toBe('unchanged');
  });

  it('legacy row that differs → update (via legacy)', () => {
    const row = storedFrom(input, { contentBody: '# Old text' });
    const d = decideSeedAction(input, found(row));
    expect(d).toMatchObject({ kind: 'update', via: 'legacy', patch: { contentBody: '# Lament' } });
  });

  it('a missing authored metadata key on the stored row is a change', () => {
    const row = storedFrom(input, { metadata: { moduleNumber: 1 } });
    expect(decideSeedAction(input, found(row)).kind).toBe('update');
  });

  it('a changed reach is carried separately (conductor path), never inside the field patch', () => {
    const row = storedFrom(input, { reach: 'public' });
    const d = decideSeedAction(input, found(row));
    expect(d).toMatchObject({ kind: 'update', reachPatch: 'commons' });
    if (d.kind === 'update') expect(d.patch).not.toHaveProperty('reach');
  });

  it('names differences a PATCH cannot carry and never writes deploy-owned metadata', () => {
    const authored = { ...input, blobHash: 'sha256-new', metadata: { ...input.metadata, serverBlobHash: 'x' } };
    const row = storedFrom(input, { contentType: 'concept', blobHash: 'sha256-old' });
    const d = decideSeedAction(authored, found(row));
    expect(d.kind).toBe('update');
    if (d.kind !== 'update') return;
    expect(d.unpatchable).toEqual(['contentType', 'blobHash']);
    expect(d.patch.metadata).not.toHaveProperty('serverBlobHash');
  });

  it('unreadable (403 reach gate) → unverified, nothing written', () => {
    expect(decideSeedAction(input, { kind: 'unreadable', status: 403, requiredReach: 'intimate' })).toEqual({
      kind: 'unverified', id: input.id, seedHash: h, status: 403, requiredReach: 'intimate',
    });
  });

  it('read error → failed, nothing written', () => {
    expect(decideSeedAction(input, { kind: 'error', message: 'boom' })).toMatchObject({ kind: 'failed', message: 'boom' });
  });
});

describe('reachReconcileTargets', () => {
  it('re-notarizes reach only for updated rows whose stored reach differs', () => {
    const decisions: SeedDecision[] = [
      decideSeedAction(input, { kind: 'missing' }),
      decideSeedAction(input, found(storedFrom(input))),
      decideSeedAction({ ...input, id: 'a', contentBody: 'new' }, found(storedFrom({ ...input, id: 'a' }))),
      decideSeedAction({ ...input, id: 'b' }, found(storedFrom({ ...input, id: 'b' }, { reach: 'public' }))),
      decideSeedAction(input, { kind: 'unreadable', status: 403, requiredReach: 'private' }),
    ];
    expect(decisions.map(d => d.kind)).toEqual(['insert', 'unchanged', 'update', 'update', 'unverified']);
    expect(reachReconcileTargets(decisions)).toEqual([{ id: 'b', reach: 'commons' }]);
  });
});

describe('deferToSteward', () => {
  const update = {
    kind: 'update' as const,
    id: 'fct-module-01-church-dilemma',
    seedHash: 'sha256-new',
    via: 'seedHash' as const,
    patch: {},
    unpatchable: [],
  };

  it('leaves a changed row to its steward when the head was earned', () => {
    expect(deferToSteward(update, { earned: true, earnedBy: 'uhCAkSteward' })).toEqual({
      kind: 'stewarded',
      id: update.id,
      seedHash: 'sha256-new',
      earnedBy: 'uhCAkSteward',
    });
  });

  it('updates as usual when no steward earned the head, or the head is unknown', () => {
    expect(deferToSteward(update, { earned: false })).toBe(update);
    expect(deferToSteward(update, undefined)).toBe(update);
  });

  it('never touches a decision that is not an update', () => {
    const unchanged = { kind: 'unchanged' as const, id: 'x', seedHash: 'h', via: 'seedHash' as const };
    expect(deferToSteward(unchanged, { earned: true })).toBe(unchanged);
  });
});
