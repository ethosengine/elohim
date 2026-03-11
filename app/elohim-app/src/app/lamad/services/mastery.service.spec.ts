import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ContentMasteryView } from '@app/elohim/adapters/storage-types.adapter';
import { StorageApiService } from '@app/elohim/services/storage-api.service';

import {
  MasteryService,
  MasteryLevels,
  MasteryLevelType,
  MASTERY_LEVEL_ORDER,
} from './mastery.service';
import { vi } from 'vitest';

describe('MasteryService', () => {
  let service: MasteryService;
  let storageApiSpy: any;

  const createMockMastery = (overrides: Partial<ContentMasteryView> = {}): ContentMasteryView => ({
    id: 'mastery-1',
    appId: 'elohim',
    humanId: 'human-1',
    contentId: 'content-1',
    masteryLevel: 'understanding',
    masteryLevelIndex: 2,
    freshnessScore: 0.8,
    needsRefresh: false,
    engagementCount: 5,
    lastEngagementType: 'view',
    lastEngagementAt: '2024-01-01T00:00:00Z',
    levelAchievedAt: '2024-01-01T00:00:00Z',
    contentVersionAtMastery: null,
    assessmentEvidence: null,
    privileges: null,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    ...overrides,
  });

  beforeEach(() => {
    storageApiSpy = {
      getMasteryRecords: vi.fn(),
      upsertMastery: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [MasteryService, { provide: StorageApiService, useValue: storageApiSpy }],
    });
    service = TestBed.inject(MasteryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('MasteryLevels constants', () => {
    it('should have all Bloom taxonomy levels', () => {
      expect(MasteryLevels.NOT_STARTED).toBe('not_started');
      expect(MasteryLevels.AWARE).toBe('aware');
      expect(MasteryLevels.UNDERSTANDING).toBe('understanding');
      expect(MasteryLevels.APPLYING).toBe('applying');
      expect(MasteryLevels.ANALYZING).toBe('analyzing');
      expect(MasteryLevels.EVALUATING).toBe('evaluating');
      expect(MasteryLevels.MASTERED).toBe('mastered');
    });
  });

  describe('MASTERY_LEVEL_ORDER', () => {
    it('should order levels correctly', () => {
      expect(MASTERY_LEVEL_ORDER.not_started).toBe(0);
      expect(MASTERY_LEVEL_ORDER.aware).toBe(1);
      expect(MASTERY_LEVEL_ORDER.understanding).toBe(2);
      expect(MASTERY_LEVEL_ORDER.applying).toBe(3);
      expect(MASTERY_LEVEL_ORDER.analyzing).toBe(4);
      expect(MASTERY_LEVEL_ORDER.evaluating).toBe(5);
      expect(MASTERY_LEVEL_ORDER.mastered).toBe(6);
    });
  });

  describe('getMasteryForHuman', () => {
    it('should fetch all mastery records for a human', () =>
      new Promise<void>(done => {
        const mockRecords = [
          createMockMastery({ contentId: 'content-1' }),
          createMockMastery({ contentId: 'content-2' }),
        ];
        storageApiSpy.getMasteryRecords.mockReturnValue(of(mockRecords));

        service.getMasteryForHuman('human-1').subscribe(records => {
          expect(records.length).toBe(2);
          expect(storageApiSpy.getMasteryRecords).toHaveBeenCalledWith({ humanId: 'human-1' });
          done();
        });
      }));
  });

  describe('getMasteryForContent', () => {
    it('should return mastery record when found', () =>
      new Promise<void>(done => {
        const mockRecord = createMockMastery();
        storageApiSpy.getMasteryRecords.mockReturnValue(of([mockRecord]));

        service.getMasteryForContent('human-1', 'content-1').subscribe(record => {
          expect(record).toEqual(mockRecord);
          expect(storageApiSpy.getMasteryRecords).toHaveBeenCalledWith({
            humanId: 'human-1',
            contentId: 'content-1',
          });
          done();
        });
      }));

    it('should return null when no record found', () =>
      new Promise<void>(done => {
        storageApiSpy.getMasteryRecords.mockReturnValue(of([]));

        service.getMasteryForContent('human-1', 'nonexistent').subscribe(record => {
          expect(record).toBeNull();
          done();
        });
      }));
  });

  describe('getMasteryState', () => {
    it('should return map of contentId to mastery records', () =>
      new Promise<void>(done => {
        const mockRecords = [
          createMockMastery({ contentId: 'content-1', masteryLevel: 'aware' }),
          createMockMastery({ contentId: 'content-2', masteryLevel: 'mastered' }),
          createMockMastery({ contentId: 'content-3', masteryLevel: 'understanding' }),
        ];
        storageApiSpy.getMasteryRecords.mockReturnValue(of(mockRecords));

        service.getMasteryState('human-1', ['content-1', 'content-2']).subscribe(result => {
          expect(result.size).toBe(2);
          expect(result.get('content-1')?.masteryLevel).toBe('aware');
          expect(result.get('content-2')?.masteryLevel).toBe('mastered');
          expect(result.has('content-3')).toBe(false);
          done();
        });
      }));

    it('should return empty map for empty contentIds', () =>
      new Promise<void>(done => {
        service.getMasteryState('human-1', []).subscribe(result => {
          expect(result.size).toBe(0);
          expect(storageApiSpy.getMasteryRecords).not.toHaveBeenCalled();
          done();
        });
      }));
  });

  describe('getMasteryAtLevel', () => {
    it('should fetch mastery records at or above minimum level', () =>
      new Promise<void>(done => {
        const mockRecords = [createMockMastery({ masteryLevel: 'applying' })];
        storageApiSpy.getMasteryRecords.mockReturnValue(of(mockRecords));

        service.getMasteryAtLevel('human-1', 'understanding').subscribe(records => {
          expect(records).toEqual(mockRecords);
          expect(storageApiSpy.getMasteryRecords).toHaveBeenCalledWith({
            humanId: 'human-1',
            minLevel: 'understanding',
          });
          done();
        });
      }));
  });

  describe('getRefreshNeeded', () => {
    it('should fetch content needing refresh', () =>
      new Promise<void>(done => {
        const mockRecords = [createMockMastery({ freshnessScore: 0.3 })];
        storageApiSpy.getMasteryRecords.mockReturnValue(of(mockRecords));

        service.getRefreshNeeded('human-1').subscribe(records => {
          expect(records).toEqual(mockRecords);
          expect(storageApiSpy.getMasteryRecords).toHaveBeenCalledWith({
            humanId: 'human-1',
            needsRefresh: true,
          });
          done();
        });
      }));
  });

  describe('recordEngagement', () => {
    it('should record engagement and return updated mastery', () =>
      new Promise<void>(done => {
        const updatedMastery = createMockMastery({ engagementCount: 6 });
        storageApiSpy.upsertMastery.mockReturnValue(of(updatedMastery));

        service.recordEngagement('human-1', 'content-1', 'view').subscribe(result => {
          expect(result).toEqual(updatedMastery);
          expect(storageApiSpy.upsertMastery).toHaveBeenCalledWith({
            humanId: 'human-1',
            contentId: 'content-1',
            engagementType: 'view',
          });
          done();
        });
      }));
  });

  describe('updateMasteryLevel', () => {
    it('should update mastery level directly', () =>
      new Promise<void>(done => {
        const updatedMastery = createMockMastery({ masteryLevel: 'mastered' });
        storageApiSpy.upsertMastery.mockReturnValue(of(updatedMastery));

        service.updateMasteryLevel('human-1', 'content-1', 'mastered').subscribe(result => {
          expect(result.masteryLevel).toBe('mastered');
          expect(storageApiSpy.upsertMastery).toHaveBeenCalledWith({
            humanId: 'human-1',
            contentId: 'content-1',
            masteryLevel: 'mastered',
          });
          done();
        });
      }));
  });

  describe('recordBulkEngagement', () => {
    it('should record engagement for multiple content items', () =>
      new Promise<void>(done => {
        const mockResults = [
          createMockMastery({ contentId: 'content-1' }),
          createMockMastery({ contentId: 'content-2' }),
        ];
        storageApiSpy.upsertMastery
          .mockReturnValueOnce(of(mockResults[0]))
          .mockReturnValueOnce(of(mockResults[1]));

        service
          .recordBulkEngagement('human-1', ['content-1', 'content-2'], 'view')
          .subscribe(results => {
            expect(results.length).toBe(2);
            expect(storageApiSpy.upsertMastery).toHaveBeenCalledTimes(2);
            done();
          });
      }));

    it('should return empty array for empty contentIds', () =>
      new Promise<void>(done => {
        service.recordBulkEngagement('human-1', [], 'view').subscribe(results => {
          expect(results).toEqual([]);
          expect(storageApiSpy.upsertMastery).not.toHaveBeenCalled();
          done();
        });
      }));
  });

  describe('isMasteryAtLeast', () => {
    it('should return true when level1 equals level2', () => {
      expect(service.isMasteryAtLeast('understanding', 'understanding')).toBe(true);
    });

    it('should return true when level1 is higher than level2', () => {
      expect(service.isMasteryAtLeast('mastered', 'aware')).toBe(true);
      expect(service.isMasteryAtLeast('applying', 'understanding')).toBe(true);
    });

    it('should return false when level1 is lower than level2', () => {
      expect(service.isMasteryAtLeast('aware', 'mastered')).toBe(false);
      expect(service.isMasteryAtLeast('understanding', 'applying')).toBe(false);
    });

    it('should return true for mastered vs any level', () => {
      expect(service.isMasteryAtLeast('mastered', 'not_started')).toBe(true);
      expect(service.isMasteryAtLeast('mastered', 'mastered')).toBe(true);
    });

    it('should return true for not_started vs not_started', () => {
      expect(service.isMasteryAtLeast('not_started', 'not_started')).toBe(true);
    });
  });

  describe('getNextLevel', () => {
    it('should return next level in progression', () => {
      expect(service.getNextLevel('not_started')).toBe('aware');
      expect(service.getNextLevel('aware')).toBe('understanding');
      expect(service.getNextLevel('understanding')).toBe('applying');
      expect(service.getNextLevel('applying')).toBe('analyzing');
      expect(service.getNextLevel('analyzing')).toBe('evaluating');
      expect(service.getNextLevel('evaluating')).toBe('mastered');
    });

    it('should return null for mastered level', () => {
      expect(service.getNextLevel('mastered')).toBeNull();
    });
  });

  describe('getMasteryProgress', () => {
    it('should return 0% for not_started', () => {
      expect(service.getMasteryProgress('not_started')).toBe(0);
    });

    it('should return 100% for mastered', () => {
      expect(service.getMasteryProgress('mastered')).toBe(100);
    });

    it('should return intermediate percentages for other levels', () => {
      expect(service.getMasteryProgress('aware')).toBe(17); // 1/6 * 100 rounded
      expect(service.getMasteryProgress('understanding')).toBe(33); // 2/6 * 100 rounded
      expect(service.getMasteryProgress('applying')).toBe(50); // 3/6 * 100 rounded
      expect(service.getMasteryProgress('analyzing')).toBe(67); // 4/6 * 100 rounded
      expect(service.getMasteryProgress('evaluating')).toBe(83); // 5/6 * 100 rounded
    });
  });

  describe('hasStarted', () => {
    it('should return false for null mastery', () => {
      expect(service.hasStarted(null)).toBe(false);
    });

    it('should return false for not_started level', () => {
      const mastery = createMockMastery({ masteryLevel: 'not_started' });
      expect(service.hasStarted(mastery)).toBe(false);
    });

    it('should return true for any level above not_started', () => {
      expect(service.hasStarted(createMockMastery({ masteryLevel: 'aware' }))).toBe(true);
      expect(service.hasStarted(createMockMastery({ masteryLevel: 'mastered' }))).toBe(true);
    });
  });

  describe('isMastered', () => {
    it('should return false for null mastery', () => {
      expect(service.isMastered(null)).toBe(false);
    });

    it('should return false for non-mastered levels', () => {
      expect(service.isMastered(createMockMastery({ masteryLevel: 'not_started' }))).toBe(false);
      expect(service.isMastered(createMockMastery({ masteryLevel: 'evaluating' }))).toBe(false);
    });

    it('should return true for mastered level', () => {
      expect(service.isMastered(createMockMastery({ masteryLevel: 'mastered' }))).toBe(true);
    });
  });

  describe('sortByLevel', () => {
    it('should sort records by level descending', () => {
      const records = [
        createMockMastery({ contentId: 'a', masteryLevel: 'aware' }),
        createMockMastery({ contentId: 'b', masteryLevel: 'mastered' }),
        createMockMastery({ contentId: 'c', masteryLevel: 'understanding' }),
      ];

      const sorted = service.sortByLevel(records);

      expect(sorted[0].contentId).toBe('b'); // mastered
      expect(sorted[1].contentId).toBe('c'); // understanding
      expect(sorted[2].contentId).toBe('a'); // aware
    });

    it('should not mutate original array', () => {
      const records = [
        createMockMastery({ contentId: 'a', masteryLevel: 'aware' }),
        createMockMastery({ contentId: 'b', masteryLevel: 'mastered' }),
      ];

      service.sortByLevel(records);

      expect(records[0].contentId).toBe('a');
    });
  });

  describe('sortByFreshness', () => {
    it('should sort records by freshness ascending (needs refresh first)', () => {
      const records = [
        createMockMastery({ contentId: 'a', freshnessScore: 0.9 }),
        createMockMastery({ contentId: 'b', freshnessScore: 0.2 }),
        createMockMastery({ contentId: 'c', freshnessScore: 0.5 }),
      ];

      const sorted = service.sortByFreshness(records);

      expect(sorted[0].contentId).toBe('b'); // 0.2 freshness
      expect(sorted[1].contentId).toBe('c'); // 0.5 freshness
      expect(sorted[2].contentId).toBe('a'); // 0.9 freshness
    });

    it('should not mutate original array', () => {
      const records = [
        createMockMastery({ contentId: 'a', freshnessScore: 0.9 }),
        createMockMastery({ contentId: 'b', freshnessScore: 0.2 }),
      ];

      service.sortByFreshness(records);

      expect(records[0].contentId).toBe('a');
    });
  });
});
