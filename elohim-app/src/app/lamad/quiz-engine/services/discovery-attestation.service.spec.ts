import { TestBed } from '@angular/core/testing';

import type { ReachLevel } from '@app/elohim/models/protocol-core.model';
import type { Attestation } from '@app/imagodei/models/attestations.model';
import type { ResearchConsentScope } from '@app/lamad/models/knowledge-map.model';

import {
  type DiscoveryAssessment,
  type DiscoveryResult,
  type DiscoveryAttestation,
  type DiscoveryFramework,
  type DiscoveryCategory,
  type DiscoveryResultSummary,
  type CliftonStrengthsResult,
  type EnneagramResult,
  type MBTIResult,
  formatDiscoveryResult,
  formatDiscoveryShort,
  getFrameworkDisplayName,
  getCategoryIcon,
  interpolateTemplate,
} from '../models/discovery-assessment.model';
import type { AggregationContext } from '../models/community-insights.model';
import { getDisplayConfigByFramework } from '../instruments/instrument-registry';
// Side-effect import: triggers self-registration of all instruments
import '../instruments/attachment-style.instrument';
import '../instruments/epic-domain.instrument';
import '../instruments/strengths-finder.instrument';
import '../instruments/values-hierarchy.instrument';

import {
  DiscoveryAttestationService,
  VISIBILITY_TO_REACH,
  consentEncompasses,
} from './discovery-attestation.service';

// @coverage: 100% statements, 100% lines, 100% functions, 95.3% branches (2026-02-24)
// Uncovered branches: dead-code fallbacks in getFrameworkDisplayName/getCategoryIcon
// where TypeScript's exhaustive types make the || fallback unreachable.

describe('DiscoveryAttestationService', () => {
  let service: DiscoveryAttestationService;

  // ===========================================================================
  // Fixture Builders
  // ===========================================================================

  const buildResultSummary = (
    overrides: Partial<DiscoveryResultSummary> = {}
  ): DiscoveryResultSummary => ({
    typeId: 'type-4',
    name: 'Individualist',
    score: 0.85,
    ...overrides,
  });

  const buildAssessment = (overrides: Partial<DiscoveryAssessment> = {}): DiscoveryAssessment => ({
    id: 'assessment-enneagram',
    title: 'Enneagram Assessment',
    description: 'Discover your Enneagram type',
    category: 'personality',
    framework: 'enneagram',
    contentNodeId: 'content-enneagram-001',
    estimatedMinutes: 20,
    questions: [],
    scoring: {
      subscales: ['type1', 'type2', 'type3', 'type4'],
      method: 'highest-subscale',
    },
    resultTypes: [],
    allowRetake: true,
    ...overrides,
  });

  const buildResult = (overrides: Partial<DiscoveryResult> = {}): DiscoveryResult => ({
    id: 'discovery-assessment-enneagram-1700000000000',
    assessmentId: 'assessment-enneagram',
    assessmentTitle: 'Enneagram Assessment',
    framework: 'enneagram',
    category: 'personality',
    humanId: 'human-123',
    completedAt: '2026-01-15T10:00:00.000Z',
    primaryType: buildResultSummary(),
    subscaleScores: { type4: 0.85, type5: 0.6 },
    displayString: 'Type 4',
    shortDisplay: '4',
    ...overrides,
  });

  const buildAttestation = (
    overrides: Partial<DiscoveryAttestation> = {}
  ): DiscoveryAttestation => ({
    id: 'attest-discovery-assessment-enneagram-1700000000000',
    humanId: 'human-123',
    type: 'discovery',
    result: buildResult(),
    earnedAt: '2026-01-15T10:00:00.000Z',
    visibility: 'community',
    reach: 'bioregional',
    featured: true,
    displayOrder: 0,
    ...overrides,
  });

  // ===========================================================================
  // Helpers
  // ===========================================================================

  /** Seed localStorage before service construction */
  function seedStorage(results: DiscoveryResult[], attestations: DiscoveryAttestation[]): void {
    localStorage.setItem('elohim:discovery-results', JSON.stringify(results));
    localStorage.setItem('elohim:discovery-attestations', JSON.stringify(attestations));
  }

  /** Create service (must be called after seedStorage if pre-seeded data needed) */
  function createService(): DiscoveryAttestationService {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [DiscoveryAttestationService] });
    return TestBed.inject(DiscoveryAttestationService);
  }

  /** Record a result through the public API using default assessment */
  function recordDefault(
    assessmentOverrides: Partial<DiscoveryAssessment> = {},
    summaryOverrides: Partial<DiscoveryResultSummary> = {},
    subscaleScores: Record<string, number> = { type4: 0.85, type5: 0.6 },
    secondaryTypes?: DiscoveryResultSummary[]
  ): DiscoveryResult {
    return service.recordDiscoveryResult(
      buildAssessment(assessmentOverrides),
      subscaleScores,
      buildResultSummary(summaryOverrides),
      secondaryTypes
    );
  }

  // ===========================================================================
  // Setup
  // ===========================================================================

  beforeEach(() => {
    localStorage.clear();
    TestBed.configureTestingModule({ providers: [DiscoveryAttestationService] });
    service = TestBed.inject(DiscoveryAttestationService);
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ===========================================================================
  // Constructor / loadFromStorage
  // ===========================================================================

  describe('Constructor / loadFromStorage', () => {
    it('should start with empty results and attestations when localStorage is empty', () => {
      expect(service.results()).toEqual([]);
      expect(service.attestations()).toEqual([]);
    });

    it('should load results from localStorage on construction', () => {
      const r1 = buildResult({ id: 'r1', assessmentId: 'a1' });
      const r2 = buildResult({ id: 'r2', assessmentId: 'a2' });
      seedStorage([r1, r2], []);

      const svc = createService();
      expect(svc.results().length).toBe(2);
      expect(svc.results()[0].id).toBe('r1');
      expect(svc.results()[1].id).toBe('r2');
    });

    it('should load attestations from localStorage on construction', () => {
      const a1 = buildAttestation({ id: 'a1' });
      const a2 = buildAttestation({ id: 'a2', result: buildResult({ id: 'r2' }) });
      seedStorage([], [a1, a2]);

      const svc = createService();
      expect(svc.attestations().length).toBe(2);
    });

    it('should handle invalid JSON in localStorage gracefully', () => {
      localStorage.setItem('elohim:discovery-results', 'not-valid-json');
      localStorage.setItem('elohim:discovery-attestations', '{bad');

      const svc = createService();
      expect(svc.results()).toEqual([]);
      expect(svc.attestations()).toEqual([]);
    });

    it('should load attestations when only results key is missing', () => {
      const att = buildAttestation();
      localStorage.setItem('elohim:discovery-attestations', JSON.stringify([att]));

      const svc = createService();
      expect(svc.results()).toEqual([]);
      expect(svc.attestations().length).toBe(1);
    });

    it('should load results when only attestations key is missing', () => {
      const r = buildResult();
      localStorage.setItem('elohim:discovery-results', JSON.stringify([r]));

      const svc = createService();
      expect(svc.results().length).toBe(1);
      expect(svc.attestations()).toEqual([]);
    });

    it('should handle localStorage.getItem throwing', () => {
      const spy = spyOn(Storage.prototype, 'getItem').and.throwError('storage unavailable');
      const svc = createService();
      expect(svc.results()).toEqual([]);
      expect(svc.attestations()).toEqual([]);
      spy.and.callThrough();
    });
  });

  // ===========================================================================
  // recordDiscoveryResult()
  // ===========================================================================

  describe('recordDiscoveryResult()', () => {
    it('should create a DiscoveryResult with correct fields from assessment', () => {
      const result = recordDefault();
      expect(result.assessmentId).toBe('assessment-enneagram');
      expect(result.assessmentTitle).toBe('Enneagram Assessment');
      expect(result.framework).toBe('enneagram');
      expect(result.category).toBe('personality');
    });

    it('should copy contentNodeId from assessment to result', () => {
      const result = recordDefault();
      expect(result.contentNodeId).toBe('content-enneagram-001');
    });

    it('should persist contentNodeId to localStorage', () => {
      recordDefault();
      const stored = JSON.parse(localStorage.getItem('elohim:discovery-results')!);
      expect(stored[0].contentNodeId).toBe('content-enneagram-001');
    });

    it('should generate an ID containing the assessment ID', () => {
      const result = recordDefault();
      expect(result.id).toMatch(/^discovery-assessment-enneagram-\d+$/);
    });

    it('should set completedAt to an ISO date string', () => {
      const result = recordDefault();
      expect(result.completedAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    });

    it('should set subscaleScores from argument', () => {
      const scores = { openness: 23, conservation: 15 };
      const result = recordDefault({}, {}, scores);
      expect(result.subscaleScores).toEqual(scores);
    });

    it('should set primaryType from argument', () => {
      const result = recordDefault({}, { typeId: 'my-type', name: 'My Type', score: 0.9 });
      expect(result.primaryType.typeId).toBe('my-type');
      expect(result.primaryType.name).toBe('My Type');
      expect(result.primaryType.score).toBe(0.9);
    });

    it('should set displayString and shortDisplay (not empty)', () => {
      const result = recordDefault();
      expect(result.displayString).toBeTruthy();
      expect(result.shortDisplay).toBeTruthy();
    });

    it('should use primaryType.name as displayString for default frameworks', () => {
      const result = recordDefault(
        { framework: 'love-languages', category: 'relational' },
        { name: 'Words of Affirmation' }
      );
      expect(result.displayString).toBe('Words of Affirmation');
    });

    it('should get humanId from localStorage session key', () => {
      localStorage.setItem('elohim:session-human-id', 'my-human-id');
      const result = recordDefault();
      expect(result.humanId).toBe('my-human-id');
    });

    it('should fall back to anonymous when no session human ID', () => {
      const result = recordDefault();
      expect(result.humanId).toBe('anonymous');
    });

    it('should include secondaryTypes when provided', () => {
      const secondary = [buildResultSummary({ typeId: 'wing-5', name: 'Wing 5' })];
      const result = recordDefault({}, {}, { type4: 0.85 }, secondary);
      expect(result.secondaryTypes).toBeDefined();
      expect(result.secondaryTypes!.length).toBe(1);
      expect(result.secondaryTypes![0].typeId).toBe('wing-5');
    });

    it('should leave secondaryTypes undefined when not provided', () => {
      const result = recordDefault();
      expect(result.secondaryTypes).toBeUndefined();
    });

    it('should add result to resultsSignal', () => {
      recordDefault();
      expect(service.results().length).toBe(1);
    });

    it('should create a corresponding attestation', () => {
      recordDefault();
      const att = service.attestations()[0];
      expect(att).toBeDefined();
      expect(att.id).toMatch(/^attest-discovery-/);
      expect(att.type).toBe('discovery');
      expect(att.visibility).toBe('community');
      expect(att.earnedAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    });

    it('should return the created result', () => {
      const returned = recordDefault();
      expect(returned).toEqual(service.results()[0]);
    });

    it('should replace existing result for same assessmentId on retake', () => {
      recordDefault({}, { name: 'First attempt' });
      recordDefault({}, { name: 'Second attempt' });

      expect(service.results().length).toBe(1);
      expect(service.results()[0].primaryType.name).toBe('Second attempt');
    });

    it('should replace existing attestation for same assessmentId on retake', () => {
      recordDefault();
      recordDefault();
      expect(service.attestations().length).toBe(1);
    });

    it('should keep results from different assessments', () => {
      recordDefault({ id: 'assessment-a' });
      recordDefault({ id: 'assessment-b' });
      expect(service.results().length).toBe(2);
      expect(service.attestations().length).toBe(2);
    });

    it('should persist to localStorage after recording', () => {
      recordDefault();
      const stored = JSON.parse(localStorage.getItem('elohim:discovery-results')!);
      expect(stored.length).toBe(1);
      const storedAtt = JSON.parse(localStorage.getItem('elohim:discovery-attestations')!);
      expect(storedAtt.length).toBe(1);
    });

    // --- Auto-featuring ---

    describe('auto-featuring', () => {
      // Note: clifton-strengths is also auto-featured but can't be tested via
      // recordDiscoveryResult because formatDiscoveryResult() casts to
      // CliftonStrengthsResult and accesses .signatureThemes which doesn't
      // exist on the plain DiscoveryResult the service creates. This is a
      // latent bug in the format function — tracked separately.
      const autoFeatured: DiscoveryFramework[] = ['enneagram', 'mbti', 'big-five'];
      const notAutoFeatured: DiscoveryFramework[] = [
        'via-strengths', 'disc', 'love-languages', 'attachment-style',
        'learning-style', 'political-compass', 'values-hierarchy', 'custom',
      ];

      for (const fw of autoFeatured) {
        it(`should auto-feature ${fw} results`, () => {
          recordDefault({ id: `assess-${fw}`, framework: fw });
          expect(service.attestations()[0].featured).toBeTrue();
        });
      }

      for (const fw of notAutoFeatured) {
        it(`should NOT auto-feature ${fw} results`, () => {
          recordDefault({ id: `assess-${fw}`, framework: fw });
          expect(service.attestations()[0].featured).toBeFalse();
        });
      }
    });

    // --- Display order ---

    describe('displayOrder', () => {
      it('should assign displayOrder 0 for first attestation', () => {
        recordDefault();
        expect(service.attestations()[0].displayOrder).toBe(0);
      });

      it('should assign displayOrder max+1 for subsequent attestations', () => {
        recordDefault({ id: 'a1' });
        recordDefault({ id: 'a2' });
        expect(service.attestations()[1].displayOrder).toBe(1);
      });

      it('should use max+1 even with non-sequential orders', () => {
        recordDefault({ id: 'a1' });
        service.updateDisplayOrder(service.results()[0].id, 5);
        recordDefault({ id: 'a2' });
        expect(service.attestations()[1].displayOrder).toBe(6);
      });
    });
  });

  // ===========================================================================
  // getResultForAssessment()
  // ===========================================================================

  describe('getResultForAssessment()', () => {
    it('should return undefined when no results exist', () => {
      expect(service.getResultForAssessment('any')).toBeUndefined();
    });

    it('should return the result matching the assessmentId', () => {
      recordDefault({ id: 'my-assessment' });
      const found = service.getResultForAssessment('my-assessment');
      expect(found).toBeDefined();
      expect(found!.assessmentId).toBe('my-assessment');
    });

    it('should return undefined for non-matching assessmentId', () => {
      recordDefault({ id: 'assessment-a' });
      expect(service.getResultForAssessment('assessment-b')).toBeUndefined();
    });
  });

  // ===========================================================================
  // getResultForFramework()
  // ===========================================================================

  describe('getResultForFramework()', () => {
    it('should return undefined when no results exist', () => {
      expect(service.getResultForFramework('enneagram')).toBeUndefined();
    });

    it('should return the result matching the framework', () => {
      recordDefault({ framework: 'mbti' });
      const found = service.getResultForFramework('mbti');
      expect(found).toBeDefined();
      expect(found!.framework).toBe('mbti');
    });

    it('should return undefined for non-matching framework', () => {
      recordDefault({ framework: 'enneagram' });
      expect(service.getResultForFramework('mbti')).toBeUndefined();
    });
  });

  // ===========================================================================
  // hasCompletedAssessment()
  // ===========================================================================

  describe('hasCompletedAssessment()', () => {
    it('should return false when no results exist', () => {
      expect(service.hasCompletedAssessment('any')).toBeFalse();
    });

    it('should return true for a completed assessment', () => {
      recordDefault({ id: 'my-assessment' });
      expect(service.hasCompletedAssessment('my-assessment')).toBeTrue();
    });

    it('should return false for a non-completed assessment', () => {
      recordDefault({ id: 'assessment-a' });
      expect(service.hasCompletedAssessment('assessment-b')).toBeFalse();
    });
  });

  // ===========================================================================
  // updateVisibility()
  // ===========================================================================

  describe('updateVisibility()', () => {
    let resultId: string;

    beforeEach(() => {
      const result = recordDefault();
      resultId = result.id;
    });

    it('should update the attestation visibility field', () => {
      service.updateVisibility(resultId, 'trusted');
      expect(service.attestations()[0].visibility).toBe('trusted');
    });

    it('should update reach to match visibility', () => {
      service.updateVisibility(resultId, 'public');
      expect(service.attestations()[0].reach).toBe('commons' as ReachLevel);

      service.updateVisibility(resultId, 'private');
      expect(service.attestations()[0].reach).toBe('private' as ReachLevel);
    });

    it('should persist to localStorage after update', () => {
      service.updateVisibility(resultId, 'private');
      const stored = JSON.parse(localStorage.getItem('elohim:discovery-attestations')!);
      expect(stored[0].visibility).toBe('private');
      expect(stored[0].reach).toBe('private');
    });

    it('should not modify other attestations when updating one', () => {
      recordDefault({ id: 'assessment-b', framework: 'mbti' });
      service.updateVisibility(resultId, 'private');
      const other = service.attestations().find(a => a.result.assessmentId === 'assessment-b');
      expect(other!.visibility).toBe('community');
      expect(other!.reach).toBe('bioregional' as ReachLevel);
    });
  });

  // ===========================================================================
  // toggleFeatured()
  // ===========================================================================

  describe('toggleFeatured()', () => {
    it('should flip featured from true to false', () => {
      const result = recordDefault({ framework: 'enneagram' }); // auto-featured
      expect(service.attestations()[0].featured).toBeTrue();

      service.toggleFeatured(result.id);
      expect(service.attestations()[0].featured).toBeFalse();
    });

    it('should flip featured from false to true', () => {
      const result = recordDefault({ framework: 'love-languages' }); // not auto-featured
      expect(service.attestations()[0].featured).toBeFalse();

      service.toggleFeatured(result.id);
      expect(service.attestations()[0].featured).toBeTrue();
    });

    it('should persist to localStorage after toggle', () => {
      const result = recordDefault({ framework: 'enneagram' });
      service.toggleFeatured(result.id);
      const stored = JSON.parse(localStorage.getItem('elohim:discovery-attestations')!);
      expect(stored[0].featured).toBeFalse();
    });

    it('should not modify other attestations when toggling one', () => {
      const r1 = recordDefault({ id: 'a1', framework: 'enneagram' });
      recordDefault({ id: 'a2', framework: 'mbti' });

      service.toggleFeatured(r1.id);
      const other = service.attestations().find(a => a.result.assessmentId === 'a2');
      expect(other!.featured).toBeTrue();
    });
  });

  // ===========================================================================
  // updateDisplayOrder()
  // ===========================================================================

  describe('updateDisplayOrder()', () => {
    it('should update displayOrder for matching result', () => {
      const result = recordDefault();
      service.updateDisplayOrder(result.id, 42);
      expect(service.attestations()[0].displayOrder).toBe(42);
    });

    it('should persist to localStorage after update', () => {
      const result = recordDefault();
      service.updateDisplayOrder(result.id, 7);
      const stored = JSON.parse(localStorage.getItem('elohim:discovery-attestations')!);
      expect(stored[0].displayOrder).toBe(7);
    });

    it('should not modify other attestations', () => {
      const r1 = recordDefault({ id: 'a1' });
      recordDefault({ id: 'a2' });

      service.updateDisplayOrder(r1.id, 99);
      const other = service.attestations().find(a => a.result.assessmentId === 'a2');
      expect(other!.displayOrder).toBe(1); // was auto-assigned 1
    });
  });

  // ===========================================================================
  // addReflection()
  // ===========================================================================

  describe('addReflection()', () => {
    let resultId: string;

    beforeEach(() => {
      const result = recordDefault();
      resultId = result.id;
    });

    it('should add reflection to result in resultsSignal', () => {
      service.addReflection(resultId, 'This resonates deeply');
      expect(service.results()[0].reflection).toBe('This resonates deeply');
    });

    it('should add reflection to result in attestationsSignal', () => {
      service.addReflection(resultId, 'This resonates deeply');
      expect(service.attestations()[0].result.reflection).toBe('This resonates deeply');
    });

    it('should keep both signals in sync', () => {
      service.addReflection(resultId, 'Synced reflection');
      expect(service.results()[0].reflection).toBe(service.attestations()[0].result.reflection);
    });

    it('should persist to localStorage after adding reflection', () => {
      service.addReflection(resultId, 'Persisted');
      const stored = JSON.parse(localStorage.getItem('elohim:discovery-results')!);
      expect(stored[0].reflection).toBe('Persisted');
    });

    it('should not modify other results', () => {
      const other = recordDefault({ id: 'assessment-b', framework: 'mbti' });
      service.addReflection(resultId, 'Only mine');
      const otherResult = service.results().find(r => r.id === other.id);
      expect(otherResult!.reflection).toBeUndefined();
    });

    it('should overwrite existing reflection', () => {
      service.addReflection(resultId, 'First thought');
      service.addReflection(resultId, 'Revised thought');
      expect(service.results()[0].reflection).toBe('Revised thought');
      expect(service.attestations()[0].result.reflection).toBe('Revised thought');
    });
  });

  // ===========================================================================
  // toStandardAttestation()
  // ===========================================================================

  describe('toStandardAttestation()', () => {
    let att: DiscoveryAttestation;
    let standard: Attestation;

    beforeEach(() => {
      const result = recordDefault(
        {},
        { name: 'Individualist', shortCode: '4' },
        { type4: 0.85, type5: 0.6 }
      );
      att = service.attestations()[0];
      standard = service.toStandardAttestation(att);
    });

    it('should map id from discoveryAttestation', () => {
      expect(standard.id).toBe(att.id);
    });

    it('should format name as framework display name + display string', () => {
      expect(standard.name).toContain('Enneagram');
      expect(standard.name).toContain(':');
    });

    it('should format description with assessment title', () => {
      expect(standard.description).toContain('Completed');
      expect(standard.description).toContain('Enneagram Assessment');
    });

    it('should set type to discovery', () => {
      expect(standard.type).toBe('discovery');
    });

    it('should set earnedAt from attestation', () => {
      expect(standard.earnedAt).toBe(att.earnedAt);
    });

    it('should set revocable to false', () => {
      expect(standard.revocable).toBeFalse();
    });

    it('should include framework and category in metadata', () => {
      expect(standard.metadata!['framework']).toBe('enneagram');
      expect(standard.metadata!['category']).toBe('personality');
    });

    it('should include displayString and shortDisplay in metadata', () => {
      expect(standard.metadata!['displayString']).toBeTruthy();
      expect(standard.metadata!['shortDisplay']).toBeTruthy();
    });

    it('should include subscaleScores and primaryType in metadata', () => {
      expect(standard.metadata!['subscaleScores']).toEqual({ type4: 0.85, type5: 0.6 });
      expect(standard.metadata!['primaryType']).toBeDefined();
    });

    it('should include category icon in metadata', () => {
      expect(standard.metadata!['icon']).toBe(getCategoryIcon('personality'));
    });

    it('should include secondaryTypes in metadata when present', () => {
      const secondary = [buildResultSummary({ typeId: 'wing-5' })];
      const r = recordDefault({ id: 'assess-with-secondary' }, {}, { type4: 0.85 }, secondary);
      const a = service.attestations().find(x => x.result.id === r.id)!;
      const s = service.toStandardAttestation(a);
      const meta = s.metadata!['secondaryTypes'] as DiscoveryResultSummary[];
      expect(meta.length).toBe(1);
      expect(meta[0].typeId).toBe('wing-5');
    });

    it('should include reflection in metadata when present', () => {
      service.addReflection(att.result.id, 'Deep insight');
      const updated = service.attestations()[0];
      const s = service.toStandardAttestation(updated);
      expect(s.metadata!['reflection']).toBe('Deep insight');
    });
  });

  // ===========================================================================
  // getAsStandardAttestations()
  // ===========================================================================

  describe('getAsStandardAttestations()', () => {
    it('should return empty array when no attestations', () => {
      expect(service.getAsStandardAttestations()).toEqual([]);
    });

    it('should map all attestations to standard format', () => {
      recordDefault({ id: 'a1' });
      recordDefault({ id: 'a2', framework: 'mbti' });
      const standards = service.getAsStandardAttestations();
      expect(standards.length).toBe(2);
    });

    it('should return Attestation-compatible objects', () => {
      recordDefault();
      const standards = service.getAsStandardAttestations();
      const s = standards[0];
      expect(s.id).toBeDefined();
      expect(s.name).toBeDefined();
      expect(s.description).toBeDefined();
      expect(s.type).toBe('discovery');
      expect(s.earnedAt).toBeDefined();
      expect(s.revocable).toBeFalse();
    });
  });

  // ===========================================================================
  // clearAll()
  // ===========================================================================

  describe('clearAll()', () => {
    beforeEach(() => {
      recordDefault();
    });

    it('should clear all results', () => {
      expect(service.results().length).toBe(1);
      service.clearAll();
      expect(service.results()).toEqual([]);
    });

    it('should clear all attestations', () => {
      expect(service.attestations().length).toBe(1);
      service.clearAll();
      expect(service.attestations()).toEqual([]);
    });

    it('should remove from localStorage', () => {
      expect(localStorage.getItem('elohim:discovery-results')).not.toBeNull();
      service.clearAll();
      expect(localStorage.getItem('elohim:discovery-results')).toBeNull();
      expect(localStorage.getItem('elohim:discovery-attestations')).toBeNull();
    });
  });

  // ===========================================================================
  // Computed Signals
  // ===========================================================================

  describe('Computed Signals', () => {
    // -------------------------------------------------------------------------
    // resultsByCategory
    // -------------------------------------------------------------------------

    describe('resultsByCategory', () => {
      it('should return empty category buckets when no results', () => {
        const grouped = service.resultsByCategory();
        expect(grouped['personality']).toEqual([]);
        expect(grouped['strengths']).toEqual([]);
        expect(grouped['values']).toEqual([]);
        expect(grouped['learning']).toEqual([]);
        expect(grouped['emotional']).toEqual([]);
        expect(grouped['relational']).toEqual([]);
        expect(grouped['vocational']).toEqual([]);
        expect(grouped['spiritual']).toEqual([]);
      });

      it('should group results by category', () => {
        recordDefault({ id: 'a1', category: 'personality' });
        recordDefault({ id: 'a2', category: 'strengths', framework: 'via-strengths' });
        recordDefault({ id: 'a3', category: 'values', framework: 'values-hierarchy' });

        const grouped = service.resultsByCategory();
        expect(grouped['personality'].length).toBe(1);
        expect(grouped['strengths'].length).toBe(1);
        expect(grouped['values'].length).toBe(1);
        expect(grouped['learning'].length).toBe(0);
      });

      it('should contain all 8 category keys', () => {
        const grouped = service.resultsByCategory();
        const keys = Object.keys(grouped);
        expect(keys).toContain('personality');
        expect(keys).toContain('strengths');
        expect(keys).toContain('values');
        expect(keys).toContain('learning');
        expect(keys).toContain('emotional');
        expect(keys).toContain('relational');
        expect(keys).toContain('vocational');
        expect(keys).toContain('spiritual');
        expect(keys.length).toBe(8);
      });
    });

    // -------------------------------------------------------------------------
    // resultsByFramework
    // -------------------------------------------------------------------------

    describe('resultsByFramework', () => {
      it('should return empty Map when no results', () => {
        expect(service.resultsByFramework().size).toBe(0);
      });

      it('should group results by framework', () => {
        recordDefault({ id: 'a1', framework: 'enneagram' });
        recordDefault({ id: 'a2', framework: 'mbti' });

        const grouped = service.resultsByFramework();
        expect(grouped.size).toBe(2);
        expect(grouped.get('enneagram')!.length).toBe(1);
        expect(grouped.get('mbti')!.length).toBe(1);
      });

      it('should accumulate multiple results for same framework', () => {
        recordDefault({ id: 'a1', framework: 'enneagram' });
        recordDefault({ id: 'a2', framework: 'enneagram' });

        const grouped = service.resultsByFramework();
        expect(grouped.get('enneagram')!.length).toBe(2);
      });
    });

    // -------------------------------------------------------------------------
    // featuredResults
    // -------------------------------------------------------------------------

    describe('featuredResults', () => {
      it('should return empty array when no attestations', () => {
        expect(service.featuredResults()).toEqual([]);
      });

      it('should return only featured attestation results', () => {
        recordDefault({ id: 'a1', framework: 'enneagram' }); // auto-featured
        recordDefault({ id: 'a2', framework: 'love-languages' }); // not featured

        const featured = service.featuredResults();
        expect(featured.length).toBe(1);
        expect(featured[0].framework).toBe('enneagram');
      });

      it('should sort by displayOrder ascending', () => {
        recordDefault({ id: 'a1', framework: 'enneagram' }); // order 0
        recordDefault({ id: 'a2', framework: 'mbti' }); // order 1
        recordDefault({ id: 'a3', framework: 'big-five' }); // order 2

        // Reorder: big-five first, enneagram last
        const ids = service.results().map(r => r.id);
        service.updateDisplayOrder(ids[2], 0); // big-five → 0
        service.updateDisplayOrder(ids[0], 5); // enneagram → 5
        service.updateDisplayOrder(ids[1], 3); // mbti → 3

        const featured = service.featuredResults();
        expect(featured.length).toBe(3);
        expect(featured[0].framework).toBe('big-five');
        expect(featured[1].framework).toBe('mbti');
        expect(featured[2].framework).toBe('enneagram');
      });

      it('should reflect toggleFeatured changes', () => {
        const result = recordDefault({ framework: 'enneagram' }); // auto-featured
        expect(service.featuredResults().length).toBe(1);

        service.toggleFeatured(result.id);
        expect(service.featuredResults().length).toBe(0);
      });
    });

  });

  // ===========================================================================
  // Display Helpers
  // ===========================================================================

  describe('Display Helpers', () => {
    // -------------------------------------------------------------------------
    // getDisplayCard()
    // -------------------------------------------------------------------------

    describe('getDisplayCard()', () => {
      it('should return icon from getCategoryIcon', () => {
        const result = recordDefault({ category: 'personality' });
        const card = service.getDisplayCard(result);
        expect(card.icon).toBe(getCategoryIcon('personality'));
      });

      it('should return framework display name', () => {
        const result = recordDefault({ framework: 'enneagram' });
        const card = service.getDisplayCard(result);
        expect(card.framework).toBe('Enneagram');
      });

      it('should return displayString as title', () => {
        const result = recordDefault();
        const card = service.getDisplayCard(result);
        expect(card.title).toBe(result.displayString);
      });

      it('should return shortDisplay as shortCode', () => {
        const result = recordDefault();
        const card = service.getDisplayCard(result);
        expect(card.shortCode).toBe(result.shortDisplay);
      });

      it('should return category string', () => {
        const result = recordDefault({ category: 'values' });
        const card = service.getDisplayCard(result);
        expect(card.category).toBe('values');
      });

      it('should return formatted completedAt date (not raw ISO)', () => {
        const result = recordDefault();
        const card = service.getDisplayCard(result);
        // Should be a locale date string, not the raw ISO
        expect(card.completedAt).not.toContain('T');
        expect(card.completedAt).toBeTruthy();
      });
    });

    // -------------------------------------------------------------------------
    // getBadgeDisplay()
    // -------------------------------------------------------------------------

    describe('getBadgeDisplay()', () => {
      it('should return shortDisplay as label', () => {
        const result = recordDefault();
        const badge = service.getBadgeDisplay(result);
        expect(badge.label).toBe(result.shortDisplay);
      });

      it('should return category icon', () => {
        const result = recordDefault({ category: 'personality' });
        const badge = service.getBadgeDisplay(result);
        expect(badge.icon).toBe(getCategoryIcon('personality'));
      });

      it('should use primaryType.color when available', () => {
        const result = recordDefault({}, { color: '#FF0000' });
        const badge = service.getBadgeDisplay(result);
        expect(badge.color).toBe('#FF0000');
      });

      it('should fall back to category color when primaryType has no color', () => {
        const result = recordDefault({ category: 'personality' });
        const badge = service.getBadgeDisplay(result);
        expect(badge.color).toBe('#8B5CF6'); // personality = purple
      });

      it('should return correct color for each category', () => {
        const expected: Record<DiscoveryCategory, string> = {
          personality: '#8B5CF6',
          strengths: '#10B981',
          values: '#F59E0B',
          learning: '#3B82F6',
          emotional: '#EC4899',
          relational: '#F97316',
          vocational: '#6366F1',
          spiritual: '#14B8A6',
        };

        for (const [category, color] of Object.entries(expected)) {
          const result = buildResult({
            category: category as DiscoveryCategory,
            primaryType: buildResultSummary(), // no color
          });
          const badge = service.getBadgeDisplay(result);
          expect(badge.color).toBe(color, `Expected ${color} for ${category}`);
        }
      });
    });
  });

  // ===========================================================================
  // Storage Error Handling
  // ===========================================================================

  describe('Storage Error Handling', () => {
    it('should handle localStorage write failure silently', () => {
      const spy = spyOn(Storage.prototype, 'setItem').and.throwError('quota exceeded');
      expect(() => recordDefault()).not.toThrow();
      // The result should still be in the signal even if storage failed
      expect(service.results().length).toBe(1);
      spy.and.callThrough();
    });

    it('should handle localStorage read failure on construction silently', () => {
      const spy = spyOn(Storage.prototype, 'getItem').and.throwError('storage unavailable');
      const svc = createService();
      expect(svc.results()).toEqual([]);
      expect(svc.attestations()).toEqual([]);
      spy.and.callThrough();
    });
  });

  // ===========================================================================
  // Model Format Functions (direct tests for framework-specific branches)
  // ===========================================================================

  describe('Model Format Functions', () => {
    describe('formatDiscoveryResult()', () => {
      it('should format enneagram with core type and wing', () => {
        const result = buildResult({ framework: 'enneagram' }) as EnneagramResult;
        result.coreType = 4;
        result.wing = 'w5';
        expect(formatDiscoveryResult(result)).toBe('Type 4w5');
      });

      it('should format enneagram with instinct variant', () => {
        const result = buildResult({ framework: 'enneagram' }) as EnneagramResult;
        result.coreType = 9;
        result.wing = 'w1';
        result.instinct = 'sp';
        expect(formatDiscoveryResult(result)).toBe('Type 9w1 (sp)');
      });

      it('should format enneagram without wing or instinct', () => {
        const result = buildResult({ framework: 'enneagram' }) as EnneagramResult;
        result.coreType = 7;
        expect(formatDiscoveryResult(result)).toBe('Type 7');
      });

      it('should format mbti with type code', () => {
        const result = buildResult({ framework: 'mbti' }) as MBTIResult;
        result.typeCode = 'INFP';
        expect(formatDiscoveryResult(result)).toBe('INFP');
      });

      it('should format clifton-strengths with top 5 themes joined', () => {
        const result = buildResult({ framework: 'clifton-strengths' }) as CliftonStrengthsResult;
        result.signatureThemes = [
          { name: 'Belief', domain: 'executing', rank: 1 },
          { name: 'Learner', domain: 'strategic', rank: 2 },
          { name: 'Analytical', domain: 'strategic', rank: 3 },
          { name: 'Responsibility', domain: 'executing', rank: 4 },
          { name: 'Restorative', domain: 'executing', rank: 5 },
        ];
        result.domains = { executing: 3, influencing: 0, relationship: 0, strategic: 2 };
        expect(formatDiscoveryResult(result)).toBe(
          'Belief \u2022 Learner \u2022 Analytical \u2022 Responsibility \u2022 Restorative'
        );
      });

      it('should use primaryType.name for unknown frameworks', () => {
        const result = buildResult({
          framework: 'custom',
          primaryType: buildResultSummary({ name: 'My Custom Type' }),
        });
        expect(formatDiscoveryResult(result)).toBe('My Custom Type');
      });
    });

    describe('formatDiscoveryShort()', () => {
      it('should format enneagram short with core type and wing', () => {
        const result = buildResult({ framework: 'enneagram' }) as EnneagramResult;
        result.coreType = 4;
        result.wing = 'w5';
        expect(formatDiscoveryShort(result)).toBe('4w5');
      });

      it('should format enneagram short without wing', () => {
        const result = buildResult({ framework: 'enneagram' }) as EnneagramResult;
        result.coreType = 7;
        expect(formatDiscoveryShort(result)).toBe('7');
      });

      it('should format mbti short with type code', () => {
        const result = buildResult({ framework: 'mbti' }) as MBTIResult;
        result.typeCode = 'ISTP';
        expect(formatDiscoveryShort(result)).toBe('ISTP');
      });

      it('should format clifton-strengths short with 3-char abbreviations', () => {
        const result = buildResult({ framework: 'clifton-strengths' }) as CliftonStrengthsResult;
        result.signatureThemes = [
          { name: 'Belief', domain: 'executing', rank: 1 },
          { name: 'Learner', domain: 'strategic', rank: 2 },
          { name: 'Analytical', domain: 'strategic', rank: 3 },
        ];
        result.domains = { executing: 1, influencing: 0, relationship: 0, strategic: 2 };
        expect(formatDiscoveryShort(result)).toBe('Bel\u00b7Lea\u00b7Ana');
      });

      it('should use shortCode for unknown frameworks when available', () => {
        const result = buildResult({
          framework: 'custom',
          primaryType: buildResultSummary({ name: 'Custom', shortCode: 'CUS' }),
        });
        expect(formatDiscoveryShort(result)).toBe('CUS');
      });

      it('should use name substring for unknown frameworks without shortCode', () => {
        const result = buildResult({
          framework: 'custom',
          primaryType: buildResultSummary({ name: 'Individualist' }),
        });
        expect(formatDiscoveryShort(result)).toBe('Indi');
      });
    });

    describe('Registry-backed display (data-driven instruments)', () => {
      it('should use displayTemplate from registry for attachment-style framework', () => {
        const result = buildResult({
          framework: 'attachment-style',
          primaryType: buildResultSummary({ name: 'Secure' }),
        });
        expect(formatDiscoveryResult(result)).toBe('Secure');
      });

      it('should use shortTemplate from registry for attachment-style framework', () => {
        const result = buildResult({
          framework: 'attachment-style',
          primaryType: buildResultSummary({ name: 'Secure', shortCode: 'SEC' }),
        });
        expect(formatDiscoveryShort(result)).toBe('SEC');
      });

      it('should use shortTemplate fallback when shortCode is undefined', () => {
        const result = buildResult({
          framework: 'values-hierarchy',
          primaryType: buildResultSummary({ name: 'Openness to Change' }),
        });
        // shortTemplate is {{primaryType.shortCode}} which resolves to empty string,
        // so falls back to switch default
        expect(formatDiscoveryShort(result)).toBe('Open');
      });

      it('should use registry frameworkDisplayName for registered instruments', () => {
        expect(getFrameworkDisplayName('attachment-style')).toBe('Attachment Style');
        expect(getFrameworkDisplayName('values-hierarchy')).toBe('Values Hierarchy');
        expect(getFrameworkDisplayName('via-strengths')).toBe('VIA Character Strengths');
      });

      it('should fall back to Record lookup for unregistered frameworks', () => {
        expect(getFrameworkDisplayName('enneagram')).toBe('Enneagram');
        expect(getFrameworkDisplayName('mbti')).toBe('MBTI');
        expect(getFrameworkDisplayName('big-five')).toBe('Big Five');
      });
    });
  });

  // ===========================================================================
  // Template Interpolation Engine
  // ===========================================================================

  describe('interpolateTemplate()', () => {
    it('should interpolate simple field', () => {
      expect(interpolateTemplate('Hello {{name}}', { name: 'World' })).toBe('Hello World');
    });

    it('should interpolate dot-path fields', () => {
      const data = { primaryType: { name: 'Secure', shortCode: 'SEC' } };
      expect(interpolateTemplate('{{primaryType.name}}', data)).toBe('Secure');
      expect(interpolateTemplate('{{primaryType.shortCode}}', data)).toBe('SEC');
    });

    it('should replace missing fields with empty string', () => {
      expect(interpolateTemplate('{{missing}}', {})).toBe('');
    });

    it('should replace nested missing fields with empty string', () => {
      expect(interpolateTemplate('{{a.b.c}}', { a: { b: {} } })).toBe('');
    });

    it('should handle null in nested path gracefully', () => {
      expect(interpolateTemplate('{{a.b}}', { a: null })).toBe('');
    });

    it('should trim result', () => {
      expect(interpolateTemplate('  {{name}}  ', { name: 'test' })).toBe('test');
    });

    it('should interpolate multiple placeholders', () => {
      expect(interpolateTemplate('{{a}} and {{b}}', { a: 'X', b: 'Y' })).toBe('X and Y');
    });

    it('should convert numbers to strings', () => {
      expect(interpolateTemplate('Type {{coreType}}', { coreType: 4 })).toBe('Type 4');
    });

    it('should return template text when no placeholders', () => {
      expect(interpolateTemplate('plain text', {})).toBe('plain text');
    });
  });

  // ===========================================================================
  // Display Config Registry
  // ===========================================================================

  describe('getDisplayConfigByFramework()', () => {
    it('should return display config for registered framework', () => {
      const config = getDisplayConfigByFramework('attachment-style');
      expect(config).toBeDefined();
      expect(config!.frameworkDisplayName).toBe('Attachment Style');
      expect(config!.autoFeature).toBe(false);
    });

    it('should return display config for via-strengths', () => {
      const config = getDisplayConfigByFramework('via-strengths');
      expect(config).toBeDefined();
      expect(config!.frameworkDisplayName).toBe('VIA Character Strengths');
    });

    it('should return undefined for unregistered framework', () => {
      expect(getDisplayConfigByFramework('enneagram')).toBeUndefined();
      expect(getDisplayConfigByFramework('mbti')).toBeUndefined();
    });

    it('should return display config for values-hierarchy', () => {
      const config = getDisplayConfigByFramework('values-hierarchy');
      expect(config).toBeDefined();
      expect(config!.frameworkDisplayName).toBe('Values Hierarchy');
    });

    it('should return display config for epic-domain', () => {
      const config = getDisplayConfigByFramework('epic-domain');
      expect(config).toBeDefined();
      expect(config!.frameworkDisplayName).toBe('Epic Domain');
      expect(config!.autoFeature).toBe(false);
    });
  });

  // ===========================================================================
  // shouldAutoFeature registry integration
  // ===========================================================================

  describe('shouldAutoFeature (registry integration)', () => {
    it('should read autoFeature=false from registered instrument', () => {
      // attachment-style has autoFeature: false in its displayConfig
      const assessment = buildAssessment({ framework: 'attachment-style' });
      const result = service.recordDiscoveryResult(
        assessment,
        { anxiety: 5, avoidance: 3 },
        buildResultSummary({ name: 'Secure' })
      );
      const attestation = service.attestations().find(a => a.result.id === result.id);
      expect(attestation!.featured).toBe(false);
    });

    it('should read autoFeature=false from via-strengths registered instrument', () => {
      const assessment = buildAssessment({ id: 'via-test', framework: 'via-strengths' });
      const result = service.recordDiscoveryResult(
        assessment,
        { creativity: 10 },
        buildResultSummary({ name: 'Creativity' })
      );
      const attestation = service.attestations().find(a => a.result.id === result.id);
      expect(attestation!.featured).toBe(false);
    });

    it('should fall back to hardcoded list for unregistered framework', () => {
      // 'enneagram' is not registered but is in the hardcoded autoFeatured list
      const assessment = buildAssessment({ framework: 'enneagram' });
      const result = service.recordDiscoveryResult(
        assessment,
        { type: 4 },
        buildResultSummary({ name: 'Individualist' })
      );
      const attestation = service.attestations().find(a => a.result.id === result.id);
      expect(attestation!.featured).toBe(true);
    });
  });

  // ===========================================================================
  // Open String Types (Step 2: unknown frameworks/categories flow through)
  // ===========================================================================

  describe('Open string type support', () => {
    it('should record a result with an unknown framework', () => {
      const assessment = buildAssessment({
        id: 'community-poll-1',
        framework: 'community-resilience-index',
        category: 'relational',
      });
      const result = service.recordDiscoveryResult(
        assessment,
        { trust: 8, reciprocity: 6 },
        buildResultSummary({ name: 'High Trust' })
      );
      expect(result.framework).toBe('community-resilience-index');
      expect(result.displayString).toBe('High Trust');
    });

    it('should record a result with an unknown category', () => {
      const assessment = buildAssessment({
        id: 'civic-engagement-1',
        framework: 'civic-engagement',
        category: 'civic' as DiscoveryCategory,
      });
      const result = service.recordDiscoveryResult(
        assessment,
        { participation: 9 },
        buildResultSummary({ name: 'Active Citizen' })
      );
      expect(result.category).toBe('civic');
    });

    it('should group unknown categories in resultsByCategory dynamically', () => {
      const assessment = buildAssessment({
        id: 'wellness-1',
        framework: 'wellness-index',
        category: 'wellness' as DiscoveryCategory,
      });
      service.recordDiscoveryResult(
        assessment,
        { physical: 7 },
        buildResultSummary({ name: 'Balanced' })
      );
      const grouped = service.resultsByCategory();
      expect(grouped['wellness']).toBeDefined();
      expect(grouped['wellness'].length).toBe(1);
    });

    it('should return framework string for getFrameworkDisplayName with unknown framework', () => {
      expect(getFrameworkDisplayName('community-resilience-index')).toBe(
        'community-resilience-index'
      );
    });

    it('should return fallback icon for getCategoryIcon with unknown category', () => {
      expect(getCategoryIcon('civic')).toBe('🔍');
    });

    it('should not auto-feature unknown frameworks', () => {
      const assessment = buildAssessment({
        id: 'custom-survey-1',
        framework: 'custom-community-survey',
      });
      const result = service.recordDiscoveryResult(
        assessment,
        { engagement: 5 },
        buildResultSummary({ name: 'Engaged' })
      );
      const attestation = service.attestations().find(a => a.result.id === result.id);
      expect(attestation!.featured).toBe(false);
    });
  });

  // ===========================================================================
  // VISIBILITY_TO_REACH mapping
  // ===========================================================================

  describe('VISIBILITY_TO_REACH', () => {
    it('should map private to private', () => {
      expect(VISIBILITY_TO_REACH['private']).toBe('private' as ReachLevel);
    });

    it('should map trusted to invited', () => {
      expect(VISIBILITY_TO_REACH['trusted']).toBe('invited' as ReachLevel);
    });

    it('should map community to bioregional', () => {
      expect(VISIBILITY_TO_REACH['community']).toBe('bioregional' as ReachLevel);
    });

    it('should map public to commons', () => {
      expect(VISIBILITY_TO_REACH['public']).toBe('commons' as ReachLevel);
    });
  });

  // ===========================================================================
  // consentEncompasses()
  // ===========================================================================

  describe('consentEncompasses()', () => {
    const levels: ResearchConsentScope[] = ['none', 'aggregate-only', 'anonymized', 'identifiable'];

    it('should return true when scope equals required', () => {
      for (const level of levels) {
        expect(consentEncompasses(level, level)).toBeTrue();
      }
    });

    it('should return true when scope exceeds required', () => {
      expect(consentEncompasses('identifiable', 'none')).toBeTrue();
      expect(consentEncompasses('anonymized', 'aggregate-only')).toBeTrue();
      expect(consentEncompasses('aggregate-only', 'none')).toBeTrue();
    });

    it('should return false when scope is below required', () => {
      expect(consentEncompasses('none', 'aggregate-only')).toBeFalse();
      expect(consentEncompasses('aggregate-only', 'anonymized')).toBeFalse();
      expect(consentEncompasses('anonymized', 'identifiable')).toBeFalse();
    });

    it('should enforce full ordering: none < aggregate-only < anonymized < identifiable', () => {
      for (let i = 0; i < levels.length; i++) {
        for (let j = 0; j < levels.length; j++) {
          const result = consentEncompasses(levels[i], levels[j]);
          expect(result).toBe(i >= j, `consentEncompasses(${levels[i]}, ${levels[j]})`);
        }
      }
    });
  });

  // ===========================================================================
  // getResultsForContext()
  // ===========================================================================

  describe('getResultsForContext()', () => {
    it('should return results that meet reach and consent requirements', () => {
      recordDefault(); // default: visibility=community, reach=bioregional
      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'aggregate-only',
      };
      const results = service.getResultsForContext(context);
      expect(results.length).toBe(1);
    });

    it('should exclude results below required reach', () => {
      const result = recordDefault();
      service.updateVisibility(result.id, 'trusted'); // reach becomes 'invited'
      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'aggregate-only',
      };
      expect(service.getResultsForContext(context).length).toBe(0);
    });

    it('should include results at higher reach than required', () => {
      const result = recordDefault();
      service.updateVisibility(result.id, 'public'); // reach becomes 'commons'
      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'aggregate-only',
      };
      expect(service.getResultsForContext(context).length).toBe(1);
    });

    it('should exclude results with insufficient consent', () => {
      recordDefault(); // no researchConsent override, defaults to aggregate-only
      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'anonymized', // requires higher consent
      };
      expect(service.getResultsForContext(context).length).toBe(0);
    });

    it('should respect per-result researchConsent override', () => {
      // Seed a result with explicit researchConsent
      const r = buildResult({ researchConsent: 'anonymized' });
      const a = buildAttestation({ result: r, reach: 'bioregional' });
      seedStorage([r], [a]);
      const svc = createService();

      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'anonymized',
      };
      expect(svc.getResultsForContext(context).length).toBe(1);
    });

    it('should exclude results with researchConsent none', () => {
      const r = buildResult({ researchConsent: 'none' });
      const a = buildAttestation({ result: r, reach: 'commons' });
      seedStorage([r], [a]);
      const svc = createService();

      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'aggregate-only',
      };
      expect(svc.getResultsForContext(context).length).toBe(0);
    });

    it('should filter by excluded domains', () => {
      recordDefault({ category: 'personality' });
      recordDefault({ id: 'a2', category: 'strengths', framework: 'via-strengths' });

      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'aggregate-only',
        excludedDomains: ['personality'],
      };
      const results = service.getResultsForContext(context);
      expect(results.length).toBe(1);
      expect(results[0].category).toBe('strengths');
    });

    it('should return empty when all results are private', () => {
      const result = recordDefault();
      service.updateVisibility(result.id, 'private');
      const context: AggregationContext = {
        requiredReach: 'bioregional',
        minimumConsent: 'aggregate-only',
      };
      expect(service.getResultsForContext(context).length).toBe(0);
    });
  });

  // ===========================================================================
  // aggregatableResults signal
  // ===========================================================================

  describe('aggregatableResults', () => {
    it('should include community-visibility results', () => {
      recordDefault(); // default visibility: community -> reach: bioregional
      expect(service.aggregatableResults().length).toBe(1);
    });

    it('should include public-visibility results', () => {
      const result = recordDefault();
      service.updateVisibility(result.id, 'public');
      expect(service.aggregatableResults().length).toBe(1);
    });

    it('should exclude private-visibility results', () => {
      const result = recordDefault();
      service.updateVisibility(result.id, 'private');
      expect(service.aggregatableResults().length).toBe(0);
    });

    it('should exclude trusted-visibility results', () => {
      const result = recordDefault();
      service.updateVisibility(result.id, 'trusted');
      expect(service.aggregatableResults().length).toBe(0);
    });
  });

  // ===========================================================================
  // Reach field on attestations
  // ===========================================================================

  describe('reach field', () => {
    it('should set reach to bioregional on new attestation (default community visibility)', () => {
      recordDefault();
      expect(service.attestations()[0].reach).toBe('bioregional' as ReachLevel);
    });

    it('should update reach when updateVisibility is called', () => {
      const result = recordDefault();
      service.updateVisibility(result.id, 'public');
      expect(service.attestations()[0].reach).toBe('commons' as ReachLevel);

      service.updateVisibility(result.id, 'trusted');
      expect(service.attestations()[0].reach).toBe('invited' as ReachLevel);

      service.updateVisibility(result.id, 'private');
      expect(service.attestations()[0].reach).toBe('private' as ReachLevel);
    });
  });

  // ===========================================================================
  // Backward compatibility: loadFromStorage without reach
  // ===========================================================================

  describe('backward compatibility', () => {
    it('should derive reach from visibility for old attestations without reach', () => {
      // Simulate old data without reach field
      const oldAttestation = {
        id: 'attest-old',
        humanId: 'human-123',
        type: 'discovery',
        result: buildResult(),
        earnedAt: '2026-01-15T10:00:00.000Z',
        visibility: 'public',
        featured: true,
        displayOrder: 0,
        // No reach field!
      };
      localStorage.setItem('elohim:discovery-results', JSON.stringify([buildResult()]));
      localStorage.setItem('elohim:discovery-attestations', JSON.stringify([oldAttestation]));

      const svc = createService();
      expect(svc.attestations()[0].reach).toBe('commons' as ReachLevel);
    });

    it('should preserve reach for attestations that already have it', () => {
      const att = buildAttestation({ reach: 'municipal' as ReachLevel });
      seedStorage([buildResult()], [att]);

      const svc = createService();
      expect(svc.attestations()[0].reach).toBe('municipal' as ReachLevel);
    });

    it('should derive bioregional reach for old community-visibility attestations', () => {
      const oldAttestation = {
        id: 'attest-old-community',
        humanId: 'human-123',
        type: 'discovery',
        result: buildResult(),
        earnedAt: '2026-01-15T10:00:00.000Z',
        visibility: 'community',
        featured: false,
        displayOrder: 0,
      };
      localStorage.setItem('elohim:discovery-results', JSON.stringify([buildResult()]));
      localStorage.setItem('elohim:discovery-attestations', JSON.stringify([oldAttestation]));

      const svc = createService();
      expect(svc.attestations()[0].reach).toBe('bioregional' as ReachLevel);
    });
  });
});
