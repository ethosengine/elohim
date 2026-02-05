import { Injectable, inject } from '@angular/core';

// @coverage: 96.5% (2026-02-05)

import { Observable, of, forkJoin, map, catchError, shareReplay } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { LearningPath } from '../../models/learning-path.model';
import { PathService } from '../../services/path.service';
import {
  createEmptyPool,
  canPractice,
  canMastery,
  calculateCompleteness,
} from '../models/question-pool.model';

import type {
  PerseusItem,
  BloomsLevel,
  QuestionDifficulty,
} from '../../content-io/plugins/sophia/sophia-moment.model';
import type {
  QuestionPool,
  HierarchicalQuestionSource,
  HierarchicalPoolStats,
  QuestionSelectionOptions,
  QuestionSelectionResult,
  QuestionPoolQuery,
} from '../models/question-pool.model';

/**
 * QuestionPoolService - Manages question pools for content nodes.
 *
 * Responsibilities:
 * - Load question pools for individual content nodes
 * - Aggregate pools from content hierarchy (for practice quizzes)
 * - Select questions based on criteria (Bloom's level, difficulty, etc.)
 * - Support weighted selection for mastery quizzes
 *
 * @example
 * ```typescript
 * // Get pool for a single content node
 * const pool = await poolService.getPoolForContent('governance-basics').toPromise();
 *
 * // Get hierarchical pool for practice quiz
 * const source = await poolService.getHierarchicalPool('path-123', 'section-456').toPromise();
 *
 * // Select questions for a quiz
 * const questions = poolService.selectQuestions(source.combinedPool, {
 *   count: 5,
 *   bloomsLevels: ['understand', 'apply'],
 *   randomize: true
 * });
 * ```
 */
@Injectable({
  providedIn: 'root',
})
export class QuestionPoolService {
  private readonly dataLoader = inject(DataLoaderService);
  private readonly pathService = inject(PathService);

  /** Cache for loaded pools */
  private readonly poolCache = new Map<string, Observable<QuestionPool | null>>();

  /** Cache duration (5 minutes) */
  private readonly CACHE_DURATION_MS = 5 * 60 * 1000;

  // ═══════════════════════════════════════════════════════════════════════════
  // Pool Loading
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get question pool for a single content node.
   *
   * @param contentId - Content node ID
   * @returns Observable of pool or null if none exists
   */
  getPoolForContent(contentId: string): Observable<QuestionPool | null> {
    // Check cache first
    const cached = this.poolCache.get(contentId);
    if (cached) {
      return cached;
    }

    // Load from data loader
    const pool$ = this.loadPool(contentId).pipe(
      shareReplay(1),
      catchError(_err => {
        return of(null);
      })
    );

    // Cache with expiration
    this.poolCache.set(contentId, pool$);
    setTimeout(() => this.poolCache.delete(contentId), this.CACHE_DURATION_MS);

    return pool$;
  }

  /**
   * Get pools for multiple content nodes.
   *
   * @param contentIds - Array of content node IDs
   * @returns Observable of map from ID to pool
   */
  getPoolsForContents(contentIds: string[]): Observable<Map<string, QuestionPool>> {
    if (contentIds.length === 0) {
      return of(new Map());
    }

    const poolObservables = contentIds.map(id =>
      this.getPoolForContent(id).pipe(map(pool => ({ id, pool })))
    );

    return forkJoin(poolObservables).pipe(
      map(results => {
        const poolMap = new Map<string, QuestionPool>();
        for (const { id, pool } of results) {
          if (pool) {
            poolMap.set(id, pool);
          }
        }
        return poolMap;
      })
    );
  }

  /**
   * Get hierarchical question source for practice quizzes.
   *
   * Aggregates pools from all content below the current position in the path,
   * allowing learners to be tested on prerequisite material.
   *
   * @param pathId - Learning path ID
   * @param currentSectionId - Current section ID
   * @returns Observable of hierarchical source
   */
  getHierarchicalPool(
    pathId: string,
    currentSectionId: string
  ): Observable<HierarchicalQuestionSource> {
    return this.pathService.getPath(pathId).pipe(
      map(path => {
        if (!path) {
          return this.createEmptyHierarchicalSource(pathId, currentSectionId);
        }

        const eligibleContentIds = this.collectEligibleContentIds(path, currentSectionId);

        return {
          currentContentId: currentSectionId,
          pathId,
          sectionId: currentSectionId,
          eligibleContentIds,
          combinedPool: [], // Will be populated after loading pools
          stats: this.createEmptyStats(),
        };
      }),
      // Load pools for all eligible content
      map(source => {
        // This is synchronous placeholder - actual loading happens in next step
        return source;
      })
    );
  }

  /**
   * Collect all content IDs from path sections up to and including the current section.
   *
   * Traverses the path hierarchy (chapters → modules → sections) and gathers
   * concept IDs until the target section is found.
   */
  private collectEligibleContentIds(path: LearningPath, currentSectionId: string): string[] {
    const eligibleContentIds: string[] = [];

    for (const chapter of path.chapters ?? []) {
      for (const module of chapter.modules ?? []) {
        for (const section of module.sections ?? []) {
          if (section.conceptIds) {
            eligibleContentIds.push(...section.conceptIds);
          }

          if (section.id === currentSectionId) {
            return eligibleContentIds;
          }
        }
      }
    }

    return eligibleContentIds;
  }

  /**
   * Load and aggregate pools for hierarchical source.
   *
   * @param source - Hierarchical source with eligible content IDs
   * @returns Observable of source with populated pool
   */
  loadHierarchicalPools(
    source: HierarchicalQuestionSource
  ): Observable<HierarchicalQuestionSource> {
    if (source.eligibleContentIds.length === 0) {
      return of(source);
    }

    return this.getPoolsForContents(source.eligibleContentIds).pipe(
      map(poolMap => {
        const combinedPool: PerseusItem[] = [];
        const questionsByContent = new Map<string, number>();
        const questionsByBlooms: Record<BloomsLevel, number> = {
          remember: 0,
          understand: 0,
          apply: 0,
          analyze: 0,
          evaluate: 0,
          create: 0,
        };
        const questionsByDifficulty: Record<QuestionDifficulty, number> = {
          easy: 0,
          medium: 0,
          hard: 0,
        };

        // Aggregate all questions
        for (const [contentId, pool] of poolMap) {
          for (const question of pool.questions) {
            combinedPool.push(question);

            // Track stats
            questionsByContent.set(contentId, (questionsByContent.get(contentId) ?? 0) + 1);

            const bloomsLevel = (question.metadata?.['bloomsLevel'] as BloomsLevel) ?? 'understand';
            questionsByBlooms[bloomsLevel]++;

            const difficulty =
              (question.metadata?.['difficulty'] as QuestionDifficulty) ?? 'medium';
            questionsByDifficulty[difficulty]++;
          }
        }

        return {
          ...source,
          combinedPool,
          stats: {
            totalQuestions: combinedPool.length,
            questionsByContent,
            questionsByBlooms,
            questionsByDifficulty,
          },
        };
      })
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Question Selection
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Select questions from a pool based on criteria.
   *
   * @param pool - Questions to select from
   * @param options - Selection options
   * @returns Selection result with questions and notes
   */
  selectQuestions(pool: PerseusItem[], options: QuestionSelectionOptions): QuestionSelectionResult {
    let candidates = [...pool];
    const notes: string[] = [];

    // Apply filters
    if (options.bloomsLevels && options.bloomsLevels.length > 0) {
      candidates = candidates.filter(q =>
        options.bloomsLevels!.includes((q.metadata?.['bloomsLevel'] as BloomsLevel) ?? 'understand')
      );
    }

    if (options.difficulty && options.difficulty.length > 0) {
      candidates = candidates.filter(q =>
        options.difficulty!.includes((q.metadata?.['difficulty'] as QuestionDifficulty) ?? 'medium')
      );
    }

    if (options.tags && options.tags.length > 0) {
      candidates = candidates.filter(q => q.metadata?.tags?.some(t => options.tags!.includes(t)));
    }

    if (options.excludeIds && options.excludeIds.length > 0) {
      candidates = candidates.filter(q => !options.excludeIds!.includes(q.id));
    }

    // Apply weighted selection if specified
    if (options.weightedContentIds && options.weightedContentIds.size > 0) {
      candidates = this.applyWeighting(candidates, options.weightedContentIds);
    }

    // Randomize if requested
    if (options.randomize) {
      this.shuffleArray(candidates);
    }

    // Ensure variety if requested
    if (options.ensureVariety) {
      candidates = this.ensureVariety(candidates);
    }

    // Select requested count
    const selected = candidates.slice(0, options.count);
    const selectionComplete = selected.length >= options.count;

    if (!selectionComplete) {
      notes.push(`Only ${selected.length} questions available (requested ${options.count})`);
    }

    // Get content IDs represented
    const contentIds = [...new Set(selected.map(q => q.metadata?.assessesContentId ?? ''))];

    return {
      questions: selected,
      selectionComplete,
      selectionNotes: notes.length > 0 ? notes : undefined,
      contentIds,
    };
  }

  /**
   * Select questions for a practice quiz.
   *
   * Practice quizzes are unlimited and draw from the content hierarchy.
   *
   * @param source - Hierarchical question source
   * @param count - Number of questions to select
   * @returns Selected questions
   */
  selectPracticeQuestions(source: HierarchicalQuestionSource, count = 5): QuestionSelectionResult {
    return this.selectQuestions(source.combinedPool, {
      count,
      randomize: true,
      ensureVariety: true,
      bloomsLevels: ['remember', 'understand', 'apply'],
    });
  }

  /**
   * Select questions for a mastery quiz.
   *
   * Mastery quizzes weight toward practiced content and include
   * higher-level Bloom's taxonomy questions.
   *
   * @param pool - Question pool
   * @param count - Number of questions
   * @param practicedContentIds - Content IDs the learner has practiced
   * @returns Selected questions
   */
  selectMasteryQuestions(
    pool: PerseusItem[],
    count = 5,
    practicedContentIds: string[] = []
  ): QuestionSelectionResult {
    // Weight toward practiced content (2x for practiced)
    const weights = new Map<string, number>();
    for (const id of practicedContentIds) {
      weights.set(id, 2);
    }

    return this.selectQuestions(pool, {
      count,
      randomize: true,
      ensureVariety: true,
      weightedContentIds: weights,
      bloomsLevels: ['understand', 'apply', 'analyze'],
    });
  }

  /**
   * Select questions for inline quiz (post-content attestation).
   *
   * @param contentId - Content being assessed
   * @param maxQuestions - Maximum questions to draw
   * @returns Observable of selected questions
   */
  selectInlineQuestions(contentId: string, maxQuestions = 10): Observable<QuestionSelectionResult> {
    return this.getPoolForContent(contentId).pipe(
      map(pool => {
        if (!pool || pool.questions.length === 0) {
          return {
            questions: [],
            selectionComplete: false,
            selectionNotes: ['No questions available for this content'],
            contentIds: [],
          };
        }

        return this.selectQuestions(pool.questions, {
          count: maxQuestions,
          randomize: true,
          bloomsLevels: ['remember', 'understand'],
        });
      })
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pool Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if content has enough questions for practice.
   */
  canPracticeContent(contentId: string): Observable<boolean> {
    return this.getPoolForContent(contentId).pipe(map(pool => pool !== null && canPractice(pool)));
  }

  /**
   * Check if content has enough questions for mastery.
   */
  canMasteryContent(contentId: string): Observable<boolean> {
    return this.getPoolForContent(contentId).pipe(map(pool => pool !== null && canMastery(pool)));
  }

  /**
   * Get pool completeness percentage.
   */
  getPoolCompleteness(contentId: string): Observable<number> {
    return this.getPoolForContent(contentId).pipe(
      map(pool => (pool ? calculateCompleteness(pool) : 0))
    );
  }

  /**
   * Search for pools matching criteria.
   */
  searchPools(query: QuestionPoolQuery): Observable<QuestionPool[]> {
    // This would typically search via backend
    // For now, load all specified content IDs
    if (!query.contentIds || query.contentIds.length === 0) {
      return of([]);
    }

    return this.getPoolsForContents(query.contentIds).pipe(
      map(poolMap => {
        let pools = Array.from(poolMap.values());

        // Apply filters
        if (query.minQuestions !== undefined) {
          pools = pools.filter(p => p.questions.length >= query.minQuestions!);
        }

        if (query.isComplete !== undefined) {
          pools = pools.filter(p => p.metadata.isComplete === query.isComplete);
        }

        if (query.tags && query.tags.length > 0) {
          pools = pools.filter(p => query.tags!.some(t => p.metadata.tags.includes(t)));
        }

        return pools;
      })
    );
  }

  /**
   * Clear pool cache.
   */
  clearCache(): void {
    this.poolCache.clear();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private loadPool(contentId: string): Observable<QuestionPool | null> {
    // Try to load from data loader (questions stored with content)
    // This could be:
    // 1. A separate question-pool file
    // 2. Questions embedded in content metadata
    // 3. Questions from Holochain DHT

    // For now, try loading from questions/{contentId}.json
    const poolPath = `questions/${contentId}.json`;

    return this.dataLoader.getContent(poolPath).pipe(
      map(content => {
        if (!content || content.contentType === 'placeholder') return null;
        // Parse content if it's a string
        if (typeof content.content === 'string') {
          try {
            return JSON.parse(content.content);
          } catch {
            return null;
          }
        }
        return content.content;
      }),
      map(data => {
        if (!data) return null;

        // Handle both pool format and raw questions array
        if (Array.isArray(data)) {
          // Raw questions array - wrap in pool
          return {
            ...createEmptyPool(contentId),
            questions: data,
            metadata: {
              ...createEmptyPool(contentId).metadata,
              isComplete: data.length >= 5,
            },
          };
        }

        return data as QuestionPool;
      }),
      catchError(() => of(null))
    );
  }

  private createEmptyHierarchicalSource(
    pathId: string,
    currentSectionId: string
  ): HierarchicalQuestionSource {
    return {
      currentContentId: currentSectionId,
      pathId,
      sectionId: currentSectionId,
      eligibleContentIds: [],
      combinedPool: [],
      stats: this.createEmptyStats(),
    };
  }

  private createEmptyStats(): HierarchicalPoolStats {
    return {
      totalQuestions: 0,
      questionsByContent: new Map(),
      questionsByBlooms: {
        remember: 0,
        understand: 0,
        apply: 0,
        analyze: 0,
        evaluate: 0,
        create: 0,
      },
      questionsByDifficulty: {
        easy: 0,
        medium: 0,
        hard: 0,
      },
    };
  }

  private applyWeighting(candidates: PerseusItem[], weights: Map<string, number>): PerseusItem[] {
    // Duplicate questions based on weight
    const weighted: PerseusItem[] = [];

    for (const question of candidates) {
      const contentId = question.metadata?.assessesContentId ?? '';
      const weight = weights.get(contentId) ?? 1;

      for (let i = 0; i < weight; i++) {
        weighted.push(question);
      }
    }

    return weighted;
  }

  private ensureVariety(candidates: PerseusItem[]): PerseusItem[] {
    // Reorder to spread out questions from the same content
    const byContent = new Map<string, PerseusItem[]>();

    for (const q of candidates) {
      const contentId = q.metadata?.assessesContentId ?? 'unknown';
      const existing = byContent.get(contentId) ?? [];
      existing.push(q);
      byContent.set(contentId, existing);
    }

    // Interleave questions from different content
    const result: PerseusItem[] = [];
    const contentQueues = Array.from(byContent.values());

    while (contentQueues.some(q => q.length > 0)) {
      for (const queue of contentQueues) {
        if (queue.length > 0) {
          result.push(queue.shift()!);
        }
      }
    }

    return result;
  }

  private shuffleArray<T>(array: T[]): void {
    for (let i = array.length - 1; i > 0; i--) {
      const randomValue = crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32;
      const j = Math.floor(randomValue * (i + 1));
      [array[i], array[j]] = [array[j], array[i]];
    }
  }
}
