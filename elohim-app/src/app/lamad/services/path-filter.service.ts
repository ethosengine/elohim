import { Injectable } from '@angular/core';
import { PathIndexEntry } from '../models/learning-path.model';

/**
 * PathFilterService - Filters and selects paths for display.
 *
 * Used to reduce the number of paths shown on the homepage
 * and support future archetype-based human story selection.
 */
@Injectable({ providedIn: 'root' })
export class PathFilterService {

  /**
   * Get featured paths for homepage display.
   * Returns a limited number of paths, prioritizing:
   * 1. Beginner-friendly paths (for new visitors)
   * 2. Paths with certain featured tags
   * 3. Most recently updated (by order in index)
   */
  getFeaturedPaths(paths: PathIndexEntry[], limit: number = 6): PathIndexEntry[] {
    if (paths.length <= limit) {
      return paths;
    }

    // Priority scoring for featured selection
    const scored = paths.map(path => ({
      path,
      score: this.calculateFeaturedScore(path)
    }));

    // Sort by score descending, then take top N
    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, limit).map(s => s.path);
  }

  /**
   * Filter paths by tags (any match).
   */
  filterByTags(paths: PathIndexEntry[], tags: string[]): PathIndexEntry[] {
    if (!tags.length) {
      return paths;
    }
    const tagSet = new Set(tags.map(t => t.toLowerCase()));
    return paths.filter(path =>
      path.tags?.some(t => tagSet.has(t.toLowerCase()))
    );
  }

  /**
   * Filter paths by difficulty level.
   */
  filterByDifficulty(
    paths: PathIndexEntry[],
    levels: Array<'beginner' | 'intermediate' | 'advanced'>
  ): PathIndexEntry[] {
    if (!levels.length) {
      return paths;
    }
    const levelSet = new Set(levels);
    return paths.filter(path => levelSet.has(path.difficulty));
  }

  /**
   * Filter paths by category (when available).
   */
  filterByCategory(paths: PathIndexEntry[], category: string): PathIndexEntry[] {
    return paths.filter(path => path.category === category);
  }

  /**
   * Search paths by title/description text.
   */
  searchPaths(paths: PathIndexEntry[], query: string): PathIndexEntry[] {
    if (!query.trim()) {
      return paths;
    }
    const lowerQuery = query.toLowerCase();
    return paths.filter(path =>
      path.title.toLowerCase().includes(lowerQuery) ||
      path.description.toLowerCase().includes(lowerQuery) ||
      path.tags?.some(t => t.toLowerCase().includes(lowerQuery))
    );
  }

  /**
   * Calculate a score for featured path selection.
   * Higher scores = more likely to be featured.
   */
  private calculateFeaturedScore(path: PathIndexEntry): number {
    let score = 0;

    // Beginner paths get priority (more accessible)
    if (path.difficulty === 'beginner') {
      score += 30;
    } else if (path.difficulty === 'intermediate') {
      score += 20;
    } else if (path.difficulty === 'advanced') {
      score += 10;
    }

    // Paths with featured-related tags get a boost
    const featuredTags = ['featured', 'introduction', 'getting-started', 'overview', 'beginner-friendly'];
    const pathTagsLower = path.tags?.map(t => t.toLowerCase()) ?? [];
    for (const tag of featuredTags) {
      if (pathTagsLower.includes(tag)) {
        score += 15;
      }
    }

    // Shorter paths (< 10 steps) are more approachable
    if (path.stepCount <= 5) {
      score += 10;
    } else if (path.stepCount <= 10) {
      score += 5;
    }

    // Paths with descriptions are more useful
    if (path.description && path.description.length > 50) {
      score += 5;
    }

    return score;
  }
}
