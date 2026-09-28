import { describe, expect, it } from 'vitest';

import { buildRelationshipInputs, relationshipId } from '../content-input.js';
import { RelationshipRemapLedger } from '../relationship-vocabulary.js';
import { decideRelationshipAction, type StoredRelationshipRow } from '../seed-idempotency.js';

describe('buildRelationshipInputs', () => {
  it('carries the authored role in edge metadata and a deterministic storage-identity id', () => {
    const edges = buildRelationshipInputs({
      id: 'fct-module-01',
      reach: 'commons',
      relationships: [
        { target: 'fct-bible-psalm-13', type: 'REFERENCES', role: 'anchor' },
        { target: 'fct-module-01-story', type: 'CONTAINS' },
      ],
    });
    expect(edges).toEqual([
      {
        id: relationshipId('fct-module-01', 'CONTAINS', 'fct-module-01-story'),
        schemaVersion: 1,
        sourceId: 'fct-module-01',
        targetId: 'fct-module-01-story',
        relationshipType: 'CONTAINS',
        confidence: 1,
        inferenceSource: 'explicit',
        reach: 'commons',
      },
      {
        id: relationshipId('fct-module-01', 'REFERENCES', 'fct-bible-psalm-13'),
        schemaVersion: 1,
        sourceId: 'fct-module-01',
        targetId: 'fct-bible-psalm-13',
        relationshipType: 'REFERENCES',
        confidence: 1,
        inferenceSource: 'explicit',
        reach: 'commons',
        metadata: { role: 'anchor' },
      },
    ]);
    expect(edges[1].id).toBe(relationshipId('fct-module-01', 'REFERENCES', 'fct-bible-psalm-13'));
    expect(edges[1].id).toMatch(/^rel-[0-9a-f]{32}$/);
  });

  it('inherits the source atom reach through the content builder resolution', () => {
    const one = (json: Parameters<typeof buildRelationshipInputs>[0], opts = {}) =>
      buildRelationshipInputs(json, opts)[0].reach;
    const rel = [{ target: 'b', type: 'REFERENCES' }];
    // inverted burden: ungraded → private
    expect(one({ id: 'a', relationships: rel })).toBe('private');
    expect(one({ id: 'a', reach: 'commons', relationships: rel })).toBe('commons');
    // the account-package advisory may only raise it
    expect(one({ id: 'a', reach: 'community', relationships: rel }, { advisoryReach: 'public' })).toBe('public');
    expect(one({ id: 'a', reach: 'public', relationships: rel }, { advisoryReach: 'community' })).toBe('public');
    // the doorway seeder passes the reach its row was written at
    expect(one({ id: 'a', relationships: rel }, { reach: 'public' })).toBe('public');
    // non-canonical authored reach hard-fails, as it does for the row
    expect(() => one({ id: 'a', reach: 'neighborhood', relationships: rel })).toThrow(/non-canonical reach/);
  });

  it('reads legacy shapes: targetId/target_id, relationship_type, metadata.relationships, relatedNodeIds', () => {
    const edges = buildRelationshipInputs({
      id: 'src',
      reach: 'public',
      relationships: [{ targetId: 't1', relationshipType: 'DEPENDS_ON', confidence: 0.5, inference_source: 'author' }],
      metadata: { relationships: [{ target_id: 't2', relationship_type: 'FOLLOWS' }] },
      relatedNodeIds: ['t3'],
    });
    expect(edges.map(e => [e.relationshipType, e.targetId, e.confidence, e.inferenceSource, e.metadata])).toEqual([
      ['DEPENDS_ON', 't1', 0.5, 'author', undefined],
      ['FOLLOWS', 't2', 1, 'explicit', undefined],
      ['RELATES_TO', 't3', 1, 'explicit', { role: 'related' }],
    ]);
  });

  it('canonicalizes prose types onto the manifest and counts the remaps', () => {
    const remaps = new RelationshipRemapLedger();
    const edges = buildRelationshipInputs(
      {
        id: 'unit-why-elohim',
        reach: 'public',
        relationships: [
          { target: 'a', type: 'DERIVED_FROM' },
          { target: 'b', type: 'prereq' },
          { target: 'c', type: 'extends' },
          { target: 'd', type: 'wholly-unknown' },
          { target: 'e', type: 'references' },
        ],
      },
      { remaps },
    );
    expect(edges.map(e => `${e.relationshipType}:${e.targetId}`)).toEqual([
      'DEPENDS_ON:a',
      'DEPENDS_ON:c',
      'REFERENCES:e',
      'RELATES_TO:d',
      'REQUIRES:b',
    ]);
    // 'references' upper-cases onto a manifest id — not a remap
    expect(remaps.total()).toBe(4);
    expect(remaps.summary()).toContain('DERIVED_FROM → DEPENDS_ON (alias)');
    expect(remaps.summary()).toContain('wholly-unknown → RELATES_TO (fallback)');
  });

  it('dedupes repeats and collapses onto storage identity with the typed edge winning', () => {
    const edges = buildRelationshipInputs({
      id: 'm1',
      reach: 'commons',
      relationships: [
        { target: 'm2', type: 'RELATES_TO', role: 'callback' },
        { target: 'm2', type: 'RELATES_TO', role: 'callback' },
        { target: 'm2', type: 'relates', role: 'other' },
        { target: 'p', type: 'REFERENCES', role: 'anchor' },
      ],
      metadata: { relationships: [{ target: 'p', type: 'REFERENCES', role: 'supporting' }] },
      relatedNodeIds: ['m2', 'm3', 'm3'],
    });
    expect(edges.map(e => [e.relationshipType, e.targetId, e.metadata?.role])).toEqual([
      ['REFERENCES', 'p', 'anchor'],
      ['RELATES_TO', 'm2', 'callback'],
      ['RELATES_TO', 'm3', 'related'],
    ]);
    expect(new Set(edges.map(e => e.id)).size).toBe(edges.length);
  });

  it('skips self-edges and items without a target', () => {
    const edges = buildRelationshipInputs({
      id: 'self',
      reach: 'public',
      relationships: [
        { target: 'self', type: 'REFERENCES' },
        { type: 'REFERENCES' },
        { target: '  ', type: 'REFERENCES' },
        { target: 'other', type: 'REFERENCES' },
      ],
      relatedNodeIds: ['self'],
    });
    expect(edges.map(e => e.targetId)).toEqual(['other']);
  });

  it('is deterministic regardless of authored order', () => {
    const a = buildRelationshipInputs({
      id: 's',
      reach: 'public',
      relationships: [{ target: 'z', type: 'CONTAINS' }, { target: 'a', type: 'REFERENCES' }, { target: 'b', type: 'CONTAINS' }],
    });
    const b = buildRelationshipInputs({
      id: 's',
      reach: 'public',
      relationships: [{ target: 'b', type: 'CONTAINS' }, { target: 'a', type: 'REFERENCES' }, { target: 'z', type: 'CONTAINS' }],
    });
    expect(a).toEqual(b);
    expect(a.map(e => e.targetId)).toEqual(['b', 'z', 'a']);
  });

  it('clamps an out-of-range confidence to what storage accepts', () => {
    const [e] = buildRelationshipInputs({ id: 's', reach: 'public', relationships: [{ target: 't', type: 'REFERENCES', confidence: 3 }] });
    expect(e.confidence).toBe(1);
  });
});

describe('decideRelationshipAction', () => {
  const desired = { confidence: 1, inferenceSource: 'explicit', reach: 'commons', metadata: { role: 'anchor' } };
  const stored = (over: Partial<StoredRelationshipRow> = {}): StoredRelationshipRow => ({
    id: 'rel-x',
    sourceId: 's',
    targetId: 't',
    relationshipType: 'REFERENCES',
    confidence: 1,
    inferenceSource: 'explicit',
    reach: 'commons',
    metadata: { role: 'anchor' },
    ...over,
  });

  it('inserts a missing edge', () => {
    expect(decideRelationshipAction(desired, undefined)).toEqual({ kind: 'insert' });
  });

  it('leaves a current edge alone, whatever its id', () => {
    expect(decideRelationshipAction(desired, stored({ id: 'some-legacy-uuid' }))).toEqual({ kind: 'unchanged', reachDiffers: false });
  });

  it('updates when the role, confidence or inference source changed', () => {
    expect(decideRelationshipAction(desired, stored({ metadata: null }))).toEqual({
      kind: 'update', reachDiffers: false, changed: ['metadata'],
    });
    expect(decideRelationshipAction(desired, stored({ confidence: 0.5, inferenceSource: 'inferred' }))).toEqual({
      kind: 'update', reachDiffers: false, changed: ['confidence', 'inferenceSource'],
    });
    expect(decideRelationshipAction({ ...desired, metadata: undefined }, stored({ metadata: null }))).toEqual({
      kind: 'unchanged', reachDiffers: false,
    });
  });

  it('reports a reach difference but never writes for it (the bulk route cannot carry reach)', () => {
    expect(decideRelationshipAction({ ...desired, reach: 'public' }, stored())).toEqual({ kind: 'unchanged', reachDiffers: true });
  });
});
