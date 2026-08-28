import { describe, expect, it } from 'vitest';
import { LAMAD_RELATIONSHIPS } from '../generated/manifest-types.js';
import { RelationshipRemapLedger, canonicalRelationshipType } from '../relationship-vocabulary.js';

describe('canonicalRelationshipType', () => {
  it('passes every manifest id through unchanged, case-insensitively', () => {
    for (const id of LAMAD_RELATIONSHIPS) {
      expect(canonicalRelationshipType(id)).toEqual({ type: id, via: 'manifest', authored: id });
      expect(canonicalRelationshipType(id.toLowerCase()).type).toBe(id);
    }
  });

  it('maps the authored aliases seen in genesis/data/lamad/content onto manifest ids', () => {
    expect(canonicalRelationshipType('extends')).toMatchObject({ type: 'DEPENDS_ON', via: 'alias' });
    expect(canonicalRelationshipType('DERIVED_FROM')).toMatchObject({ type: 'DEPENDS_ON', via: 'alias' });
    expect(canonicalRelationshipType('prereq')).toMatchObject({ type: 'REQUIRES', via: 'alias' });
    expect(canonicalRelationshipType('FOLLOWUP')).toMatchObject({ type: 'FOLLOWS', via: 'alias' });
  });

  it('never emits a type outside the manifest — unknowns fall back to RELATES_TO and are counted', () => {
    const manifest = new Set<string>(LAMAD_RELATIONSHIPS as readonly string[]);
    const ledger = new RelationshipRemapLedger();
    for (const raw of ['likert-scale', 'wisdom-invoke', '', undefined, 'EXTENDS', 'synthesize']) {
      const c = canonicalRelationshipType(raw);
      expect(manifest.has(c.type)).toBe(true);
      ledger.note(c);
    }
    const summary = ledger.summary();
    expect(summary).toContain('→ RELATES_TO (fallback)');
    expect(summary).toContain('EXTENDS → DEPENDS_ON (alias)');
  });

  it('a manifest-only run leaves the ledger silent', () => {
    const ledger = new RelationshipRemapLedger();
    ledger.note(canonicalRelationshipType('CONTAINS'));
    expect(ledger.summary()).toBeNull();
  });
});
