/**
 * Epic Domain Discovery Instrument Tests
 *
 * Tests for psychometric instrument definition and scoring logic.
 */

import {
  EPIC_DOMAIN_INSTRUMENT_ID,
  EPIC_DOMAIN_SUBSCALES,
  EPIC_DOMAIN_RESULT_TYPES,
  EPIC_DOMAIN_INSTRUMENT_CONFIG,
  getEpicSubscale,
  getEpicSubscaleIds,
  getEpicResultType,
  findPrimaryEpicDomain,
  sortEpicDomainsByScore,
} from './epic-domain.instrument';

describe('Epic Domain Discovery Instrument', () => {
  // ==========================================================================
  // Instrument ID
  // ==========================================================================

  it('should have valid instrument ID', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_ID).toBe('epic-domain-discovery');
    expect(typeof EPIC_DOMAIN_INSTRUMENT_ID).toBe('string');
  });

  // ==========================================================================
  // Subscale Definitions
  // ==========================================================================

  it('should define five subscales', () => {
    expect(EPIC_DOMAIN_SUBSCALES).toBeDefined();
    expect(EPIC_DOMAIN_SUBSCALES.length).toBe(5);
  });

  it('should have governance subscale', () => {
    const governance = EPIC_DOMAIN_SUBSCALES.find(s => s.id === 'governance');
    expect(governance).toBeDefined();
    expect(governance?.name).toBe('AI Constitutional');
    expect(governance?.dimension).toBe('epic-domain');
  });

  it('should have care subscale', () => {
    const care = EPIC_DOMAIN_SUBSCALES.find(s => s.id === 'care');
    expect(care).toBeDefined();
    expect(care?.name).toBe('Value Scanner');
  });

  it('should have economic subscale', () => {
    const economic = EPIC_DOMAIN_SUBSCALES.find(s => s.id === 'economic');
    expect(economic).toBeDefined();
    expect(economic?.name).toBe('Economic Coordination');
  });

  it('should have public subscale', () => {
    const publicSub = EPIC_DOMAIN_SUBSCALES.find(s => s.id === 'public');
    expect(publicSub).toBeDefined();
    expect(publicSub?.name).toBe('Public Observer');
  });

  it('should have social subscale', () => {
    const social = EPIC_DOMAIN_SUBSCALES.find(s => s.id === 'social');
    expect(social).toBeDefined();
    expect(social?.name).toBe('Social Medium');
  });

  it('should have description for each subscale', () => {
    EPIC_DOMAIN_SUBSCALES.forEach(subscale => {
      expect(subscale.description).toBeDefined();
      expect(subscale.description?.length).toBeGreaterThan(0);
    });
  });

  it('should have color for each subscale', () => {
    EPIC_DOMAIN_SUBSCALES.forEach(subscale => {
      expect(subscale.color).toBeDefined();
      expect(subscale.color).toMatch(/^#[0-9A-F]{6}$/i);
    });
  });

  it('should have icon for each subscale', () => {
    EPIC_DOMAIN_SUBSCALES.forEach(subscale => {
      expect(subscale.icon).toBeDefined();
      expect(subscale.icon?.length).toBeGreaterThan(0);
    });
  });

  // ==========================================================================
  // Result Type Definitions
  // ==========================================================================

  it('should define five result types', () => {
    expect(EPIC_DOMAIN_RESULT_TYPES).toBeDefined();
    expect(EPIC_DOMAIN_RESULT_TYPES.length).toBe(5);
  });

  it('should have governance result type', () => {
    const governance = EPIC_DOMAIN_RESULT_TYPES.find(r => r.id === 'governance');
    expect(governance).toBeDefined();
    expect(governance?.name).toBe('AI Constitutional');
    expect(governance?.description).toBeDefined();
  });

  it('should have result type for each subscale', () => {
    const subscaleIds = EPIC_DOMAIN_SUBSCALES.map(s => s.id);
    const resultIds = EPIC_DOMAIN_RESULT_TYPES.map(r => r.id);

    subscaleIds.forEach(id => {
      expect(resultIds).toContain(id);
    });
  });

  // ==========================================================================
  // Instrument Configuration
  // ==========================================================================

  it('should have valid instrument config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG).toBeDefined();
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.id).toBe(EPIC_DOMAIN_INSTRUMENT_ID);
  });

  it('should have name in config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.name).toBe('Epic Domain Discovery');
  });

  it('should have category in config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.category).toBe('vocational');
  });

  it('should have description in config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.description).toBeDefined();
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.description?.length).toBeGreaterThan(0);
  });

  it('should have version in config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.version).toBe('1.0.0');
  });

  it('should have subscales in config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.subscales).toBeDefined();
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.subscales?.length).toBe(5);
  });

  it('should have result types in config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.resultTypes).toBeDefined();
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.resultTypes?.length).toBe(5);
  });

  it('should have scoring config', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.scoringConfig).toBeDefined();
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.scoringConfig?.method).toBe('highest-subscale');
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.scoringConfig?.normalize).toBe(true);
  });

  it('should have interpret function', () => {
    expect(EPIC_DOMAIN_INSTRUMENT_CONFIG.interpret).toBeDefined();
    expect(typeof EPIC_DOMAIN_INSTRUMENT_CONFIG.interpret).toBe('function');
  });

  // ==========================================================================
  // Helper Accessors - getEpicSubscale
  // ==========================================================================

  it('should get subscale by ID', () => {
    const subscale = getEpicSubscale('governance');
    expect(subscale).toBeDefined();
    expect(subscale?.id).toBe('governance');
  });

  it('should return undefined for invalid subscale ID', () => {
    const subscale = getEpicSubscale('invalid-id');
    expect(subscale).toBeUndefined();
  });

  it('should get all subscale types', () => {
    const ids = ['governance', 'care', 'economic', 'public', 'social'];
    ids.forEach(id => {
      expect(getEpicSubscale(id)).toBeDefined();
    });
  });

  // ==========================================================================
  // Helper Accessors - getEpicSubscaleIds
  // ==========================================================================

  it('should get all subscale IDs', () => {
    const ids = getEpicSubscaleIds();
    expect(ids).toBeDefined();
    expect(ids.length).toBe(5);
  });

  it('should contain all expected IDs', () => {
    const ids = getEpicSubscaleIds();
    expect(ids).toContain('governance');
    expect(ids).toContain('care');
    expect(ids).toContain('economic');
    expect(ids).toContain('public');
    expect(ids).toContain('social');
  });

  // ==========================================================================
  // Helper Accessors - getEpicResultType
  // ==========================================================================

  it('should get result type by ID', () => {
    const result = getEpicResultType('care');
    expect(result).toBeDefined();
    expect(result?.id).toBe('care');
    expect(result?.name).toBe('Value Scanner');
  });

  it('should return undefined for invalid result type ID', () => {
    const result = getEpicResultType('non-existent');
    expect(result).toBeUndefined();
  });

  // ==========================================================================
  // Deprecated Helper - findPrimaryEpicDomain
  // ==========================================================================

  it('should find primary epic domain from scores', () => {
    const scores = {
      governance: 45,
      care: 38,
      economic: 42,
      public: 35,
      social: 40,
    };

    const primary = findPrimaryEpicDomain(scores);

    expect(primary).toBeDefined();
    expect(primary?.id).toBe('governance');
    expect(primary?.score).toBe(45);
  });

  it('should include name in primary domain result', () => {
    const scores = {
      governance: 50,
      care: 30,
      economic: 25,
      public: 20,
      social: 15,
    };

    const primary = findPrimaryEpicDomain(scores);

    expect(primary?.name).toBe('AI Constitutional');
  });

  it('should include icon in primary domain result', () => {
    const scores = {
      care: 100,
      governance: 10,
      economic: 10,
      public: 10,
      social: 10,
    };

    const primary = findPrimaryEpicDomain(scores);

    expect(primary?.icon).toBe('💝');
  });

  it('should include color in primary domain result', () => {
    const scores = {
      social: 100,
      governance: 10,
      care: 10,
      economic: 10,
      public: 10,
    };

    const primary = findPrimaryEpicDomain(scores);

    expect(primary?.color).toBeDefined();
    expect(primary?.color).toMatch(/^#[0-9A-F]{6}$/i);
  });

  it('should return null for empty scores', () => {
    const primary = findPrimaryEpicDomain({});
    expect(primary).toBeNull();
  });

  it('should pick first domain when tied scores', () => {
    const scores = {
      governance: 50,
      care: 50,
      economic: 50,
      public: 50,
      social: 50,
    };

    const primary = findPrimaryEpicDomain(scores);

    expect(primary).toBeDefined();
    expect(primary?.score).toBe(50);
  });

  // ==========================================================================
  // Deprecated Helper - sortEpicDomainsByScore
  // ==========================================================================

  it('should sort epic domains by score descending', () => {
    const scores = {
      governance: 45,
      care: 38,
      economic: 42,
      public: 35,
      social: 40,
    };

    const sorted = sortEpicDomainsByScore(scores);

    expect(sorted.length).toBe(5);
    expect(sorted[0].id).toBe('governance');
    expect(sorted[0].score).toBe(45);
    expect(sorted[1].id).toBe('economic');
    expect(sorted[1].score).toBe(42);
  });

  it('should calculate percent for each domain', () => {
    const scores = {
      governance: 25,
      care: 25,
      economic: 25,
      public: 25,
      social: 0,
    };

    const sorted = sortEpicDomainsByScore(scores);

    expect(sorted[0].percent).toBe(25);
    expect(sorted[4].percent).toBe(0);
  });

  it('should include icon for each domain in sorted result', () => {
    const scores = {
      governance: 40,
      care: 30,
      economic: 20,
      public: 5,
      social: 5,
    };

    const sorted = sortEpicDomainsByScore(scores);

    sorted.forEach(domain => {
      expect(domain.icon).toBeDefined();
      expect(domain.icon?.length).toBeGreaterThan(0);
    });
  });

  it('should include color for each domain in sorted result', () => {
    const scores = {
      governance: 40,
      care: 30,
      economic: 20,
      public: 5,
      social: 5,
    };

    const sorted = sortEpicDomainsByScore(scores);

    sorted.forEach(domain => {
      expect(domain.color).toBeDefined();
      expect(domain.color).toMatch(/^#[0-9A-F]{6}$/i);
    });
  });

  it('should handle zero total scores', () => {
    const scores = {
      governance: 0,
      care: 0,
      economic: 0,
      public: 0,
      social: 0,
    };

    const sorted = sortEpicDomainsByScore(scores);

    expect(sorted.length).toBe(5);
    sorted.forEach(domain => {
      expect(domain.percent).toBe(0);
    });
  });

  it('should map all subscales in sorted result', () => {
    const scores = {
      governance: 10,
      care: 20,
      economic: 30,
      public: 40,
      social: 50,
    };

    const sorted = sortEpicDomainsByScore(scores);
    const ids = sorted.map(d => d.id);

    expect(ids).toContain('governance');
    expect(ids).toContain('care');
    expect(ids).toContain('economic');
    expect(ids).toContain('public');
    expect(ids).toContain('social');
  });

  // ==========================================================================
  // Interpretation Function
  // ==========================================================================

  it('should interpret aggregated scores correctly', () => {
    const aggregated = {
      normalizedScores: {
        governance: 0.9,
        care: 0.7,
        economic: 0.6,
        public: 0.5,
        social: 0.4,
      },
      rawScores: {},
      itemCount: 0,
    };

    const result = EPIC_DOMAIN_INSTRUMENT_CONFIG.interpret?.(aggregated as any) as any;

    expect(result).toBeDefined();
    expect(result?.primaryType).toBeDefined();
    expect(result?.primaryType?.typeId).toBe('governance');
  });

  it('should include type name in interpretation result', () => {
    const aggregated = {
      normalizedScores: {
        care: 0.95,
        governance: 0.5,
        economic: 0.4,
        public: 0.3,
        social: 0.2,
      },
      rawScores: {},
      itemCount: 0,
    };

    const result = EPIC_DOMAIN_INSTRUMENT_CONFIG.interpret?.(aggregated as any) as any;

    expect(result?.primaryType?.typeName).toBe('Value Scanner');
  });

  it('should include description in interpretation result', () => {
    const aggregated = {
      normalizedScores: {
        economic: 0.92,
        governance: 0.6,
        care: 0.5,
        public: 0.4,
        social: 0.3,
      },
      rawScores: {},
      itemCount: 0,
    };

    const result = EPIC_DOMAIN_INSTRUMENT_CONFIG.interpret?.(aggregated as any) as any;

    expect(result?.primaryType?.description).toBeDefined();
    expect(result?.primaryType?.description?.length).toBeGreaterThan(0);
  });

  it('should return null for empty normalized scores', () => {
    const aggregated = {
      normalizedScores: {},
      rawScores: {},
      itemCount: 0,
    };

    const result = EPIC_DOMAIN_INSTRUMENT_CONFIG.interpret?.(aggregated as any);

    expect(result).toBeNull();
  });
});
