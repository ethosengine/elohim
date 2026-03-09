import { Injectable, inject } from '@angular/core';

import { map } from 'rxjs/operators';

import { Observable, of } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { LearnerContextService } from './learner-context.service';

import type { PathIndexEntry } from '../models/learning-path.model';
import type { DiscoveryProfile } from '../quiz-engine/models/discovery-assessment.model';

/**
 * A ranked path recommendation derived from discovery profile analysis.
 */
export interface PathRecommendation {
  pathId: string;
  pathTitle: string;
  relevanceScore: number;
  matchedCategories: string[];
  reason: string;
}

/**
 * Framework-to-tag affinity map.
 *
 * When a learner completes a specific discovery framework, these extra
 * tags are boosted during path matching (beyond the framework's own category).
 */
const TAG_SELF_KNOWLEDGE = 'self-knowledge';

const FRAMEWORK_TAG_AFFINITY: Record<string, string[]> = {
  'values-hierarchy': ['values', TAG_SELF_KNOWLEDGE, 'ethics'],
  enneagram: ['personality', TAG_SELF_KNOWLEDGE, 'relational'],
  'clifton-strengths': ['strengths', 'vocational', TAG_SELF_KNOWLEDGE],
  'attachment-style': ['emotional', 'relational', TAG_SELF_KNOWLEDGE],
  'learning-style': ['learning', TAG_SELF_KNOWLEDGE],
};

/** Visibility levels that exclude a path from public recommendations. */
const EXCLUDED_VISIBILITIES = new Set(['intimate', 'private']);

/**
 * PathRecommendationService - Maps discovery profile traits to relevant learning paths.
 *
 * Scoring logic (no ML, pure tag overlap):
 * 1. For each path, compute overlap between path tags and completed discovery categories
 * 2. Bonus weight for framework-specific affinity matches
 * 3. Exclude intimate/private paths
 * 4. Return sorted descending by relevance score
 */
@Injectable({ providedIn: 'root' })
export class PathRecommendationService {
  private readonly dataLoader = inject(DataLoaderService);
  private readonly learnerContext = inject(LearnerContextService);

  /**
   * Get ranked path recommendations for a discovery profile.
   *
   * Loads path index, scores each path by tag overlap with the learner's
   * completed discovery categories, and returns sorted by relevance.
   */
  getRecommendations(profile: DiscoveryProfile): Observable<PathRecommendation[]> {
    if (profile.totalAssessments === 0) {
      return of([]);
    }

    return this.dataLoader.getPathIndex().pipe(
      map(index => {
        const completedCategories = this.getCompletedCategories(profile);
        const completedFrameworks = this.getCompletedFrameworks(profile);
        const affinityTags = this.buildAffinityTags(completedFrameworks);

        return index.paths
          .filter(entry => this.isRecommendable(entry))
          .map(entry => this.scorePath(entry, completedCategories, affinityTags))
          .filter(rec => rec.relevanceScore > 0)
          .sort((a, b) => b.relevanceScore - a.relevanceScore);
      })
    );
  }

  /**
   * Get the top recommendation for a discovery profile.
   * Convenience wrapper — returns null if no recommendations found.
   */
  getTopRecommendation(profile: DiscoveryProfile): Observable<PathRecommendation | null> {
    return this.getRecommendations(profile).pipe(map(recs => recs[0] ?? null));
  }

  // =========================================================================
  // Internals
  // =========================================================================

  /**
   * Extract completed discovery categories from a profile.
   * Categories with at least one result are considered "completed".
   */
  private getCompletedCategories(profile: DiscoveryProfile): Set<string> {
    const categories = new Set<string>();
    for (const [category, results] of Object.entries(profile.byCategory)) {
      if (results.length > 0) {
        categories.add(category);
      }
    }
    return categories;
  }

  /**
   * Extract unique frameworks from all completed discovery results.
   */
  private getCompletedFrameworks(profile: DiscoveryProfile): Set<string> {
    const frameworks = new Set<string>();
    for (const results of Object.values(profile.byCategory)) {
      for (const result of results) {
        frameworks.add(result.framework);
      }
    }
    return frameworks;
  }

  /**
   * Build the set of extra tags to boost based on completed frameworks.
   */
  private buildAffinityTags(frameworks: Set<string>): Set<string> {
    const tags = new Set<string>();
    for (const framework of frameworks) {
      const affinityList = FRAMEWORK_TAG_AFFINITY[framework];
      if (affinityList) {
        for (const tag of affinityList) {
          tags.add(tag);
        }
      }
    }
    return tags;
  }

  /**
   * Check if a path entry is eligible for recommendation.
   * Excludes intimate/private paths.
   */
  private isRecommendable(entry: PathIndexEntry): boolean {
    const visibility = (entry as { visibility?: string }).visibility;
    return !(visibility && EXCLUDED_VISIBILITIES.has(visibility));
  }

  /**
   * Score a path against the learner's discovery profile.
   *
   * Score = (category tag matches * 1.0) + (affinity tag matches * 0.5)
   * Normalized to 0-1 range based on total path tags.
   */
  private scorePath(
    entry: PathIndexEntry,
    completedCategories: Set<string>,
    affinityTags: Set<string>
  ): PathRecommendation {
    const pathTags = new Set(entry.tags ?? []);
    const matchedCategories: string[] = [];
    let rawScore = 0;

    // Direct category matches (weight 1.0 each)
    for (const category of completedCategories) {
      if (pathTags.has(category)) {
        matchedCategories.push(category);
        rawScore += 1;
      }
    }

    // Affinity tag matches (weight 0.5 each, no double-count)
    for (const tag of affinityTags) {
      if (pathTags.has(tag) && !completedCategories.has(tag)) {
        rawScore += 0.5;
      }
    }

    // Normalize: max possible = pathTags.size (if every tag matched at weight 1)
    const maxScore = Math.max(pathTags.size, 1);
    const relevanceScore = Math.min(rawScore / maxScore, 1);

    const reason =
      matchedCategories.length > 0
        ? `Matches your ${matchedCategories.join(', ')} discovery results`
        : 'Related to your discovery profile';

    return {
      pathId: entry.id,
      pathTitle: entry.title,
      relevanceScore,
      matchedCategories,
      reason,
    };
  }
}
