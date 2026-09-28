import { describe, expect, it } from 'vitest';

import { SEED_HASH_KEY, canonicalSeedHash, stableStringify } from '../seed-hash.js';

const base = {
  id: 'fct-module-01-church-dilemma',
  title: 'The Church Dilemma',
  contentType: 'lesson',
  contentFormat: 'markdown',
  contentBody: '# Lament',
  reach: 'commons',
  tags: ['fct', 'lament'],
  metadata: { moduleNumber: 1, movement: 1 },
};

describe('stableStringify', () => {
  it('sorts keys at every depth and drops undefined', () => {
    expect(stableStringify({ b: 1, a: { d: [2, { f: 1, e: 0 }], c: undefined } })).toBe(
      '{"a":{"d":[2,{"e":0,"f":1}]},"b":1}'
    );
  });
});

describe('canonicalSeedHash', () => {
  it('is stable under key order, tag order and duplicate tags', () => {
    const reordered = {
      ...base,
      tags: ['lament', 'fct', 'fct'],
      metadata: { movement: 1, moduleNumber: 1 },
    };
    expect(canonicalSeedHash(reordered)).toBe(canonicalSeedHash(base));
  });

  it('ignores a previously stored seed hash in metadata', () => {
    const stamped = { ...base, metadata: { ...base.metadata, [SEED_HASH_KEY]: 'sha256-old' } };
    expect(canonicalSeedHash(stamped)).toBe(canonicalSeedHash(base));
  });

  it('changes when authored meaning changes', () => {
    expect(canonicalSeedHash({ ...base, contentBody: '# Lament, revised' })).not.toBe(
      canonicalSeedHash(base)
    );
    expect(canonicalSeedHash({ ...base, reach: 'intimate' })).not.toBe(canonicalSeedHash(base));
  });

  it('treats absent and null optional fields alike', () => {
    const { contentFormat: _f, ...noFormat } = base;
    expect(canonicalSeedHash({ ...noFormat, contentFormat: null })).toBe(
      canonicalSeedHash(noFormat)
    );
  });

  it('hashes a structured body by value, not by key order', () => {
    const a = { ...base, contentFormat: 'sophia-quiz-json', contentBody: '[{"id":"q1","purpose":"mastery"}]' };
    const b = { ...a, contentBody: '[{"purpose":"mastery","id":"q1"}]' };
    expect(canonicalSeedHash(a)).toBe(canonicalSeedHash(b));
    expect(canonicalSeedHash({ ...a, contentBody: '[{"id":"q2","purpose":"mastery"}]' })).not.toBe(
      canonicalSeedHash(a)
    );
  });

  it('hashes prose that merely starts with a bracket as text', () => {
    const prose = { ...base, contentBody: '[Draft] lament' };
    expect(canonicalSeedHash(prose)).not.toBe(canonicalSeedHash(base));
  });
});
