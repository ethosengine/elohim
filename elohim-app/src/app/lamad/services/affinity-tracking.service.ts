import { Injectable, Inject, Optional } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';
import {
  UserAffinity,
  AffinityStats,
  AffinityChangeEvent,
  CategoryAffinityStats,
  TypeAffinityStats,
} from '../models/user-affinity.model';
import { ContentNode } from '../models/content-node.model';
import { SessionUserService } from './session-user.service';

/**
 * Service for tracking user affinity (relationship strength) to content nodes.
 *
 * Integrates with SessionUserService for session-scoped storage.
 * Falls back to default storage key if no session.
 * Affinity values range from 0.0 (no connection) to 1.0 (strong connection).
 *
 * Holochain migration:
 * - Session: localStorage with session-specific key
 * - Holochain: Agent's private source chain
 */
@Injectable({
  providedIn: 'root',
})
export class AffinityTrackingService {
  private readonly DEFAULT_STORAGE_KEY = 'elohim-user-affinity';
  private readonly AUTO_INCREMENT_DELTA = 0.2; // Bump on first view
  private readonly AUTO_INCREMENT_THRESHOLD = 0.01; // Only auto-increment if below this

  private readonly affinitySubject = new BehaviorSubject<UserAffinity>(
    this.loadFromStorage()
  );
  private readonly changeSubject = new BehaviorSubject<AffinityChangeEvent | null>(null);

  public readonly affinity$: Observable<UserAffinity> =
    this.affinitySubject.asObservable();
  public readonly changes$: Observable<AffinityChangeEvent | null> =
    this.changeSubject.asObservable();

  constructor(
    @Optional() private sessionUserService: SessionUserService | null
  ) {
    // Re-load if session changes
    if (this.sessionUserService) {
      this.sessionUserService.session$.subscribe(session => {
        if (session) {
          const loaded = this.loadFromStorage();
          this.affinitySubject.next(loaded);
        }
      });
    }
  }

  /**
   * Get the storage key based on session context.
   */
  private getStorageKey(): string {
    if (this.sessionUserService) {
      return this.sessionUserService.getAffinityStorageKey();
    }
    return this.DEFAULT_STORAGE_KEY;
  }

  /**
   * Get the user ID based on session context.
   */
  private getUserId(): string {
    if (this.sessionUserService) {
      return this.sessionUserService.getSessionId() || 'anonymous';
    }
    return 'anonymous';
  }

  /**
   * Get affinity value for a specific node
   * @param nodeId The node ID
   * @returns Affinity value (0.0 to 1.0), defaults to 0.0
   */
  getAffinity(nodeId: string): number {
    return this.affinitySubject.value.affinity[nodeId] ?? 0.0;
  }

  /**
   * Set affinity value for a specific node
   * @param nodeId The node ID
   * @param value Affinity value (0.0 to 1.0)
   */
  setAffinity(nodeId: string, value: number): void {
    // Clamp value between 0.0 and 1.0
    const clampedValue = Math.max(0.0, Math.min(1.0, value));
    const oldValue = this.getAffinity(nodeId);

    if (oldValue === clampedValue) {
      return; // No change
    }

    const current = this.affinitySubject.value;
    const updated: UserAffinity = {
      ...current,
      affinity: {
        ...current.affinity,
        [nodeId]: clampedValue,
      },
      lastUpdated: new Date(),
    };

    this.affinitySubject.next(updated);
    this.saveToStorage(updated);

    // Emit change event
    this.changeSubject.next({
      nodeId,
      oldValue,
      newValue: clampedValue,
      timestamp: new Date(),
    });

    // Notify session service of affinity change
    if (this.sessionUserService) {
      this.sessionUserService.recordAffinityChange(nodeId, clampedValue);
    }
  }

  /**
   * Increment affinity value by a delta
   * @param nodeId The node ID
   * @param delta Amount to increment (can be negative to decrement)
   */
  incrementAffinity(nodeId: string, delta: number): void {
    const current = this.getAffinity(nodeId);
    this.setAffinity(nodeId, current + delta);
  }

  /**
   * Auto-increment affinity when user views content
   * Only increments if current affinity is below threshold
   * @param nodeId The node ID
   */
  trackView(nodeId: string): void {
    // Always record the view in session
    if (this.sessionUserService) {
      this.sessionUserService.recordContentView(nodeId);
    }

    const current = this.getAffinity(nodeId);
    if (current < this.AUTO_INCREMENT_THRESHOLD) {
      this.setAffinity(nodeId, this.AUTO_INCREMENT_DELTA);
    }
  }

  /**
   * Get aggregate statistics across all nodes
   * @param nodes All content nodes in the system
   */
  getStats(nodes: ContentNode[]): AffinityStats {
    const affinity = this.affinitySubject.value.affinity;
    const totalNodes = nodes.length;
    let engagedNodes = 0;
    let totalAffinity = 0;

    const distribution = {
      unseen: 0,
      low: 0,
      medium: 0,
      high: 0,
    };

    const categoryMap = new Map<string, number[]>();
    const typeMap = new Map<string, number[]>();

    nodes.forEach((node) => {
      const nodeAffinity = affinity[node.id] || 0.0;

      // Track engagement
      if (nodeAffinity > 0) {
        engagedNodes++;
      }
      totalAffinity += nodeAffinity;

      // Distribution
      if (nodeAffinity === 0) {
        distribution.unseen++;
      } else if (nodeAffinity <= 0.33) {
        distribution.low++;
      } else if (nodeAffinity <= 0.66) {
        distribution.medium++;
      } else {
        distribution.high++;
      }

      // By category
      const category = (node.metadata?.['category'] as string) || 'uncategorized';
      if (!categoryMap.has(category)) {
        categoryMap.set(category, []);
      }
      categoryMap.get(category)!.push(nodeAffinity);

      // By type
      const type = node.contentType;
      if (!typeMap.has(type)) {
        typeMap.set(type, []);
      }
      typeMap.get(type)!.push(nodeAffinity);
    });

    // Calculate category stats
    const byCategory = new Map<string, CategoryAffinityStats>();
    categoryMap.forEach((affinities, category) => {
      const engaged = affinities.filter((a) => a > 0).length;
      const avg =
        affinities.reduce((sum, a) => sum + a, 0) / affinities.length;
      byCategory.set(category, {
        category,
        nodeCount: affinities.length,
        averageAffinity: avg,
        engagedCount: engaged,
      });
    });

    // Calculate type stats
    const byType = new Map<string, TypeAffinityStats>();
    typeMap.forEach((affinities, type) => {
      const engaged = affinities.filter((a) => a > 0).length;
      const avg =
        affinities.reduce((sum, a) => sum + a, 0) / affinities.length;
      byType.set(type, {
        type,
        nodeCount: affinities.length,
        averageAffinity: avg,
        engagedCount: engaged,
      });
    });

    return {
      totalNodes,
      engagedNodes,
      averageAffinity: totalNodes > 0 ? totalAffinity / totalNodes : 0,
      distribution,
      byCategory,
      byType,
    };
  }

  /**
   * Reset all affinity data
   */
  reset(): void {
    const fresh: UserAffinity = {
      userId: this.getUserId(),
      affinity: {},
      lastUpdated: new Date(),
    };
    this.affinitySubject.next(fresh);
    this.saveToStorage(fresh);
  }

  /**
   * Load affinity data from localStorage
   */
  private loadFromStorage(): UserAffinity {
    const storageKey = this.getStorageKey();
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) {
        const parsed = JSON.parse(stored);
        // Restore Date object
        parsed.lastUpdated = new Date(parsed.lastUpdated);
        return parsed;
      }
    } catch (error) {
      console.error('Failed to load affinity from localStorage:', error);
    }

    // Return default
    return {
      userId: this.getUserId(),
      affinity: {},
      lastUpdated: new Date(),
    };
  }

  /**
   * Save affinity data to localStorage
   */
  private saveToStorage(affinity: UserAffinity): void {
    const storageKey = this.getStorageKey();
    try {
      localStorage.setItem(storageKey, JSON.stringify(affinity));
    } catch (error) {
      console.error('Failed to save affinity to localStorage:', error);
    }
  }
}
