import { Injectable } from '@angular/core';
import { Observable, of } from 'rxjs';
import { map, catchError } from 'rxjs/operators';
import { DataLoaderService } from './data-loader.service';
import { TrustBadgeService } from './trust-badge.service';
import { ContentType, ContentReach } from '../models/content-node.model';
import { TrustLevel, calculateTrustLevel } from '../models/trust-badge.model';
import { ContentIndexEntry } from './content.service';
import {
  SearchQuery,
  SearchResult,
  SearchResults,
  SearchFacets,
  FacetCount,
  SearchHighlight,
  MatchedField,
  SearchSuggestion,
  SearchSuggestions,
  SEARCH_FIELD_WEIGHTS,
  SEARCH_MATCH_BONUSES,
  DEFAULT_SEARCH_CONFIG,
  createEmptyResults,
  extractSnippet
} from '../models/search.model';

/**
 * SearchService - Enhanced content search with relevance scoring and facets.
 *
 * Features:
 * - Relevance-scored results (title > tags > description)
 * - Highlighted match snippets
 * - Faceted filtering (type, reach, trust, tags)
 * - Pagination
 * - Autocomplete suggestions
 *
 * Usage:
 * ```typescript
 * // Basic search
 * this.searchService.search({ text: 'governance' }).subscribe(results => {
 *   console.log(results.results); // Scored and highlighted results
 *   console.log(results.facets);  // Facet counts for filter UI
 * });
 *
 * // Filtered search
 * this.searchService.search({
 *   text: 'protocol',
 *   contentTypes: ['epic', 'feature'],
 *   minTrustScore: 0.5,
 *   page: 1,
 *   pageSize: 10
 * }).subscribe(results => ...);
 *
 * // Autocomplete
 * this.searchService.suggest('gov').subscribe(suggestions => ...);
 * ```
 */
@Injectable({ providedIn: 'root' })
export class SearchService {
  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly trustBadgeService: TrustBadgeService
  ) {}

  /**
   * Search content with relevance scoring, filtering, and facets.
   */
  search(query: SearchQuery): Observable<SearchResults> {
    const startTime = Date.now();

    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        const nodes = index.nodes ?? [];

        // Score and filter all nodes
        const scoredResults = this.scoreAndFilter(nodes, query);

        // Compute facets from ALL matching results (before pagination)
        const facets = this.computeFacets(scoredResults, query);

        // Sort results
        const sortedResults = this.sortResults(scoredResults, query);

        // Paginate
        const page = query.page ?? 1;
        const pageSize = Math.min(
          query.pageSize ?? DEFAULT_SEARCH_CONFIG.pageSize,
          DEFAULT_SEARCH_CONFIG.maxPageSize
        );
        const startIndex = (page - 1) * pageSize;
        const paginatedResults = sortedResults.slice(startIndex, startIndex + pageSize);

        const totalPages = Math.ceil(sortedResults.length / pageSize);

        return {
          query,
          results: paginatedResults,
          totalCount: sortedResults.length,
          page,
          pageSize,
          totalPages,
          hasMore: page < totalPages,
          facets,
          executionTimeMs: Date.now() - startTime
        };
      }),
      catchError(err => {
        console.error('[SearchService] Search failed:', err);
        return of({
          ...createEmptyResults(query),
          executionTimeMs: Date.now() - startTime
        });
      })
    );
  }

  /**
   * Get autocomplete suggestions for partial query.
   */
  suggest(partialQuery: string, limit: number = 10): Observable<SearchSuggestions> {
    if (!partialQuery || partialQuery.trim().length < 2) {
      return of({ query: partialQuery, suggestions: [] });
    }

    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        const nodes = index.nodes ?? [];
        const query = partialQuery.toLowerCase().trim();
        const suggestions: SearchSuggestion[] = [];
        const seen = new Set<string>();

        // Suggest matching titles
        for (const node of nodes) {
          if (suggestions.length >= limit) break;

          const titleLower = node.title.toLowerCase();
          if (titleLower.includes(query) && !seen.has(titleLower)) {
            seen.add(titleLower);
            suggestions.push({
              text: node.title,
              type: 'title',
              highlight: this.highlightMatch(node.title, query)
            });
          }
        }

        // Suggest matching tags
        const tagCounts = new Map<string, number>();
        for (const node of nodes) {
          for (const tag of node.tags ?? []) {
            if (tag.toLowerCase().includes(query)) {
              tagCounts.set(tag, (tagCounts.get(tag) ?? 0) + 1);
            }
          }
        }

        // Sort tags by count and add to suggestions
        const sortedTags = Array.from(tagCounts.entries())
          .sort((a, b) => b[1] - a[1])
          .slice(0, limit - suggestions.length);

        for (const [tag, count] of sortedTags) {
          if (!seen.has(tag.toLowerCase())) {
            seen.add(tag.toLowerCase());
            suggestions.push({
              text: tag,
              type: 'tag',
              resultCount: count,
              highlight: this.highlightMatch(tag, query)
            });
          }
        }

        return { query: partialQuery, suggestions };
      }),
      catchError(() => of({ query: partialQuery, suggestions: [] }))
    );
  }

  /**
   * Get all unique tags with counts.
   */
  getTagCloud(): Observable<Array<{ tag: string; count: number }>> {
    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        const tagCounts = new Map<string, number>();

        for (const node of index.nodes ?? []) {
          for (const tag of node.tags ?? []) {
            tagCounts.set(tag, (tagCounts.get(tag) ?? 0) + 1);
          }
        }

        return Array.from(tagCounts.entries())
          .map(([tag, count]) => ({ tag, count }))
          .sort((a, b) => b.count - a.count);
      })
    );
  }

  // ===========================================================================
  // Scoring and Filtering
  // ===========================================================================

  /**
   * Score and filter nodes based on query.
   */
  private scoreAndFilter(nodes: ContentIndexEntry[], query: SearchQuery): SearchResult[] {
    const results: SearchResult[] = [];
    const searchText = (query.text ?? '').toLowerCase().trim();
    const searchWords = searchText.split(/\s+/).filter(w => w.length > 0);

    for (const node of nodes) {
      // Apply filters first (cheaper than scoring)
      if (!this.passesFilters(node, query)) {
        continue;
      }

      // Score the node
      const { score, matchedFields, highlights } = this.scoreNode(node, searchWords);

      // If there's search text, require a minimum score
      if (searchText && score === 0) {
        continue;
      }

      // Compute trust level
      const trustLevel = this.computeTrustLevel(node);

      results.push({
        id: node.id,
        title: node.title,
        description: node.description,
        contentType: node.contentType,
        tags: node.tags ?? [],
        reach: (node as any).reach ?? 'commons',
        trustScore: (node as any).trustScore ?? 1.0,
        trustLevel,
        hasFlags: ((node as any).flags ?? []).length > 0,
        relevanceScore: score,
        matchedFields,
        highlights,
        createdAt: (node as any).createdAt,
        updatedAt: (node as any).updatedAt
      });
    }

    return results;
  }

  /**
   * Check if node passes all query filters.
   */
  private passesFilters(node: ContentIndexEntry, query: SearchQuery): boolean {
    // Content type filter
    if (query.contentTypes && query.contentTypes.length > 0) {
      if (!query.contentTypes.includes(node.contentType)) {
        return false;
      }
    }

    // Reach level filter
    if (query.reachLevels && query.reachLevels.length > 0) {
      const nodeReach = (node as any).reach ?? 'commons';
      if (!query.reachLevels.includes(nodeReach)) {
        return false;
      }
    }

    // Trust level filter
    if (query.trustLevels && query.trustLevels.length > 0) {
      const trustLevel = this.computeTrustLevel(node);
      if (!query.trustLevels.includes(trustLevel)) {
        return false;
      }
    }

    // Tag filter (OR logic)
    if (query.tags && query.tags.length > 0) {
      const nodeTags = (node.tags ?? []).map(t => t.toLowerCase());
      const hasAnyTag = query.tags.some(t => nodeTags.includes(t.toLowerCase()));
      if (!hasAnyTag) {
        return false;
      }
    }

    // Required tags filter (AND logic)
    if (query.requiredTags && query.requiredTags.length > 0) {
      const nodeTags = (node.tags ?? []).map(t => t.toLowerCase());
      const hasAllTags = query.requiredTags.every(t => nodeTags.includes(t.toLowerCase()));
      if (!hasAllTags) {
        return false;
      }
    }

    // Minimum trust score
    if (query.minTrustScore !== undefined) {
      const trustScore = (node as any).trustScore ?? 1.0;
      if (trustScore < query.minTrustScore) {
        return false;
      }
    }

    // Exclude flagged
    if (query.excludeFlagged) {
      const flags = (node as any).flags ?? [];
      if (flags.length > 0) {
        return false;
      }
    }

    return true;
  }

  /**
   * Score a node against search words.
   */
  private scoreNode(
    node: ContentIndexEntry,
    searchWords: string[]
  ): { score: number; matchedFields: MatchedField[]; highlights: SearchHighlight[] } {
    if (searchWords.length === 0) {
      // No search text - return neutral score
      return { score: 50, matchedFields: [], highlights: [] };
    }

    let totalScore = 0;
    const matchedFields: MatchedField[] = [];
    const highlights: SearchHighlight[] = [];

    const titleLower = node.title.toLowerCase();
    const descLower = (node.description ?? '').toLowerCase();
    const tagsLower = (node.tags ?? []).map(t => t.toLowerCase());

    for (const word of searchWords) {
      // Title matching
      const titleMatchType = this.getMatchType(titleLower, word);
      if (titleMatchType) {
        const wordScore = SEARCH_FIELD_WEIGHTS.title * SEARCH_MATCH_BONUSES[titleMatchType];
        totalScore += wordScore;
        matchedFields.push({
          field: 'title',
          weight: wordScore,
          matchedText: word
        });
      }

      // Tag matching
      for (const tag of tagsLower) {
        const tagMatchType = this.getMatchType(tag, word);
        if (tagMatchType) {
          const wordScore = SEARCH_FIELD_WEIGHTS.tags * SEARCH_MATCH_BONUSES[tagMatchType];
          totalScore += wordScore;
          matchedFields.push({
            field: 'tags',
            weight: wordScore,
            matchedText: word
          });
          break; // Only count once per word
        }
      }

      // Description matching
      const descMatchType = this.getMatchType(descLower, word);
      if (descMatchType) {
        const wordScore = SEARCH_FIELD_WEIGHTS.description * SEARCH_MATCH_BONUSES[descMatchType];
        totalScore += wordScore;
        matchedFields.push({
          field: 'description',
          weight: wordScore,
          matchedText: word
        });
      }
    }

    // Normalize score to 0-100
    const maxPossibleScore = searchWords.length * (
      SEARCH_FIELD_WEIGHTS.title * SEARCH_MATCH_BONUSES.exactMatch +
      SEARCH_FIELD_WEIGHTS.tags * SEARCH_MATCH_BONUSES.exactMatch +
      SEARCH_FIELD_WEIGHTS.description * SEARCH_MATCH_BONUSES.exactMatch
    );
    const normalizedScore = Math.round((totalScore / maxPossibleScore) * 100);

    // Generate highlights
    const queryText = searchWords.join(' ');

    if (matchedFields.some(f => f.field === 'title')) {
      highlights.push({
        field: 'title',
        ...extractSnippet(node.title, queryText)
      });
    }

    if (matchedFields.some(f => f.field === 'description') && node.description) {
      highlights.push({
        field: 'description',
        ...extractSnippet(node.description, queryText)
      });
    }

    if (matchedFields.some(f => f.field === 'tags')) {
      const matchingTags = (node.tags ?? []).filter(tag =>
        searchWords.some(w => tag.toLowerCase().includes(w))
      );
      if (matchingTags.length > 0) {
        highlights.push({
          field: 'tags',
          snippet: matchingTags.join(', '),
          matchRanges: [] // Tags don't need ranges
        });
      }
    }

    return { score: normalizedScore, matchedFields, highlights };
  }

  /**
   * Determine match type for scoring.
   */
  private getMatchType(text: string, word: string): keyof typeof SEARCH_MATCH_BONUSES | null {
    // Check for exact word match (word boundaries)
    const exactRegex = new RegExp(`\\b${this.escapeRegex(word)}\\b`, 'i');
    if (exactRegex.test(text)) {
      return 'exactMatch';
    }

    // Check for prefix match (word starts with search term)
    const prefixRegex = new RegExp(`\\b${this.escapeRegex(word)}`, 'i');
    if (prefixRegex.test(text)) {
      return 'prefixMatch';
    }

    // Check for contains match
    if (text.includes(word)) {
      return 'containsMatch';
    }

    return null;
  }

  /**
   * Escape special regex characters.
   */
  private escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  // ===========================================================================
  // Sorting
  // ===========================================================================

  /**
   * Sort results based on query sort options.
   */
  private sortResults(results: SearchResult[], query: SearchQuery): SearchResult[] {
    const sortBy = query.sortBy ?? 'relevance';
    const direction = query.sortDirection ?? 'desc';
    const multiplier = direction === 'asc' ? 1 : -1;

    return [...results].sort((a, b) => {
      let comparison = 0;

      switch (sortBy) {
        case 'relevance':
          comparison = a.relevanceScore - b.relevanceScore;
          break;
        case 'title':
          comparison = a.title.localeCompare(b.title);
          break;
        case 'trustScore':
          comparison = a.trustScore - b.trustScore;
          break;
        case 'reach':
          comparison = this.reachToNumber(a.reach) - this.reachToNumber(b.reach);
          break;
        case 'newest':
          comparison = (a.createdAt ?? '').localeCompare(b.createdAt ?? '');
          break;
        case 'updated':
          comparison = (a.updatedAt ?? '').localeCompare(b.updatedAt ?? '');
          break;
      }

      return comparison * multiplier;
    });
  }

  /**
   * Convert reach level to numeric for sorting.
   */
  private reachToNumber(reach: ContentReach): number {
    const levels: ContentReach[] = ['private', 'invited', 'local', 'community', 'federated', 'commons'];
    return levels.indexOf(reach);
  }

  // ===========================================================================
  // Facets
  // ===========================================================================

  /**
   * Compute facet counts from results.
   */
  private computeFacets(results: SearchResult[], query: SearchQuery): SearchFacets {
    const byContentType = new Map<ContentType, number>();
    const byReach = new Map<ContentReach, number>();
    const byTrustLevel = new Map<TrustLevel, number>();
    const byTag = new Map<string, number>();
    let flagged = 0;
    let unflagged = 0;

    for (const result of results) {
      // Content type
      byContentType.set(
        result.contentType,
        (byContentType.get(result.contentType) ?? 0) + 1
      );

      // Reach
      byReach.set(result.reach, (byReach.get(result.reach) ?? 0) + 1);

      // Trust level
      byTrustLevel.set(
        result.trustLevel,
        (byTrustLevel.get(result.trustLevel) ?? 0) + 1
      );

      // Tags
      for (const tag of result.tags) {
        byTag.set(tag, (byTag.get(tag) ?? 0) + 1);
      }

      // Flag status
      if (result.hasFlags) {
        flagged++;
      } else {
        unflagged++;
      }
    }

    // Convert maps to sorted arrays
    const toFacetArray = <T>(
      map: Map<T, number>,
      selectedValues?: T[]
    ): FacetCount<T>[] => {
      return Array.from(map.entries())
        .map(([value, count]) => ({
          value,
          count,
          selected: selectedValues?.includes(value) ?? false
        }))
        .sort((a, b) => b.count - a.count);
    };

    return {
      byContentType: toFacetArray(byContentType, query.contentTypes),
      byReach: toFacetArray(byReach, query.reachLevels),
      byTrustLevel: toFacetArray(byTrustLevel, query.trustLevels),
      byTag: toFacetArray(byTag, query.tags).slice(0, DEFAULT_SEARCH_CONFIG.maxFacetTags),
      byFlagStatus: { flagged, unflagged }
    };
  }

  // ===========================================================================
  // Helpers
  // ===========================================================================

  /**
   * Compute trust level for a node.
   */
  private computeTrustLevel(node: ContentIndexEntry): TrustLevel {
    const attestationTypes = (node as any).attestationTypes ?? [];
    const hasFlags = ((node as any).flags ?? []).length > 0;
    return calculateTrustLevel(
      (node as any).reach ?? 'commons',
      attestationTypes,
      hasFlags
    );
  }

  /**
   * Highlight matching text in a string.
   */
  private highlightMatch(text: string, query: string): string {
    const index = text.toLowerCase().indexOf(query.toLowerCase());
    if (index === -1) return text;

    return (
      text.slice(0, index) +
      '<mark>' +
      text.slice(index, index + query.length) +
      '</mark>' +
      text.slice(index + query.length)
    );
  }
}
