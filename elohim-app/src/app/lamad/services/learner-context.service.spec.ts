import { TestBed } from '@angular/core/testing';

import { LearnerContextService } from './learner-context.service';
import { ContentMasteryService } from './content-mastery.service';
import { PathAdaptationService } from '../quiz-engine/services/path-adaptation.service';
import { DiscoveryAttestationService } from '../quiz-engine/services/discovery-attestation.service';
import { vi, Mock } from 'vitest';

describe('LearnerContextService', () => {
  let service: LearnerContextService;
  let contentMasterySpy: any;
  let pathAdaptationSpy: any;
  let discoveryAttestationSpy: any;

  beforeEach(() => {
    const contentMasterySpyObj = {
      getMasteryLevelSync: vi.fn(),
    };
    const pathAdaptationSpyObj = {
      getState: vi.fn(),
    };
    const discoveryAttestationSpyObj = {
      getDiscoveryProfile: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        LearnerContextService,
        { provide: ContentMasteryService, useValue: contentMasterySpyObj },
        { provide: PathAdaptationService, useValue: pathAdaptationSpyObj },
        { provide: DiscoveryAttestationService, useValue: discoveryAttestationSpyObj },
      ],
    });

    service = TestBed.inject(LearnerContextService);
    contentMasterySpy = TestBed.inject(ContentMasteryService) as {
      [K in keyof ContentMasteryService]?: Mock;
    };
    pathAdaptationSpy = TestBed.inject(PathAdaptationService) as {
      [K in keyof PathAdaptationService]?: Mock;
    };
    discoveryAttestationSpy = TestBed.inject(DiscoveryAttestationService) as {
      [K in keyof DiscoveryAttestationService]?: Mock;
    };

    // Defaults
    contentMasterySpy.getMasteryLevelSync.mockReturnValue('not_started');
    pathAdaptationSpy.getState.mockReturnValue({
      pathId: 'test-path',
      humanId: 'test-human',
      gates: new Map(),
      recommendations: [],
      completedSections: new Set(),
      skippedSections: new Set(),
      updatedAt: new Date().toISOString(),
    });
    discoveryAttestationSpy.getDiscoveryProfile.mockReturnValue(null);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // isMasteryUnlocked
  // =========================================================================

  describe('isMasteryUnlocked', () => {
    it('should return false for not_started mastery', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('not_started');
      expect(service.isMasteryUnlocked('content-1')).toBe(false);
    });

    it('should return false for seen mastery (below threshold)', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('seen');
      expect(service.isMasteryUnlocked('content-1')).toBe(false);
    });

    it('should return false for remember mastery (below threshold)', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('remember');
      expect(service.isMasteryUnlocked('content-1')).toBe(false);
    });

    it('should return true for understand mastery (at threshold)', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('understand');
      expect(service.isMasteryUnlocked('content-1')).toBe(true);
    });

    it('should return true for apply mastery (above threshold)', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('apply');
      expect(service.isMasteryUnlocked('content-1')).toBe(true);
    });

    it('should return true for create mastery (highest level)', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('create');
      expect(service.isMasteryUnlocked('content-1')).toBe(true);
    });

    it('should return false for undefined resourceId', () => {
      expect(service.isMasteryUnlocked(undefined)).toBe(false);
    });

    it('should return false for empty string resourceId', () => {
      expect(service.isMasteryUnlocked('')).toBe(false);
    });
  });

  // =========================================================================
  // getMasteryAccessReason
  // =========================================================================

  describe('getMasteryAccessReason', () => {
    it('should return "Prior mastery" for unlocked content', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('understand');
      expect(service.getMasteryAccessReason('content-1')).toBe('Prior mastery');
    });

    it('should return null for content below threshold', () => {
      contentMasterySpy.getMasteryLevelSync.mockReturnValue('remember');
      expect(service.getMasteryAccessReason('content-1')).toBeNull();
    });

    it('should return null for undefined resourceId', () => {
      expect(service.getMasteryAccessReason(undefined)).toBeNull();
    });
  });

  // =========================================================================
  // isInSkippedSection
  // =========================================================================

  describe('isInSkippedSection', () => {
    it('should return true when section is in skippedSections', () => {
      pathAdaptationSpy.getState.mockReturnValue({
        pathId: 'test-path',
        humanId: 'test-human',
        gates: new Map(),
        recommendations: [],
        completedSections: new Set(['foundations']),
        skippedSections: new Set(['foundations']),
        updatedAt: new Date().toISOString(),
      });

      expect(service.isInSkippedSection('test-path', 'foundations', 'test-human')).toBe(true);
    });

    it('should return false when section is not in skippedSections', () => {
      expect(service.isInSkippedSection('test-path', 'advanced', 'test-human')).toBe(false);
    });

    it('should return false for undefined sectionId', () => {
      expect(service.isInSkippedSection('test-path', undefined, 'test-human')).toBe(false);
    });

    it('should return false for empty humanId', () => {
      expect(service.isInSkippedSection('test-path', 'foundations', '')).toBe(false);
    });
  });

  // =========================================================================
  // getDiscoveryProfile
  // =========================================================================

  describe('getDiscoveryProfile', () => {
    it('should return null when no assessments completed', () => {
      discoveryAttestationSpy.getDiscoveryProfile.mockReturnValue(null);
      expect(service.getDiscoveryProfile()).toBeNull();
    });

    it('should return profile when assessments exist', () => {
      const mockProfile = {
        byCategory: { personality: [], strengths: [] } as Record<string, unknown[]>,
        featured: [],
        totalAssessments: 2,
        topTraits: [{ framework: 'enneagram', label: '4w5' }],
      };

      discoveryAttestationSpy.getDiscoveryProfile.mockReturnValue(mockProfile as never);
      const result = service.getDiscoveryProfile();
      expect(result).toBeTruthy();
      expect(result!.totalAssessments).toBe(2);
    });
  });
});
