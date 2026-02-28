import { TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { LearnerContextService } from './learner-context.service';
import { PathRecommendationService } from './path-recommendation.service';

import type { DiscoveryProfile, DiscoveryResult } from '../quiz-engine/models/discovery-assessment.model';
import type { PathIndex, PathIndexEntry } from '../models/learning-path.model';
import { vi } from 'vitest';

describe('PathRecommendationService', () => {
  let service: PathRecommendationService;
  let mockDataLoader: any;
  let mockLearnerContext: any;

  const buildPathEntry = (overrides: Partial<PathIndexEntry> = {}): PathIndexEntry => ({
    id: 'test-path',
    title: 'Test Path',
    description: 'A test path',
    difficulty: 'beginner',
    estimatedDuration: '1 hour',
    stepCount: 5,
    tags: ['test'],
    ...overrides,
  });

  const buildPathIndex = (paths: PathIndexEntry[]): PathIndex => ({
    lastUpdated: new Date().toISOString(),
    totalCount: paths.length,
    paths,
  });

  const buildResult = (overrides: Partial<DiscoveryResult> = {}): DiscoveryResult => ({
    id: 'result-1',
    assessmentId: 'assessment-1',
    assessmentTitle: 'Test Assessment',
    framework: 'values-hierarchy',
    category: 'values',
    humanId: 'user-1',
    completedAt: new Date().toISOString(),
    primaryType: { typeId: 'type-1', name: 'Growth', score: 0.8 },
    subscaleScores: { growth: 0.8 },
    displayString: 'Growth',
    shortDisplay: 'Gro',
    ...overrides,
  });

  const buildProfile = (overrides: Partial<DiscoveryProfile> = {}): DiscoveryProfile => ({
    byCategory: {},
    featured: [],
    totalAssessments: 0,
    topTraits: [],
    ...overrides,
  });

  beforeEach(() => {
    mockDataLoader = {
    getPathIndex: vi.fn(),
  };
    mockLearnerContext = {
    getDiscoveryProfile: vi.fn(),
  };

    TestBed.configureTestingModule({
      providers: [
        PathRecommendationService,
        { provide: DataLoaderService, useValue: mockDataLoader },
        { provide: LearnerContextService, useValue: mockLearnerContext },
      ],
    });

    service = TestBed.inject(PathRecommendationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // Empty / no profile cases
  // =========================================================================

  describe('empty profile', () => {
    it('should return empty when totalAssessments is 0', () => new Promise<void>(done => {
      const profile = buildProfile({ totalAssessments: 0 });

      service.getRecommendations(profile).subscribe(recs => {
        expect(recs).toEqual([]);
        done();
      });
    }));

    it('should not call dataLoader when profile has no assessments', () => new Promise<void>(done => {
      const profile = buildProfile({ totalAssessments: 0 });

      service.getRecommendations(profile).subscribe(() => {
        expect(mockDataLoader.getPathIndex).not.toHaveBeenCalled();
        done();
      });
    }));
  });

  // =========================================================================
  // Tag overlap scoring
  // =========================================================================

  describe('tag overlap scoring', () => {
    it('should score paths by category tag overlap', () => new Promise<void>(done => {
      const knowThyself = buildPathEntry({
        id: 'know-thyself-path',
        title: 'Know Thyself',
        tags: ['self-knowledge', 'discovery', 'personality', 'values', 'strengths', 'emotional'],
      });
      const unrelated = buildPathEntry({
        id: 'governance',
        title: 'Governance',
        tags: ['governance', 'policy', 'ai-oversight'],
      });

      mockDataLoader.getPathIndex.mockReturnValue(of(buildPathIndex([knowThyself, unrelated])));

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        expect(recs.length).toBeGreaterThanOrEqual(1);
        expect(recs[0].pathId).toBe('know-thyself-path');
        done();
      });
    }));

    it('should rank paths with more tag matches higher', () => new Promise<void>(done => {
      const manyMatches = buildPathEntry({
        id: 'many-matches',
        title: 'Many Matches',
        tags: ['values', 'personality', 'strengths'],
      });
      const fewMatches = buildPathEntry({
        id: 'few-matches',
        title: 'Few Matches',
        tags: ['values', 'governance', 'policy'],
      });

      mockDataLoader.getPathIndex.mockReturnValue(of(buildPathIndex([fewMatches, manyMatches])));

      const profile = buildProfile({
        totalAssessments: 2,
        byCategory: {
          values: [buildResult({ category: 'values' })],
          personality: [buildResult({ category: 'personality', framework: 'enneagram' })],
          strengths: [buildResult({ category: 'strengths', framework: 'clifton-strengths' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        expect(recs[0].pathId).toBe('many-matches');
        expect(recs[0].relevanceScore).toBeGreaterThan(recs[1].relevanceScore);
        done();
      });
    }));

    it('should include matched categories in recommendation', () => new Promise<void>(done => {
      const path = buildPathEntry({
        id: 'test-path',
        title: 'Test',
        tags: ['values', 'personality', 'other'],
      });

      mockDataLoader.getPathIndex.mockReturnValue(of(buildPathIndex([path])));

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        expect(recs[0].matchedCategories).toContain('values');
        expect(recs[0].matchedCategories).not.toContain('personality');
        done();
      });
    }));
  });

  // =========================================================================
  // Framework affinity
  // =========================================================================

  describe('framework affinity', () => {
    it('should boost paths with framework-affinity tags', () => new Promise<void>(done => {
      const selfKnowledge = buildPathEntry({
        id: 'self-knowledge-path',
        title: 'Self Knowledge',
        tags: ['self-knowledge', 'ethics'],
      });
      const noAffinity = buildPathEntry({
        id: 'no-affinity',
        title: 'No Affinity',
        tags: ['coding', 'tech'],
      });

      mockDataLoader.getPathIndex.mockReturnValue(
        of(buildPathIndex([noAffinity, selfKnowledge]))
      );

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values', framework: 'values-hierarchy' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        // self-knowledge-path should get affinity boost from values-hierarchy → self-knowledge, ethics
        const selfKnowledgeRec = recs.find(r => r.pathId === 'self-knowledge-path');
        expect(selfKnowledgeRec).toBeDefined();
        expect(selfKnowledgeRec!.relevanceScore).toBeGreaterThan(0);
        done();
      });
    }));
  });

  // =========================================================================
  // Intimate path filtering
  // =========================================================================

  describe('intimate path filtering', () => {
    it('should exclude paths with intimate visibility', () => new Promise<void>(done => {
      const publicPath = buildPathEntry({
        id: 'public-path',
        title: 'Public',
        tags: ['values'],
      });
      const intimatePath = Object.assign(
        buildPathEntry({
          id: 'love-map',
          title: 'Love Map',
          tags: ['values', 'intimate'],
        }),
        { visibility: 'intimate' }
      );

      mockDataLoader.getPathIndex.mockReturnValue(
        of(buildPathIndex([publicPath, intimatePath as PathIndexEntry]))
      );

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        const ids = recs.map(r => r.pathId);
        expect(ids).not.toContain('love-map');
        expect(ids).toContain('public-path');
        done();
      });
    }));

    it('should exclude paths with private visibility', () => new Promise<void>(done => {
      const privatePath = Object.assign(
        buildPathEntry({ id: 'private-path', title: 'Private', tags: ['values'] }),
        { visibility: 'private' }
      );

      mockDataLoader.getPathIndex.mockReturnValue(
        of(buildPathIndex([privatePath as PathIndexEntry]))
      );

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        expect(recs.length).toBe(0);
        done();
      });
    }));
  });

  // =========================================================================
  // Sorting
  // =========================================================================

  describe('sorting', () => {
    it('should return recommendations sorted by relevance descending', () => new Promise<void>(done => {
      const paths = [
        buildPathEntry({ id: 'low', title: 'Low', tags: ['other', 'unrelated', 'values'] }),
        buildPathEntry({ id: 'high', title: 'High', tags: ['values', 'personality'] }),
        buildPathEntry({ id: 'mid', title: 'Mid', tags: ['values', 'governance'] }),
      ];

      mockDataLoader.getPathIndex.mockReturnValue(of(buildPathIndex(paths)));

      const profile = buildProfile({
        totalAssessments: 2,
        byCategory: {
          values: [buildResult({ category: 'values' })],
          personality: [buildResult({ category: 'personality', framework: 'enneagram' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        for (let i = 1; i < recs.length; i++) {
          expect(recs[i - 1].relevanceScore).toBeGreaterThanOrEqual(recs[i].relevanceScore);
        }
        done();
      });
    }));
  });

  // =========================================================================
  // getTopRecommendation
  // =========================================================================

  describe('getTopRecommendation', () => {
    it('should return the highest-scored recommendation', () => new Promise<void>(done => {
      const paths = [
        buildPathEntry({ id: 'low', title: 'Low', tags: ['unrelated'] }),
        buildPathEntry({ id: 'best', title: 'Best', tags: ['values', 'personality'] }),
      ];

      mockDataLoader.getPathIndex.mockReturnValue(of(buildPathIndex(paths)));

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values' })],
          personality: [buildResult({ category: 'personality', framework: 'enneagram' })],
        },
      });

      service.getTopRecommendation(profile).subscribe(rec => {
        expect(rec).not.toBeNull();
        expect(rec!.pathId).toBe('best');
        done();
      });
    }));

    it('should return null when no recommendations match', () => new Promise<void>(done => {
      const paths = [
        buildPathEntry({ id: 'unrelated', title: 'Unrelated', tags: ['governance'] }),
      ];

      mockDataLoader.getPathIndex.mockReturnValue(of(buildPathIndex(paths)));

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values' })],
        },
      });

      service.getTopRecommendation(profile).subscribe(rec => {
        expect(rec).toBeNull();
        done();
      });
    }));

    it('should return null when profile has no assessments', () => new Promise<void>(done => {
      const profile = buildProfile({ totalAssessments: 0 });

      service.getTopRecommendation(profile).subscribe(rec => {
        expect(rec).toBeNull();
        done();
      });
    }));
  });

  // =========================================================================
  // Reason text
  // =========================================================================

  describe('reason text', () => {
    it('should include matched category names in reason', () => new Promise<void>(done => {
      const path = buildPathEntry({
        id: 'path',
        title: 'Path',
        tags: ['values', 'personality'],
      });

      mockDataLoader.getPathIndex.mockReturnValue(of(buildPathIndex([path])));

      const profile = buildProfile({
        totalAssessments: 1,
        byCategory: {
          values: [buildResult({ category: 'values' })],
        },
      });

      service.getRecommendations(profile).subscribe(recs => {
        expect(recs[0].reason).toContain('values');
        done();
      });
    }));
  });
});
