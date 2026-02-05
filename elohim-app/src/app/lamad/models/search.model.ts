/**
 * Search Model - Enhanced search with relevance scoring, highlights, and facets.
 *
 * This model provides:
 * - Relevance-scored search results
 * - Highlighted match snippets
 * - Faceted filtering (by type, reach, trust level, tags)
 * - Pagination support
 */

import { TrustLevel } from '@app/elohim/models/trust-badge.model';

// @coverage: 66.7% (2026-02-05)

import { ContentType, ContentReach } from './content-node.model';

// ============================================================================
// Search Query
// ============================================================================

/**
 * SearchQuery - Parameters for content search.
 */
export interface SearchQuery {
  /** Search text (searches title, description, tags) */
  text: string;

  /** Filter by content types */
  contentTypes?: ContentType[];

  /** Filter by reach levels */
  reachLevels?: ContentReach[];

  /** Filter by trust levels */
  trustLevels?: TrustLevel[];

  /** Filter by tags (OR logic - any of these tags) */
  tags?: string[];

  /** Filter by tags (AND logic - must have all these tags) */
  requiredTags?: string[];

  /** Minimum trust score (0-1) */
  minTrustScore?: number;

  /** Include only content with no warnings */
  excludeFlagged?: boolean;

  /** Sort order */
  sortBy?: SearchSortOption;

  /** Sort direction */
  sortDirection?: 'asc' | 'desc';

  /** Pagination: page number (1-based) */
  page?: number;

  /** Pagination: results per page */
  pageSize?: number;
}

/**
 * SearchSortOption - How to sort results.
 */
export type SearchSortOption =
  | 'relevance' // Default: by match score
  | 'title' // Alphabetical by title
  | 'trustScore' // By trust score
  | 'reach' // By reach level
  | 'newest' // By creation date
  | 'updated'; // By last update

// ============================================================================
// Search Results
// ============================================================================

/**
 * SearchResult - A single search result with scoring and highlights.
 */
export interface SearchResult {
  /** Content ID */
  id: string;

  /** Content title */
  title: string;

  /** Content description */
  description: string;

  /** Content type */
  contentType: ContentType;

  /** Tags */
  tags: string[];

  /** Reach level */
  reach: ContentReach;

  /** Trust score (0-1) */
  trustScore: number;

  /** Computed trust level */
  trustLevel: TrustLevel;

  /** Has active warnings/flags */
  hasFlags: boolean;

  /** Relevance score (0-100, higher = better match) */
  relevanceScore: number;

  /** Which fields matched the query */
  matchedFields: MatchedField[];

  /** Highlighted snippets showing matches */
  highlights: SearchHighlight[];

  /** Creation timestamp */
  createdAt?: string;

  /** Last update timestamp */
  updatedAt?: string;
}

/**
 * MatchedField - Which field matched and how strongly.
 */
export interface MatchedField {
  /** Field name that matched */
  field: 'title' | 'description' | 'tags';

  /** Match strength (title > tags > description) */
  weight: number;

  /** The matched text */
  matchedText: string;
}

/**
 * SearchHighlight - A highlighted snippet showing the match context.
 */
export interface SearchHighlight {
  /** Field the highlight is from */
  field: 'title' | 'description' | 'tags';

  /** Text snippet with match markers */
  snippet: string;

  /** Start/end positions of matches within snippet */
  matchRanges: { start: number; end: number }[];
}

/**
 * SearchResults - Paginated search response with facets.
 */
export interface SearchResults {
  /** The search query that produced these results */
  query: SearchQuery;

  /** Matching results for current page */
  results: SearchResult[];

  /** Total number of matching results (across all pages) */
  totalCount: number;

  /** Current page (1-based) */
  page: number;

  /** Results per page */
  pageSize: number;

  /** Total pages */
  totalPages: number;

  /** Has more results after this page */
  hasMore: boolean;

  /** Facet counts for filtering UI */
  facets: SearchFacets;

  /** Search execution time (ms) */
  executionTimeMs: number;
}

// ============================================================================
// Facets
// ============================================================================

/**
 * SearchFacets - Aggregated counts for filter UI.
 */
export interface SearchFacets {
  /** Count by content type */
  byContentType: FacetCount<ContentType>[];

  /** Count by reach level */
  byReach: FacetCount<ContentReach>[];

  /** Count by trust level */
  byTrustLevel: FacetCount<TrustLevel>[];

  /** Count by tag (top N tags) */
  byTag: FacetCount<string>[];

  /** Count of flagged vs unflagged */
  byFlagStatus: {
    flagged: number;
    unflagged: number;
  };
}

/**
 * FacetCount - A single facet value with count.
 */
export interface FacetCount<T> {
  /** The facet value */
  value: T;

  /** Number of results with this value */
  count: number;

  /** Whether this facet is currently selected in the query */
  selected: boolean;
}

// ============================================================================
// Search Scoring Configuration
// ============================================================================

/**
 * Field weights for relevance scoring.
 * Title matches are most important, then tags, then description.
 */
export const SEARCH_FIELD_WEIGHTS = {
  title: 10,
  tags: 5,
  description: 2,
} as const;

/**
 * Bonus multipliers for exact vs partial matches.
 */
export const SEARCH_MATCH_BONUSES = {
  exactMatch: 2, // Full word match
  prefixMatch: 1.5, // Word starts with query
  containsMatch: 1, // Query found anywhere in word
} as const;

/**
 * Default search configuration.
 */
export const DEFAULT_SEARCH_CONFIG = {
  pageSize: 20,
  maxPageSize: 100,
  maxHighlightSnippetLength: 150,
  maxHighlightsPerField: 3,
  maxFacetTags: 20,
} as const;

// ============================================================================
// Search Suggestions
// ============================================================================

/**
 * SearchSuggestion - Autocomplete suggestion.
 */
export interface SearchSuggestion {
  /** Suggested text */
  text: string;

  /** Type of suggestion */
  type: 'query' | 'tag' | 'title' | 'contentType' | 'path';

  /** Number of results this would return */
  resultCount?: number;

  /** Highlighted portion matching user input */
  highlight?: string;
}

/**
 * SearchSuggestions - Response for autocomplete.
 */
export interface SearchSuggestions {
  /** User's partial query */
  query: string;

  /** Suggestions sorted by relevance */
  suggestions: SearchSuggestion[];
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Create a default/empty search query.
 */
export function createEmptyQuery(): SearchQuery {
  return {
    text: '',
    sortBy: 'relevance',
    sortDirection: 'desc',
    page: 1,
    pageSize: DEFAULT_SEARCH_CONFIG.pageSize,
  };
}

/**
 * Create empty search results.
 */
export function createEmptyResults(query: SearchQuery): SearchResults {
  return {
    query,
    results: [],
    totalCount: 0,
    page: query.page ?? 1,
    pageSize: query.pageSize ?? DEFAULT_SEARCH_CONFIG.pageSize,
    totalPages: 0,
    hasMore: false,
    facets: {
      byContentType: [],
      byReach: [],
      byTrustLevel: [],
      byTag: [],
      byFlagStatus: { flagged: 0, unflagged: 0 },
    },
    executionTimeMs: 0,
  };
}

/**
 * Highlight matched text with markers.
 * Returns text with <mark> tags around matches.
 */
export function highlightMatches(text: string, query: string): string {
  if (!query || !text) return text;

  const words = query
    .toLowerCase()
    .split(/\s+/)
    .filter(w => w.length > 0);
  let result = text;

  for (const word of words) {
    const regex = new RegExp(`(${escapeRegex(word)})`, 'gi');
    result = result.replace(regex, '<mark>$1</mark>');
  }

  return result;
}

/**
 * Escape special regex characters.
 */
function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Extract a snippet around matched text.
 */
export function extractSnippet(
  text: string,
  query: string,
  maxLength: number = DEFAULT_SEARCH_CONFIG.maxHighlightSnippetLength
): { snippet: string; matchRanges: { start: number; end: number }[] } {
  if (!query || !text) {
    return {
      snippet: text.slice(0, maxLength) + (text.length > maxLength ? '...' : ''),
      matchRanges: [],
    };
  }

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const firstMatchIndex = lowerText.indexOf(lowerQuery);

  if (firstMatchIndex === -1) {
    return {
      snippet: text.slice(0, maxLength) + (text.length > maxLength ? '...' : ''),
      matchRanges: [],
    };
  }

  // Center snippet around first match
  const contextBefore = Math.floor((maxLength - query.length) / 2);
  const start = Math.max(0, firstMatchIndex - contextBefore);
  const end = Math.min(text.length, start + maxLength);

  let snippet = text.slice(start, end);
  if (start > 0) snippet = '...' + snippet;
  if (end < text.length) snippet = snippet + '...';

  // Find all match ranges in snippet
  const matchRanges: { start: number; end: number }[] = [];
  const snippetLower = snippet.toLowerCase();
  let searchStart = 0;

  while (true) {
    const matchIndex = snippetLower.indexOf(lowerQuery, searchStart);
    if (matchIndex === -1) break;
    matchRanges.push({ start: matchIndex, end: matchIndex + query.length });
    searchStart = matchIndex + 1;
  }

  return { snippet, matchRanges };
}
