import { Injectable } from '@angular/core';

import { catchError, tap } from 'rxjs/operators';

import { BehaviorSubject, Observable, map, from, of } from 'rxjs';

import { isAboveGate, compareMasteryLevels } from '@app/elohim/models/agent.model';
import { MasteryRecordContent, SourceChainEntry } from '@app/elohim/models/source-chain.model';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { LearnerBackendService } from '@app/elohim/services/learner-backend.service';
import { LocalSourceChainService } from '@app/elohim/services/local-source-chain.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';

import {
  ContentMastery,
  EngagementType,
  ContentPrivilege,
  PrivilegeType,
  MasteryStats,
  PRIVILEGE_REQUIREMENTS,
  FRESHNESS_THRESHOLDS,
  DECAY_RATES,
  AssessmentEvidence,
  LevelProgressionEvent,
  MasteryLevel,
  PathMasteryOverview,
  transformMasteryFromWire,
} from '../models';

/** Result of migration from local to backend */
export interface MigrationResult {
  migrated: number;
  failed: number;
  success: boolean;
  errors: string[];
}

/**
 * ContentMasteryService - Manages Bloom's Taxonomy mastery tracking.
 *
 * Dual Backend Architecture:
 * - Visitor mode: localStorage via LocalSourceChainService (no account needed)
 * - Hosted account: Migrate localStorage → backend (adoption funnel)
 * - Native mode: Full backend experience
 *
 * Philosophy:
 * - All mastery data is stored on the agent's source chain
 * - Mastery entries are immutable (new entries for level changes)
 * - Freshness decays over time based on level-specific rates
 * - Privileges unlock at specific Bloom's levels
 *
 * Source Chain Entries:
 * - 'mastery-record': Each mastery level achievement
 * - Links from content → mastery records via 'mastery-for-content'
 */
@Injectable({ providedIn: 'root' })
export class ContentMasteryService {
  private readonly masteryCache = new Map<string, ContentMastery>();
  private readonly masterySubject = new BehaviorSubject<Map<string, ContentMastery>>(new Map());

  public readonly mastery$: Observable<Map<string, ContentMastery>> =
    this.masterySubject.asObservable();

  constructor(
    private readonly sourceChain: LocalSourceChainService,
    private readonly sessionHuman: SessionHumanService,
    private readonly backend: LearnerBackendService,
    private readonly holochainClient: HolochainClientService
  ) {
    // Initialize when session is available
    this.sessionHuman.session$.subscribe(session => {
      if (session) {
        this.initializeForSession(session.sessionId);
      }
    });
  }

  // =========================================================================
  // BACKEND SELECTION
  // =========================================================================

  /**
   * Check if backend is available for mastery operations.
   */
  isBackendAvailable(): boolean {
    return this.holochainClient.isConnected();
  }

  /**
   * Get the active storage backend.
   */
  private get storageBackend(): 'local' | 'backend' {
    return this.isBackendAvailable() ? 'backend' : 'local';
  }

  // =========================================================================
  // INITIALIZATION
  // =========================================================================

  /**
   * Initialize mastery tracking for a session.
   */
  private initializeForSession(sessionId: string): void {
    // Ensure source chain is initialized for this agent
    if (!this.sourceChain.isInitialized()) {
      this.sourceChain.initializeForAgent(sessionId);
    }

    // Load existing mastery records into cache
    this.loadMasteryCache();
  }

  /**
   * Load all mastery records from source chain into cache.
   */
  private loadMasteryCache(): void {
    const entries = this.sourceChain.getEntriesByType<MasteryRecordContent>('mastery-record');

    // Group by contentId, keeping latest entry for each
    const masteryMap = new Map<string, ContentMastery>();

    for (const entry of entries) {
      const contentId = entry.content.contentId;
      const existing = masteryMap.get(contentId);

      // Only keep latest entry per content
      if (!existing || new Date(entry.timestamp) > new Date(existing.levelAchievedAt)) {
        masteryMap.set(contentId, this.entryToMastery(entry));
      }
    }

    this.masteryCache.clear();
    masteryMap.forEach((mastery, contentId) => {
      // Compute current freshness
      mastery.freshness = this.computeFreshness(mastery);
      mastery.needsRefresh = mastery.freshness < FRESHNESS_THRESHOLDS.FRESH;
      this.masteryCache.set(contentId, mastery);
    });

    this.masterySubject.next(new Map(this.masteryCache));
  }

  /**
   * Convert source chain entry to ContentMastery model.
   */
  private entryToMastery(entry: SourceChainEntry<MasteryRecordContent>): ContentMastery {
    const content = entry.content;
    return {
      contentId: content.contentId,
      humanId: entry.authorAgent,
      level: content.level as MasteryLevel,
      levelAchievedAt: content.levelAchievedAt,
      levelHistory: [], // Would need to query all entries for full history
      lastEngagementAt: content.lastEngagementAt,
      lastEngagementType: content.lastEngagementType as EngagementType,
      contentVersionAtMastery: '', // Not tracked in MVP
      freshness: content.freshness,
      needsRefresh: false,
      assessmentEvidence: [],
      privileges: this.computePrivileges(content.level as MasteryLevel),
    };
  }

  // =========================================================================
  // MASTERY QUERIES
  // =========================================================================

  /**
   * Get mastery for a specific content node.
   */
  getMastery(contentId: string): Observable<ContentMastery | null> {
    return this.mastery$.pipe(map(cache => cache.get(contentId) ?? null));
  }

  /**
   * Get mastery synchronously (from cache).
   */
  getMasterySync(contentId: string): ContentMastery | null {
    return this.masteryCache.get(contentId) ?? null;
  }

  /**
   * Get mastery level for content.
   */
  getMasteryLevel(contentId: string): Observable<MasteryLevel> {
    return this.mastery$.pipe(map(cache => cache.get(contentId)?.level ?? 'not_started'));
  }

  /**
   * Get mastery level synchronously.
   */
  getMasteryLevelSync(contentId: string): MasteryLevel {
    return this.masteryCache.get(contentId)?.level ?? 'not_started';
  }

  /**
   * Get all mastery records.
   */
  getAllMastery(): Observable<ContentMastery[]> {
    return this.mastery$.pipe(map(cache => Array.from(cache.values())));
  }

  /**
   * Get mastery statistics.
   */
  getMasteryStats(): Observable<MasteryStats> {
    return this.mastery$.pipe(map(cache => this.computeStats(cache)));
  }

  /**
   * Get content needing refresh.
   */
  getContentNeedingRefresh(): Observable<ContentMastery[]> {
    return this.mastery$.pipe(map(cache => Array.from(cache.values()).filter(m => m.needsRefresh)));
  }

  // =========================================================================
  // MASTERY RECORDING
  // =========================================================================

  /**
   * Record a content view.
   * Upgrades to 'seen' if not_started.
   */
  recordView(contentId: string): void {
    const current = this.getMasteryLevelSync(contentId);

    if (current === 'not_started') {
      this.setMasteryLevel(contentId, 'seen', 'view');
    } else {
      // Just update engagement timestamp
      this.recordEngagement(contentId, 'view');
    }
  }

  /**
   * Record assessment result.
   * Upgrades mastery based on assessment type and score.
   */
  recordAssessment(
    contentId: string,
    assessmentType: AssessmentEvidence['assessmentType'],
    score: number
  ): MasteryLevel {
    const current = this.getMasteryLevelSync(contentId);
    let newLevel = current;

    // Determine level based on assessment type and passing score
    if (score >= 0.7) {
      // 70% passing threshold
      switch (assessmentType) {
        case 'recall':
          newLevel = this.maxLevel(current, 'remember');
          break;
        case 'comprehension':
          newLevel = this.maxLevel(current, 'understand');
          break;
        case 'application':
          newLevel = this.maxLevel(current, 'apply');
          break;
        case 'analysis':
          newLevel = this.maxLevel(current, 'analyze');
          break;
      }
    }

    if (compareMasteryLevels(newLevel, current) > 0) {
      this.setMasteryLevel(contentId, newLevel, 'quiz');
    } else {
      this.recordEngagement(contentId, 'quiz');
    }

    return newLevel;
  }

  /**
   * Set mastery level directly.
   */
  setMasteryLevel(
    contentId: string,
    level: MasteryLevel,
    engagementType: EngagementType = 'view'
  ): void {
    const now = new Date().toISOString();
    const current = this.masteryCache.get(contentId);

    // Create mastery record on source chain
    const masteryContent: MasteryRecordContent = {
      contentId,
      level,
      levelAchievedAt: now,
      freshness: 1.0, // Fresh when just achieved
      lastEngagementAt: now,
      lastEngagementType: engagementType,
    };

    const entry = this.sourceChain.createEntry<MasteryRecordContent>(
      'mastery-record',
      masteryContent
    );

    // Update cache
    const mastery: ContentMastery = {
      contentId,
      humanId: entry.authorAgent,
      level,
      levelAchievedAt: now,
      levelHistory: current
        ? [
            ...(current.levelHistory || []),
            {
              fromLevel: current.level,
              toLevel: level,
              timestamp: now,
              trigger: engagementType === 'quiz' ? 'assessment' : 'engagement',
            } as LevelProgressionEvent,
          ]
        : [],
      lastEngagementAt: now,
      lastEngagementType: engagementType,
      contentVersionAtMastery: '',
      freshness: 1.0,
      needsRefresh: false,
      assessmentEvidence: current?.assessmentEvidence ?? [],
      privileges: this.computePrivileges(level),
    };

    this.masteryCache.set(contentId, mastery);
    this.masterySubject.next(new Map(this.masteryCache));

    // Record activity in session
    this.sessionHuman.recordContentView(contentId);
  }

  /**
   * Record engagement without level change.
   */
  private recordEngagement(contentId: string, type: EngagementType): void {
    const existing = this.masteryCache.get(contentId);
    if (!existing) return;

    const now = new Date().toISOString();

    // Update cached mastery
    existing.lastEngagementAt = now;
    existing.lastEngagementType = type;
    existing.freshness = this.computeFreshness(existing);
    existing.needsRefresh = existing.freshness < FRESHNESS_THRESHOLDS.FRESH;

    this.masteryCache.set(contentId, { ...existing });
    this.masterySubject.next(new Map(this.masteryCache));
  }

  // =========================================================================
  // PRIVILEGES
  // =========================================================================

  /**
   * Check if human has a specific privilege for content.
   */
  hasPrivilege(contentId: string, privilege: PrivilegeType): Observable<boolean> {
    return this.mastery$.pipe(
      map(cache => {
        const mastery = cache.get(contentId);
        if (!mastery) {
          // Default privileges available without mastery
          return privilege === 'view' || privilege === 'practice';
        }

        const requiredLevel = PRIVILEGE_REQUIREMENTS[privilege];
        return compareMasteryLevels(mastery.level, requiredLevel) >= 0;
      })
    );
  }

  /**
   * Check privilege synchronously.
   */
  hasPrivilegeSync(contentId: string, privilege: PrivilegeType): boolean {
    const mastery = this.masteryCache.get(contentId);
    if (!mastery) {
      return privilege === 'view' || privilege === 'practice';
    }

    const requiredLevel = PRIVILEGE_REQUIREMENTS[privilege];
    return compareMasteryLevels(mastery.level, requiredLevel) >= 0;
  }

  /**
   * Get all privileges for content.
   */
  getPrivileges(contentId: string): Observable<ContentPrivilege[]> {
    return this.mastery$.pipe(
      map(cache => {
        const mastery = cache.get(contentId);
        return mastery?.privileges ?? this.computePrivileges('not_started');
      })
    );
  }

  /**
   * Compute privileges for a level.
   */
  private computePrivileges(level: MasteryLevel): ContentPrivilege[] {
    const privileges: ContentPrivilege[] = [];
    const now = new Date().toISOString();

    for (const [privilegeType, requiredLevel] of Object.entries(PRIVILEGE_REQUIREMENTS)) {
      const granted = compareMasteryLevels(level, requiredLevel) >= 0;
      privileges.push({
        privilege: privilegeType as PrivilegeType,
        grantedAt: granted ? now : '',
        grantedByLevel: level,
        active: granted,
      });
    }

    return privileges;
  }

  // =========================================================================
  // FRESHNESS
  // =========================================================================

  /**
   * Compute freshness for mastery.
   * Uses exponential decay based on time and level-specific decay rate.
   */
  computeFreshness(mastery: ContentMastery): number {
    const lastEngagement = new Date(mastery.lastEngagementAt).getTime();
    const now = Date.now();
    const daysSinceEngagement = (now - lastEngagement) / (1000 * 60 * 60 * 24);

    const decayRate = DECAY_RATES[mastery.level];
    const freshness = Math.exp(-decayRate * daysSinceEngagement);

    return Math.max(0, Math.min(1, freshness));
  }

  /**
   * Refresh all freshness values in cache.
   */
  refreshAllFreshness(): void {
    for (const [, mastery] of this.masteryCache) {
      mastery.freshness = this.computeFreshness(mastery);
      mastery.needsRefresh = mastery.freshness < FRESHNESS_THRESHOLDS.FRESH;

      // Determine refresh type
      if (mastery.freshness < FRESHNESS_THRESHOLDS.CRITICAL) {
        mastery.refreshType = 'relearn';
      } else if (mastery.freshness < FRESHNESS_THRESHOLDS.STALE) {
        mastery.refreshType = 'retest';
      } else if (mastery.needsRefresh) {
        mastery.refreshType = 'review';
      } else {
        mastery.refreshType = undefined;
      }
    }

    this.masterySubject.next(new Map(this.masteryCache));
  }

  // =========================================================================
  // STATISTICS
  // =========================================================================

  /**
   * Compute mastery statistics.
   */
  private computeStats(cache: Map<string, ContentMastery>): MasteryStats {
    const stats: MasteryStats = {
      humanId: this.sourceChain.isInitialized() ? this.sourceChain.getAgentId() : '',
      computedAt: new Date().toISOString(),
      totalMasteredNodes: 0,
      levelDistribution: {
        not_started: 0,
        seen: 0,
        remember: 0,
        understand: 0,
        apply: 0,
        analyze: 0,
        evaluate: 0,
        create: 0,
      },
      nodesAboveGate: 0,
      freshPercentage: 0,
      nodesNeedingRefresh: 0,
      byCategory: new Map(),
      byType: new Map(),
    };

    let totalFreshness = 0;

    for (const mastery of cache.values()) {
      // Skip not_started (shouldn't be in cache, but be safe)
      if (mastery.level === 'not_started') continue;

      stats.totalMasteredNodes++;
      stats.levelDistribution[mastery.level]++;

      if (isAboveGate(mastery.level)) {
        stats.nodesAboveGate++;
      }

      totalFreshness += mastery.freshness;

      if (mastery.needsRefresh) {
        stats.nodesNeedingRefresh++;
      }
    }

    stats.freshPercentage =
      stats.totalMasteredNodes > 0 ? (totalFreshness / stats.totalMasteredNodes) * 100 : 100;

    return stats;
  }

  // =========================================================================
  // HELPERS
  // =========================================================================

  /**
   * Get the higher of two mastery levels.
   */
  private maxLevel(a: MasteryLevel, b: MasteryLevel): MasteryLevel {
    return compareMasteryLevels(a, b) >= 0 ? a : b;
  }

  /**
   * Check if content is at or above attestation gate.
   */
  isAboveGate(contentId: string): boolean {
    const mastery = this.masteryCache.get(contentId);
    return mastery ? isAboveGate(mastery.level) : false;
  }

  // =========================================================================
  // BACKEND INTEGRATION
  // =========================================================================

  /**
   * Get path mastery overview (Khan Academy-style grid).
   * Only available when backend is connected.
   */
  getPathMasteryOverview(pathId: string): Observable<PathMasteryOverview | null> {
    if (!this.isBackendAvailable()) {
      console.warn('[ContentMastery] Backend not available for path overview');
      return of(null);
    }

    return from(this.backend.getPathMasteryOverview(pathId)).pipe(
      catchError(err => {
        console.warn('[ContentMastery] Failed to get path overview:', err);
        return of(null);
      })
    );
  }

  /**
   * Sync a mastery record to backend.
   * Called automatically when backend becomes available.
   */
  private async syncMasteryToBackend(mastery: ContentMastery): Promise<boolean> {
    if (!this.isBackendAvailable()) return false;

    try {
      // Initialize mastery on backend
      await this.backend.initializeMastery(mastery.contentId);

      // Record engagement to set the level
      if (mastery.level !== 'not_started') {
        await this.backend.recordEngagement({
          content_id: mastery.contentId,
          engagement_type: mastery.lastEngagementType,
        });
      }

      return true;
    } catch (err) {
      console.warn('[ContentMastery] Failed to sync to backend:', err);
      return false;
    }
  }

  // =========================================================================
  // MIGRATION: Visitor → Hosted Account
  // =========================================================================

  /**
   * Migrate visitor's localStorage progress to their new backend account.
   * Called when visitor creates a Hosted-level account.
   *
   * @returns Migration result with counts and errors
   */
  async migrateToBackend(): Promise<MigrationResult> {
    const result: MigrationResult = {
      migrated: 0,
      failed: 0,
      success: false,
      errors: [],
    };

    if (!this.isBackendAvailable()) {
      result.errors.push('Backend not available');
      return result;
    }

    // Get all local mastery records
    const localRecords = this.sourceChain.getEntriesByType<MasteryRecordContent>('mastery-record');
    if (localRecords.length === 0) {
      result.success = true;
      return result;
    }

    console.log(`[ContentMastery] Migrating ${localRecords.length} mastery records to backend`);

    // Group by contentId (keep latest per content)
    const latestByContent = new Map<string, SourceChainEntry<MasteryRecordContent>>();
    for (const entry of localRecords) {
      const contentId = entry.content.contentId;
      const existing = latestByContent.get(contentId);
      if (!existing || new Date(entry.timestamp) > new Date(existing.timestamp)) {
        latestByContent.set(contentId, entry);
      }
    }

    // Migrate each unique content mastery
    for (const [contentId, entry] of latestByContent) {
      try {
        // Initialize mastery on backend
        const initialized = await this.backend.initializeMastery(contentId);
        if (!initialized) {
          result.failed++;
          result.errors.push(`Failed to initialize mastery for ${contentId}`);
          continue;
        }

        // If level is beyond 'seen', we need to record assessments to reach it
        // For now, we just record the engagement type
        if (entry.content.level !== 'not_started' && entry.content.level !== 'seen') {
          await this.backend.recordEngagement({
            content_id: contentId,
            engagement_type: entry.content.lastEngagementType,
          });
        }

        result.migrated++;
      } catch (err) {
        result.failed++;
        const errorMsg = err instanceof Error ? err.message : 'Unknown error';
        result.errors.push(`${contentId}: ${errorMsg}`);
      }
    }

    result.success = result.failed === 0;
    console.log(
      `[ContentMastery] Migration complete: ${result.migrated} migrated, ${result.failed} failed`
    );

    return result;
  }

  /**
   * Check if there are local records that need migration.
   */
  hasPendingMigration(): boolean {
    if (!this.isBackendAvailable()) return false;
    const localRecords = this.sourceChain.getEntriesByType<MasteryRecordContent>('mastery-record');
    return localRecords.length > 0;
  }

  /**
   * Get count of records pending migration.
   */
  getPendingMigrationCount(): number {
    const localRecords = this.sourceChain.getEntriesByType<MasteryRecordContent>('mastery-record');
    // Group by contentId to get unique count
    const uniqueContentIds = new Set(localRecords.map(r => r.content.contentId));
    return uniqueContentIds.size;
  }
}
