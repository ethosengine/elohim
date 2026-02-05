import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';

import { QuestionPoolService } from './question-pool.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { PathService } from '../../services/path.service';
import type { PerseusItem } from '../../content-io/plugins/sophia/sophia-moment.model';
import type { QuestionPool } from '../models/question-pool.model';
import type { LearningPath } from '../../models/learning-path.model';

describe('QuestionPoolService', () => {
  let service: QuestionPoolService;
  let mockDataLoader: jasmine.SpyObj<DataLoaderService>;
  let mockPathService: jasmine.SpyObj<PathService>;

  const createMockQuestion = (id: string, contentId: string, bloomsLevel = 'understand', difficulty = 'medium'): PerseusItem => ({
    id,
    purpose: 'mastery',
    content: {
      content: 'Test question',
      widgets: {}
    },
    metadata: {
      assessesContentId: contentId,
      bloomsLevel,
      difficulty,
      tags: ['test']
    }
  });

  const createMockPool = (contentId: string, questions: PerseusItem[]): QuestionPool => ({
    contentId,
    questions,
    metadata: {
      minPracticeQuestions: 3,
      minMasteryQuestions: 5,
      bloomsDistribution: {
        remember: 0,
        understand: 0,
        apply: 0,
        analyze: 0
      },
      difficultyDistribution: {
        easy: 0,
        medium: 0,
        hard: 0
      },
      isComplete: questions.length >= 5,
      tags: [],
      sourceDocs: []
    },
    updatedAt: new Date().toISOString(),
    createdAt: new Date().toISOString(),
    version: 1
  });

  const createMockPath = (): LearningPath => ({
    id: 'path-1',
    title: 'Test Path',
    description: 'Test path description',
    chapters: [
      {
        id: 'chapter-1',
        title: 'Chapter 1',
        modules: [
          {
            id: 'module-1',
            title: 'Module 1',
            sections: [
              {
                id: 'section-1',
                title: 'Section 1',
                conceptIds: ['content-1', 'content-2']
              },
              {
                id: 'section-2',
                title: 'Section 2',
                conceptIds: ['content-3']
              }
            ]
          }
        ]
      }
    ]
  } as LearningPath);

  beforeEach(() => {
    mockDataLoader = jasmine.createSpyObj('DataLoaderService', ['getContent']);
    mockDataLoader.getContent.and.returnValue(of(null as any));

    mockPathService = jasmine.createSpyObj('PathService', ['getPath']);
    mockPathService.getPath.and.returnValue(of(null as any));

    TestBed.configureTestingModule({
      providers: [
        QuestionPoolService,
        { provide: DataLoaderService, useValue: mockDataLoader },
        { provide: PathService, useValue: mockPathService }
      ],
    });
    service = TestBed.inject(QuestionPoolService);
  });

  afterEach(() => {
    service.clearCache();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getPoolForContent', () => {
    it('should load question pool for content', (done) => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const pool = createMockPool('content-1', questions);

      mockDataLoader.getContent.and.returnValue(of({
        content: JSON.stringify(pool),
        contentType: 'question-pool'
      } as any));

      service.getPoolForContent('content-1').subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          expect(result?.contentId).toBe('content-1');
          expect(result?.questions.length).toBe(1);
          done();
        },
        error: done.fail
      });
    });

    it('should return null if pool not found', (done) => {
      mockDataLoader.getContent.and.returnValue(of(null as any));

      service.getPoolForContent('content-1').subscribe({
        next: (result) => {
          expect(result).toBeNull();
          done();
        },
        error: done.fail
      });
    });

    it('should cache loaded pools', (done) => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const pool = createMockPool('content-1', questions);

      mockDataLoader.getContent.and.returnValue(of({
        content: JSON.stringify(pool),
        contentType: 'question-pool'
      } as any));

      // First call
      service.getPoolForContent('content-1').subscribe({
        next: () => {
          // Second call should use cache
          service.getPoolForContent('content-1').subscribe({
            next: () => {
              // Should only call dataLoader once due to caching
              expect(mockDataLoader.getContent).toHaveBeenCalledTimes(1);
              done();
            },
            error: done.fail
          });
        },
        error: done.fail
      });
    });

    it('should handle error gracefully', (done) => {
      mockDataLoader.getContent.and.returnValue(throwError(() => new Error('Load failed')));

      service.getPoolForContent('content-1').subscribe({
        next: (result) => {
          expect(result).toBeNull();
          done();
        },
        error: done.fail
      });
    });

    it('should wrap raw questions array in pool format', (done) => {
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-1')
      ];

      mockDataLoader.getContent.and.returnValue(of({
        content: JSON.stringify(questions),
        contentType: 'question-pool'
      } as any));

      service.getPoolForContent('content-1').subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          expect(result?.questions.length).toBe(2);
          expect(result?.metadata).toBeDefined();
          done();
        },
        error: done.fail
      });
    });
  });

  describe('getPoolsForContents', () => {
    it('should load multiple pools', (done) => {
      const pool1 = createMockPool('content-1', [createMockQuestion('q1', 'content-1')]);
      const pool2 = createMockPool('content-2', [createMockQuestion('q2', 'content-2')]);

      mockDataLoader.getContent.and.callFake((path: string) => {
        if (path.includes('content-1')) {
          return of({ content: JSON.stringify(pool1), contentType: 'question-pool' } as any);
        }
        if (path.includes('content-2')) {
          return of({ content: JSON.stringify(pool2), contentType: 'question-pool' } as any);
        }
        return of(null as any);
      });

      service.getPoolsForContents(['content-1', 'content-2']).subscribe({
        next: (result) => {
          expect(result.size).toBe(2);
          expect(result.get('content-1')).toBeDefined();
          expect(result.get('content-2')).toBeDefined();
          done();
        },
        error: done.fail
      });
    });

    it('should return empty map for empty input', (done) => {
      service.getPoolsForContents([]).subscribe({
        next: (result) => {
          expect(result.size).toBe(0);
          done();
        },
        error: done.fail
      });
    });

    it('should skip content with no pool', (done) => {
      const pool1 = createMockPool('content-1', [createMockQuestion('q1', 'content-1')]);

      mockDataLoader.getContent.and.callFake((path: string) => {
        if (path.includes('content-1')) {
          return of({ content: JSON.stringify(pool1), contentType: 'question-pool' } as any);
        }
        return of(null as any);
      });

      service.getPoolsForContents(['content-1', 'content-2']).subscribe({
        next: (result) => {
          expect(result.size).toBe(1);
          expect(result.has('content-1')).toBeTrue();
          expect(result.has('content-2')).toBeFalse();
          done();
        },
        error: done.fail
      });
    });
  });

  describe('getHierarchicalPool', () => {
    it('should get hierarchical pool from path', (done) => {
      const path = createMockPath();
      mockPathService.getPath.and.returnValue(of(path));

      service.getHierarchicalPool('path-1', 'section-1').subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          expect(result.pathId).toBe('path-1');
          expect(result.sectionId).toBe('section-1');
          expect(result.eligibleContentIds).toContain('content-1');
          expect(result.eligibleContentIds).toContain('content-2');
          done();
        },
        error: done.fail
      });
    });

    it('should return empty source if path not found', (done) => {
      mockPathService.getPath.and.returnValue(of(null as any));

      service.getHierarchicalPool('path-1', 'section-1').subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          expect(result.eligibleContentIds.length).toBe(0);
          done();
        },
        error: done.fail
      });
    });

    it('should include content up to current section', (done) => {
      const path = createMockPath();
      mockPathService.getPath.and.returnValue(of(path));

      service.getHierarchicalPool('path-1', 'section-2').subscribe({
        next: (result) => {
          // Should include content from section-1 and section-2
          expect(result.eligibleContentIds).toContain('content-1');
          expect(result.eligibleContentIds).toContain('content-2');
          expect(result.eligibleContentIds).toContain('content-3');
          done();
        },
        error: done.fail
      });
    });
  });

  describe('loadHierarchicalPools', () => {
    it('should load and combine pools', (done) => {
      const pool1 = createMockPool('content-1', [
        createMockQuestion('q1', 'content-1', 'remember', 'easy')
      ]);
      const pool2 = createMockPool('content-2', [
        createMockQuestion('q2', 'content-2', 'understand', 'medium')
      ]);

      mockDataLoader.getContent.and.callFake((path: string) => {
        if (path.includes('content-1')) {
          return of({ content: JSON.stringify(pool1), contentType: 'question-pool' } as any);
        }
        if (path.includes('content-2')) {
          return of({ content: JSON.stringify(pool2), contentType: 'question-pool' } as any);
        }
        return of(null as any);
      });

      const source = {
        currentContentId: 'section-1',
        pathId: 'path-1',
        sectionId: 'section-1',
        eligibleContentIds: ['content-1', 'content-2'],
        combinedPool: [],
        stats: {
          totalQuestions: 0,
          questionsByContent: new Map(),
          questionsByBlooms: {
            remember: 0,
            understand: 0,
            apply: 0,
            analyze: 0,
            evaluate: 0,
            create: 0
          },
          questionsByDifficulty: {
            easy: 0,
            medium: 0,
            hard: 0
          }
        }
      };

      service.loadHierarchicalPools(source).subscribe({
        next: (result) => {
          expect(result.combinedPool.length).toBe(2);
          expect(result.stats.totalQuestions).toBe(2);
          expect(result.stats.questionsByBlooms.remember).toBe(1);
          expect(result.stats.questionsByBlooms.understand).toBe(1);
          expect(result.stats.questionsByDifficulty.easy).toBe(1);
          expect(result.stats.questionsByDifficulty.medium).toBe(1);
          done();
        },
        error: done.fail
      });
    });

    it('should handle empty eligibleContentIds', (done) => {
      const source = {
        currentContentId: 'section-1',
        pathId: 'path-1',
        sectionId: 'section-1',
        eligibleContentIds: [],
        combinedPool: [],
        stats: {
          totalQuestions: 0,
          questionsByContent: new Map(),
          questionsByBlooms: {
            remember: 0,
            understand: 0,
            apply: 0,
            analyze: 0,
            evaluate: 0,
            create: 0
          },
          questionsByDifficulty: {
            easy: 0,
            medium: 0,
            hard: 0
          }
        }
      };

      service.loadHierarchicalPools(source).subscribe({
        next: (result) => {
          expect(result.combinedPool.length).toBe(0);
          done();
        },
        error: done.fail
      });
    });
  });

  describe('selectQuestions', () => {
    const questions = [
      createMockQuestion('q1', 'content-1', 'remember', 'easy'),
      createMockQuestion('q2', 'content-1', 'understand', 'medium'),
      createMockQuestion('q3', 'content-2', 'apply', 'hard'),
      createMockQuestion('q4', 'content-2', 'understand', 'medium'),
      createMockQuestion('q5', 'content-3', 'remember', 'easy')
    ];

    it('should select requested number of questions', () => {
      const result = service.selectQuestions(questions, { count: 3, randomize: false });

      expect(result.questions.length).toBe(3);
      expect(result.selectionComplete).toBeTrue();
    });

    it('should filter by Blooms level', () => {
      const result = service.selectQuestions(questions, {
        count: 10,
        bloomsLevels: ['remember'],
        randomize: false
      });

      expect(result.questions.length).toBe(2);
      expect(result.questions.every(q => q.metadata?.['bloomsLevel'] === 'remember')).toBeTrue();
    });

    it('should filter by difficulty', () => {
      const result = service.selectQuestions(questions, {
        count: 10,
        difficulty: ['easy'],
        randomize: false
      });

      expect(result.questions.length).toBe(2);
      expect(result.questions.every(q => q.metadata?.['difficulty'] === 'easy')).toBeTrue();
    });

    it('should filter by tags', () => {
      const result = service.selectQuestions(questions, {
        count: 10,
        tags: ['test'],
        randomize: false
      });

      expect(result.questions.length).toBe(5);
    });

    it('should exclude specified IDs', () => {
      const result = service.selectQuestions(questions, {
        count: 10,
        excludeIds: ['q1', 'q2'],
        randomize: false
      });

      expect(result.questions.length).toBe(3);
      expect(result.questions.find(q => q.id === 'q1')).toBeUndefined();
      expect(result.questions.find(q => q.id === 'q2')).toBeUndefined();
    });

    it('should return incomplete selection when not enough questions', () => {
      const result = service.selectQuestions(questions, {
        count: 10,
        bloomsLevels: ['remember'],
        randomize: false
      });

      expect(result.selectionComplete).toBeFalse();
      expect(result.selectionNotes).toBeDefined();
      expect(result.selectionNotes![0]).toContain('Only 2 questions available');
    });

    it('should randomize when requested', () => {
      const result1 = service.selectQuestions(questions, { count: 3, randomize: true });
      const result2 = service.selectQuestions(questions, { count: 3, randomize: true });

      // Results might be different due to randomization (not guaranteed but likely)
      expect(result1.questions.length).toBe(3);
      expect(result2.questions.length).toBe(3);
    });

    it('should ensure variety across content', () => {
      const result = service.selectQuestions(questions, {
        count: 5,
        ensureVariety: true,
        randomize: false
      });

      // Should interleave questions from different content
      expect(result.questions.length).toBe(5);
      expect(result.contentIds.length).toBeGreaterThan(1);
    });

    it('should apply weighted selection', () => {
      const weights = new Map([['content-1', 2]]);

      const result = service.selectQuestions(questions, {
        count: 3,
        weightedContentIds: weights,
        randomize: false
      });

      // Questions from content-1 should appear more frequently
      expect(result.questions.length).toBe(3);
    });
  });

  describe('selectPracticeQuestions', () => {
    it('should select practice questions with appropriate filters', () => {
      const source = {
        currentContentId: 'section-1',
        pathId: 'path-1',
        sectionId: 'section-1',
        eligibleContentIds: ['content-1'],
        combinedPool: [
          createMockQuestion('q1', 'content-1', 'remember'),
          createMockQuestion('q2', 'content-1', 'understand'),
          createMockQuestion('q3', 'content-1', 'apply')
        ],
        stats: {
          totalQuestions: 3,
          questionsByContent: new Map(),
          questionsByBlooms: {
            remember: 1,
            understand: 1,
            apply: 1,
            analyze: 0,
            evaluate: 0,
            create: 0
          },
          questionsByDifficulty: {
            easy: 0,
            medium: 3,
            hard: 0
          }
        }
      };

      const result = service.selectPracticeQuestions(source, 5);

      expect(result.questions.length).toBeLessThanOrEqual(3);
      expect(result.questions.every(q =>
        ['remember', 'understand', 'apply'].includes(q.metadata?.['bloomsLevel'] as string)
      )).toBeTrue();
    });
  });

  describe('selectMasteryQuestions', () => {
    it('should weight toward practiced content', () => {
      const pool = [
        createMockQuestion('q1', 'content-1', 'understand'),
        createMockQuestion('q2', 'content-2', 'apply')
      ];

      const result = service.selectMasteryQuestions(pool, 5, ['content-1']);

      expect(result.questions.length).toBeGreaterThan(0);
    });

    it('should use higher-level Blooms questions', () => {
      const pool = [
        createMockQuestion('q1', 'content-1', 'understand'),
        createMockQuestion('q2', 'content-1', 'apply'),
        createMockQuestion('q3', 'content-1', 'analyze')
      ];

      const result = service.selectMasteryQuestions(pool, 5);

      expect(result.questions.every(q =>
        ['understand', 'apply', 'analyze'].includes(q.metadata?.['bloomsLevel'] as string)
      )).toBeTrue();
    });
  });

  describe('selectInlineQuestions', () => {
    it('should select questions for content', (done) => {
      const pool = createMockPool('content-1', [
        createMockQuestion('q1', 'content-1', 'remember'),
        createMockQuestion('q2', 'content-1', 'understand')
      ]);

      mockDataLoader.getContent.and.returnValue(of({
        content: JSON.stringify(pool),
        contentType: 'question-pool'
      } as any));

      service.selectInlineQuestions('content-1', 10).subscribe({
        next: (result) => {
          expect(result.questions.length).toBe(2);
          expect(result.questions.every(q =>
            ['remember', 'understand'].includes(q.metadata?.['bloomsLevel'] as string)
          )).toBeTrue();
          done();
        },
        error: done.fail
      });
    });

    it('should return empty result if no pool available', (done) => {
      mockDataLoader.getContent.and.returnValue(of(null as any));

      service.selectInlineQuestions('content-1', 10).subscribe({
        next: (result) => {
          expect(result.questions.length).toBe(0);
          expect(result.selectionComplete).toBeFalse();
          expect(result.selectionNotes).toContain('No questions available for this content');
          done();
        },
        error: done.fail
      });
    });
  });

  describe('Pool Management', () => {
    describe('canPracticeContent', () => {
      it('should return true if pool has enough questions', (done) => {
        const pool = createMockPool('content-1', [
          createMockQuestion('q1', 'content-1'),
          createMockQuestion('q2', 'content-1'),
          createMockQuestion('q3', 'content-1')
        ]);

        mockDataLoader.getContent.and.returnValue(of({
          content: JSON.stringify(pool),
          contentType: 'question-pool'
        } as any));

        service.canPracticeContent('content-1').subscribe({
          next: (result) => {
            expect(result).toBeTrue();
            done();
          },
          error: done.fail
        });
      });

      it('should return false if no pool', (done) => {
        mockDataLoader.getContent.and.returnValue(of(null as any));

        service.canPracticeContent('content-1').subscribe({
          next: (result) => {
            expect(result).toBeFalse();
            done();
          },
          error: done.fail
        });
      });
    });

    describe('canMasteryContent', () => {
      it('should return true if pool has enough questions', (done) => {
        const pool = createMockPool('content-1', [
          createMockQuestion('q1', 'content-1'),
          createMockQuestion('q2', 'content-1'),
          createMockQuestion('q3', 'content-1'),
          createMockQuestion('q4', 'content-1'),
          createMockQuestion('q5', 'content-1')
        ]);

        mockDataLoader.getContent.and.returnValue(of({
          content: JSON.stringify(pool),
          contentType: 'question-pool'
        } as any));

        service.canMasteryContent('content-1').subscribe({
          next: (result) => {
            expect(result).toBeTrue();
            done();
          },
          error: done.fail
        });
      });
    });

    describe('getPoolCompleteness', () => {
      it('should calculate completeness percentage', (done) => {
        const pool = createMockPool('content-1', [
          createMockQuestion('q1', 'content-1'),
          createMockQuestion('q2', 'content-1'),
          createMockQuestion('q3', 'content-1')
        ]);

        mockDataLoader.getContent.and.returnValue(of({
          content: JSON.stringify(pool),
          contentType: 'question-pool'
        } as any));

        service.getPoolCompleteness('content-1').subscribe({
          next: (result) => {
            expect(result).toBeGreaterThanOrEqual(0);
            expect(result).toBeLessThanOrEqual(100);
            done();
          },
          error: done.fail
        });
      });

      it('should return 0 if no pool', (done) => {
        mockDataLoader.getContent.and.returnValue(of(null as any));

        service.getPoolCompleteness('content-1').subscribe({
          next: (result) => {
            expect(result).toBe(0);
            done();
          },
          error: done.fail
        });
      });
    });
  });

  describe('searchPools', () => {
    it('should return empty array for empty contentIds', (done) => {
      service.searchPools({ contentIds: [] }).subscribe({
        next: (result) => {
          expect(result.length).toBe(0);
          done();
        },
        error: done.fail
      });
    });

    it('should filter by minimum questions', (done) => {
      const pool1 = createMockPool('content-1', [createMockQuestion('q1', 'content-1')]);
      const pool2 = createMockPool('content-2', [
        createMockQuestion('q2', 'content-2'),
        createMockQuestion('q3', 'content-2'),
        createMockQuestion('q4', 'content-2')
      ]);

      mockDataLoader.getContent.and.callFake((path: string) => {
        if (path.includes('content-1')) {
          return of({ content: JSON.stringify(pool1), contentType: 'question-pool' } as any);
        }
        if (path.includes('content-2')) {
          return of({ content: JSON.stringify(pool2), contentType: 'question-pool' } as any);
        }
        return of(null as any);
      });

      service.searchPools({ contentIds: ['content-1', 'content-2'], minQuestions: 3 }).subscribe({
        next: (result) => {
          expect(result.length).toBe(1);
          expect(result[0].contentId).toBe('content-2');
          done();
        },
        error: done.fail
      });
    });

    it('should filter by isComplete flag', (done) => {
      const completePool = createMockPool('content-1', [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-1'),
        createMockQuestion('q3', 'content-1'),
        createMockQuestion('q4', 'content-1'),
        createMockQuestion('q5', 'content-1')
      ]);
      const incompletePool = createMockPool('content-2', [createMockQuestion('q2', 'content-2')]);

      mockDataLoader.getContent.and.callFake((path: string) => {
        if (path.includes('content-1')) {
          return of({ content: JSON.stringify(completePool), contentType: 'question-pool' } as any);
        }
        if (path.includes('content-2')) {
          return of({ content: JSON.stringify(incompletePool), contentType: 'question-pool' } as any);
        }
        return of(null as any);
      });

      service.searchPools({ contentIds: ['content-1', 'content-2'], isComplete: true }).subscribe({
        next: (result) => {
          expect(result.length).toBe(1);
          expect(result[0].metadata.isComplete).toBeTrue();
          done();
        },
        error: done.fail
      });
    });
  });

  describe('clearCache', () => {
    it('should clear the pool cache', (done) => {
      const pool = createMockPool('content-1', [createMockQuestion('q1', 'content-1')]);

      mockDataLoader.getContent.and.returnValue(of({
        content: JSON.stringify(pool),
        contentType: 'question-pool'
      } as any));

      // Load pool to cache it
      service.getPoolForContent('content-1').subscribe({
        next: () => {
          // Clear cache
          service.clearCache();

          // Load again - should call dataLoader again
          service.getPoolForContent('content-1').subscribe({
            next: () => {
              expect(mockDataLoader.getContent).toHaveBeenCalledTimes(2);
              done();
            },
            error: done.fail
          });
        },
        error: done.fail
      });
    });
  });
});
