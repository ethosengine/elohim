/**
 * MasteryService - Domain service for content mastery tracking via elohim-storage.
 *
 * This service provides high-level operations for Bloom's taxonomy mastery:
 * - Querying mastery state for humans and content
 * - Recording mastery level changes
 * - Finding content that needs refresh
 *
 * NOTE: This service uses the elohim-storage SQLite backend via StorageApiService.
 * For local/Holochain dual-backend mastery, see ContentMasteryService.
 *
 * Uses StorageApiService for HTTP communication with elohim-storage.
 */

import { Injectable } from '@angular/core';

import { map } from 'rxjs/operators';

import { Observable, forkJoin, of } from 'rxjs';

import { ContentMasteryView } from '@app/elohim/adapters/storage-types.adapter';
import { StorageApiService } from '@app/elohim/services/storage-api.service';

/**
 * Mastery level constants (Bloom's Taxonomy)
 */
export const MasteryLevels = {
  NOT_STARTED: 'not_started',
  AWARE: 'aware',
  UNDERSTANDING: 'understanding',
  APPLYING: 'applying',
  ANALYZING: 'analyzing',
  EVALUATING: 'evaluating',
  MASTERED: 'mastered',
} as const;

export type MasteryLevelType = (typeof MasteryLevels)[keyof typeof MasteryLevels];

/**
 * Mastery level ordering for comparison
 */
export const MASTERY_LEVEL_ORDER: Record<MasteryLevelType, number> = {
  not_started: 0,
  aware: 1,
  understanding: 2,
  applying: 3,
  analyzing: 4,
  evaluating: 5,
  mastered: 6,
};

@Injectable({
  providedIn: 'root',
})
export class MasteryService {
  constructor(private readonly storageApi: StorageApiService) {}

  // ===========================================================================
  // Query Methods
  // ===========================================================================

  /**
   * Get all mastery records for a human.
   */
  getMasteryForHuman(humanId: string): Observable<ContentMasteryView[]> {
    return this.storageApi.getMasteryRecords({ humanId });
  }

  /**
   * Get mastery for a specific content item.
   */
  getMasteryForContent(humanId: string, contentId: string): Observable<ContentMasteryView | null> {
    return this.storageApi
      .getMasteryRecords({ humanId, contentId })
      .pipe(map(records => (records.length > 0 ? records[0] : null)));
  }

  /**
   * Get mastery state for multiple content IDs.
   * Returns a map of contentId -> mastery record.
   */
  getMasteryState(
    humanId: string,
    contentIds: string[]
  ): Observable<Map<string, ContentMasteryView>> {
    if (contentIds.length === 0) {
      return of(new Map());
    }

    // Fetch all mastery for this human then filter
    return this.getMasteryForHuman(humanId).pipe(
      map(records => {
        const contentIdSet = new Set(contentIds);
        const result = new Map<string, ContentMasteryView>();
        for (const record of records) {
          if (contentIdSet.has(record.contentId)) {
            result.set(record.contentId, record);
          }
        }
        return result;
      })
    );
  }

  /**
   * Get mastery records at or above a minimum level.
   */
  getMasteryAtLevel(humanId: string, minLevel: MasteryLevelType): Observable<ContentMasteryView[]> {
    return this.storageApi.getMasteryRecords({ humanId, minLevel });
  }

  /**
   * Get content items that need refresh (freshness decayed).
   */
  getRefreshNeeded(humanId: string): Observable<ContentMasteryView[]> {
    return this.storageApi.getMasteryRecords({
      humanId,
      needsRefresh: true,
    });
  }

  // ===========================================================================
  // Mutation Methods
  // ===========================================================================

  /**
   * Record a mastery engagement (view, practice, assess, etc.).
   * This will create or update the mastery record.
   */
  recordEngagement(
    humanId: string,
    contentId: string,
    engagementType: string
  ): Observable<ContentMasteryView> {
    return this.storageApi.upsertMastery({
      humanId,
      contentId,
      engagementType,
    });
  }

  /**
   * Update mastery level directly (e.g., after assessment).
   */
  updateMasteryLevel(
    humanId: string,
    contentId: string,
    masteryLevel: MasteryLevelType
  ): Observable<ContentMasteryView> {
    return this.storageApi.upsertMastery({
      humanId,
      contentId,
      masteryLevel,
    });
  }

  /**
   * Record mastery for multiple content items at once.
   */
  recordBulkEngagement(
    humanId: string,
    contentIds: string[],
    engagementType: string
  ): Observable<ContentMasteryView[]> {
    if (contentIds.length === 0) {
      return of([]);
    }

    const requests = contentIds.map(contentId =>
      this.recordEngagement(humanId, contentId, engagementType)
    );

    return forkJoin(requests);
  }

  // ===========================================================================
  // Utility Methods
  // ===========================================================================

  /**
   * Compare mastery levels.
   * Returns true if level1 >= level2.
   */
  isMasteryAtLeast(level1: MasteryLevelType, level2: MasteryLevelType): boolean {
    return MASTERY_LEVEL_ORDER[level1] >= MASTERY_LEVEL_ORDER[level2];
  }

  /**
   * Get the next mastery level.
   */
  getNextLevel(currentLevel: MasteryLevelType): MasteryLevelType | null {
    const levels = Object.values(MasteryLevels);
    const currentIndex = levels.indexOf(currentLevel);
    if (currentIndex < 0 || currentIndex >= levels.length - 1) {
      return null;
    }
    return levels[currentIndex + 1];
  }

  /**
   * Calculate mastery progress as a percentage (0-100).
   */
  getMasteryProgress(level: MasteryLevelType): number {
    const order = MASTERY_LEVEL_ORDER[level];
    const maxOrder = MASTERY_LEVEL_ORDER.mastered;
    return Math.round((order / maxOrder) * 100);
  }

  /**
   * Check if content has been started (any engagement).
   */
  hasStarted(mastery: ContentMasteryView | null): boolean {
    if (!mastery) return false;
    return mastery.masteryLevel !== 'not_started';
  }

  /**
   * Check if content is fully mastered.
   */
  isMastered(mastery: ContentMasteryView | null): boolean {
    if (!mastery) return false;
    return mastery.masteryLevel === 'mastered';
  }

  /**
   * Sort mastery records by level (highest first).
   */
  sortByLevel(records: ContentMasteryView[]): ContentMasteryView[] {
    return [...records].sort((a, b) => {
      const levelA = MASTERY_LEVEL_ORDER[a.masteryLevel as MasteryLevelType] ?? 0;
      const levelB = MASTERY_LEVEL_ORDER[b.masteryLevel as MasteryLevelType] ?? 0;
      return levelB - levelA;
    });
  }

  /**
   * Sort mastery records by freshness (needs refresh first).
   */
  sortByFreshness(records: ContentMasteryView[]): ContentMasteryView[] {
    return [...records].sort((a, b) => a.freshnessScore - b.freshnessScore);
  }
}
