import { Injectable, inject } from '@angular/core';

// @coverage: 94.2% (2026-02-24)

import { map, catchError } from 'rxjs/operators';

import { Observable, of, forkJoin } from 'rxjs';

import { TrustLevel } from '../models/trust-badge.model';
import { DataLoaderService } from './data-loader.service';
import { TrustBadgeService } from './trust-badge.service';
import { ContentBackendService } from './content-backend.service';

import { ContentType, ContentReach } from '../models/content-node.model';
import { PathIndexEntry } from '../models/learning-path.model';
import {
  SearchQuery,
  SearchResult,
  SearchResults,
  SearchFacets,
  SearchProvenance,
  FacetCount,
  SearchHighlight,
  MatchedField,
  SearchSuggestion,
  SearchSuggestions,
  DEFAULT_SEARCH_CONFIG,
  createEmptyResults,
} from '../models/search.model';

import { ContentIndexEntry } from './content.service';

import type { ContentSearchQuery } from '@elohim/service';
import type {
  ContentSearchView,
  ContentSearchCandidateView,
  FacetCountView,
} from '../../generated/content-search-view';

/**
 * The peer's trust legibility label, read into the client's trust ladder and score.
 * The server never invents a number (R-S5); this is the client's reading of the label,
 * and it is the only place the reading happens.
 */
const TRUST_LABEL_LEVEL: ReadonlyMap<string, TrustLevel> = new Map<string, TrustLevel>([
  ['notarized', 'verified'],
  ['published', 'trusted'],
  ['unconfirmed', 'unverified'],
]);

const TRUST_LABEL_SCORE: ReadonlyMap<string, number> = new Map([
  ['notarized', 1],
  ['published', 0.5],
  ['unconfirmed', 0],
]);

/**
 * SearchService - the lamad client of the peer's content search.
 *
 * `search()` asks `GET /db/content/search` and reads the answer. The peer folded the content,
 * declared the recipe, ranked, gated by reach and counted the facets; this service publishes what
 * it said and the provenance it said it under. It never re-scores and never re-sorts (plan Lane S,
 * ruling R-S6; backend authoritative, X5).
 *
 * `suggest()` and `getTagCloud()` stay local: autocomplete over the content index the client
 * already holds is a reading of what is in hand, not a claim about what the peer knows.
 *
 * Usage:
 * ```typescript
 * // Ask the peer
 * this.searchService.search({ text: 'governance' }).subscribe(results => {
 *   console.log(results.results);    // the peer's candidates, in the peer's order
 *   console.log(results.facets);     // the peer's counts over the admitted set
 *   console.log(results.provenance); // recipe CID, rankingKnown, fold state
 * });
 *
 * // Filtered — one content type and one reach ring reach the route; the rest is named
 * // in provenance.unresolved rather than dropped
 * this.searchService.search({
 *   text: 'protocol',
 *   contentTypes: ['epic'],
 *   page: 1,
 *   pageSize: 10
 * }).subscribe(results => ...);
 *
 * // Autocomplete (local)
 * this.searchService.suggest('gov').subscribe(suggestions => ...);
 * ```
 */
@Injectable({ providedIn: 'root' })
export class SearchService {
  private readonly dataLoader = inject(DataLoaderService);
  private readonly trustBadgeService = inject(TrustBadgeService);
  private readonly backend = inject(ContentBackendService);

  /**
   * Ask the peer its content search and read the answer it gives (plan Lane S, ruling R-S6).
   *
   * The ranking, the facets and the count are the peer's: it folded the content, it declared the
   * recipe, it ran the reach gate. This service is that route's client. It does not re-score and
   * it does not re-sort — a client that reordered the candidates would be publishing a ranking no
   * peer ever made, under a recipe CID it then prints as provenance.
   *
   * What it does do is read: the peer's `trust` label into the client's trust ladder, the matched
   * section into the card's highlight, the recipe/rankingKnown/fold into `provenance` so the page
   * can print where the answer came from. A transport failure is answered honestly — empty
   * results with `foldState: 'unreachable'`, never an empty list dressed as "no matches".
   */
  search(query: SearchQuery): Observable<SearchResults> {
    const startTime = Date.now();
    const page = query.page ?? 1;
    const pageSize = Math.min(
      query.pageSize ?? DEFAULT_SEARCH_CONFIG.pageSize,
      DEFAULT_SEARCH_CONFIG.maxPageSize
    );

    return this.backend.searchContentView(this.askPeer(query, page, pageSize)).pipe(
      map(view => this.readAnswer(view, query, page, pageSize, startTime)),
      catchError(() =>
        of({
          ...createEmptyResults({ ...query, page, pageSize }),
          executionTimeMs: Date.now() - startTime,
        })
      )
    );
  }

  /**
   * The question, in the route's own vocabulary.
   *
   * The route takes one content type and one reach ring; the client's query type allows several.
   * Whatever cannot be carried is named in `provenance.unresolved` rather than silently dropped.
   */
  private askPeer(query: SearchQuery, page: number, pageSize: number): ContentSearchQuery {
    const ask: ContentSearchQuery = {
      q: query.text ?? '',
      limit: pageSize,
      offset: (page - 1) * pageSize,
    };

    if (query.contentTypes?.length) ask.contentType = query.contentTypes[0];
    if (query.reachLevels?.length) ask.reach = query.reachLevels[0];
    if (query.tags?.length) ask.tags = [...query.tags];

    return ask;
  }

  /** One line per query parameter this route cannot carry — the client's own unresolved. */
  private unhonoured(query: SearchQuery): string[] {
    const lines: string[] = [];

    const types = query.contentTypes ?? [];
    const rings = query.reachLevels ?? [];

    if (types.length > 1) {
      lines.push(
        `contentTypes: the route filters one type; asked for ${types.length}, sent ${types[0]}`
      );
    }
    if (rings.length > 1) {
      lines.push(
        `reachLevels: the route filters one reach ring; asked for ${rings.length}, sent ${rings[0]}`
      );
    }
    if (query.requiredTags?.length) {
      lines.push('requiredTags: the route matches any tag, never all of them');
    }
    if (query.trustLevels?.length) {
      lines.push('trustLevels: the peer does not filter by trust');
    }
    if (query.minTrustScore !== undefined) {
      lines.push('minTrustScore: the peer serves a trust label, never a number to threshold');
    }
    if (query.excludeFlagged) {
      lines.push('excludeFlagged: the answer carries no flag state');
    }
    if (query.sortBy && query.sortBy !== 'relevance') {
      lines.push(`sortBy ${query.sortBy}: the peer's recipe owns the order`);
    }

    return lines;
  }

  /** Read the peer's whole answer into the page's shape, in the peer's order. */
  private readAnswer(
    view: ContentSearchView,
    query: SearchQuery,
    page: number,
    pageSize: number,
    startTime: number
  ): SearchResults {
    const results = view.candidates.map(candidate => this.readCandidate(candidate));
    const totalPages = Math.ceil(view.totalCount / pageSize);

    const provenance: SearchProvenance = {
      recipeCid: view.recipe.cid,
      rankingKnown: view.rankingKnown,
      foldState: view.fold.state,
      unresolved: [...view.unresolved, ...this.unhonoured(query)],
    };

    return {
      query,
      results,
      totalCount: view.totalCount,
      page,
      pageSize,
      totalPages,
      hasMore: page < totalPages,
      facets: this.readFacets(view, results, query),
      provenance,
      executionTimeMs: Date.now() - startTime,
    };
  }

  /**
   * One candidate, as the peer ranked it.
   *
   * `score` is the fused reciprocal rank: carried through for display, never compared across
   * answers and never used to reorder. The matched section is the peer's own account of where the
   * match landed, so it becomes both the card's description and its single highlight.
   */
  private readCandidate(candidate: ContentSearchCandidateView): SearchResult {
    const section = candidate.bestSection;
    const field = this.sectionField(section?.title);

    const matchedFields: MatchedField[] = section
      ? [{ field, weight: candidate.score, matchedText: section.snippet }]
      : [];
    const highlights: SearchHighlight[] = section
      ? [{ field, snippet: section.snippet, matchRanges: [] }]
      : [];

    return {
      id: candidate.contentId,
      title: candidate.title,
      description: section?.snippet ?? '',
      contentType: candidate.contentType as ContentType,
      tags: candidate.tags,
      // The wire's declared-visibility Reach and the client's LocalityLevel are two vocabularies
      // that share their ends ('private', 'commons'); the card reads the ring as served.
      reach: candidate.reach as unknown as ContentReach,
      trustScore: TRUST_LABEL_SCORE.get(candidate.trust) ?? 0,
      trustLevel: TRUST_LABEL_LEVEL.get(candidate.trust) ?? 'unverified',
      // The answer carries no flag state; claiming 'unflagged' as a finding would be inventing one.
      hasFlags: false,
      relevanceScore: candidate.score,
      matchedFields,
      highlights,
    };
  }

  /** The peer names its sections; the card's three fields are the client's reading of them. */
  private sectionField(title: string | undefined): MatchedField['field'] {
    if (title === 'tags') return 'tags';
    if (title === 'head') return 'title';
    return 'description';
  }

  /**
   * Facets: the peer's counts over the admitted set, as served.
   *
   * Trust and flag facets have no server counterpart — the answer carries a trust label per
   * candidate and no flags at all — so those two are counted over this page only, which is the
   * most the client can honestly say.
   */
  private readFacets(
    view: ContentSearchView,
    results: SearchResult[],
    query: SearchQuery
  ): SearchFacets {
    const byTrustLevel = new Map<TrustLevel, number>();
    for (const result of results) {
      byTrustLevel.set(result.trustLevel, (byTrustLevel.get(result.trustLevel) ?? 0) + 1);
    }

    const read = <T extends string>(counts: FacetCountView[], selected?: T[]): FacetCount<T>[] =>
      counts.map(count => ({
        value: count.value as T,
        count: count.count,
        selected: selected?.includes(count.value as T) ?? false,
      }));

    return {
      byContentType: read<ContentType>(view.facets.contentType, query.contentTypes),
      byReach: read<ContentReach>(view.facets.reach, query.reachLevels),
      byTrustLevel: Array.from(byTrustLevel.entries())
        .map(([value, count]) => ({
          value,
          count,
          selected: query.trustLevels?.includes(value) ?? false,
        }))
        .sort((a, b) => b.count - a.count),
      byTag: read<string>(view.facets.tags, query.tags).slice(0, DEFAULT_SEARCH_CONFIG.maxFacetTags),
      byFlagStatus: { flagged: 0, unflagged: results.length },
    };
  }

  /**
   * Get autocomplete suggestions for partial query.
   * Includes suggestions from both content nodes and learning paths.
   */
  suggest(partialQuery: string, limit = 10): Observable<SearchSuggestions> {
    if (!partialQuery || partialQuery.trim().length < 2) {
      return of({ query: partialQuery, suggestions: [] });
    }

    return forkJoin({
      contentIndex: this.dataLoader.getContentIndex().pipe(catchError(() => of({ nodes: [] }))),
      pathIndex: this.dataLoader.getPathIndex().pipe(catchError(() => of({ paths: [] }))),
    }).pipe(
      map(({ contentIndex, pathIndex }) => {
        const ci = contentIndex as { nodes?: ContentIndexEntry[] };
        const pi = pathIndex as { paths?: PathIndexEntry[] };
        const nodes = ci.nodes ?? [];
        const paths = pi.paths ?? [];
        const query = partialQuery.toLowerCase().trim();
        const suggestions: SearchSuggestion[] = [];
        const seen = new Set<string>();

        // Add content title suggestions
        this.addContentTitleSuggestions(nodes, query, suggestions, seen, limit);

        // Add path title suggestions
        this.addPathTitleSuggestions(paths, query, suggestions, seen, limit);

        // Add tag suggestions
        this.addTagSuggestions(nodes, paths, query, suggestions, seen, limit);

        return { query: partialQuery, suggestions };
      }),
      catchError(() => of({ query: partialQuery, suggestions: [] }))
    );
  }

  /**
   * Add content title suggestions to the suggestions array.
   */
  private addContentTitleSuggestions(
    nodes: ContentIndexEntry[],
    query: string,
    suggestions: SearchSuggestion[],
    seen: Set<string>,
    limit: number
  ): void {
    for (const node of nodes) {
      if (suggestions.length >= limit) break;

      const titleLower = node.title.toLowerCase();
      if (titleLower.includes(query) && !seen.has(titleLower)) {
        seen.add(titleLower);
        suggestions.push({
          text: node.title,
          type: 'title',
          highlight: this.highlightMatch(node.title, query),
        });
      }
    }
  }

  /**
   * Add path title suggestions to the suggestions array.
   */
  private addPathTitleSuggestions(
    paths: PathIndexEntry[],
    query: string,
    suggestions: SearchSuggestion[],
    seen: Set<string>,
    limit: number
  ): void {
    for (const path of paths) {
      if (suggestions.length >= limit) break;

      const titleLower = path.title.toLowerCase();
      if (titleLower.includes(query) && !seen.has(titleLower)) {
        seen.add(titleLower);
        suggestions.push({
          text: path.title,
          type: 'path',
          highlight: this.highlightMatch(path.title, query),
        });
      }
    }
  }

  /**
   * Add tag suggestions to the suggestions array.
   */
  private addTagSuggestions(
    nodes: ContentIndexEntry[],
    paths: PathIndexEntry[],
    query: string,
    suggestions: SearchSuggestion[],
    seen: Set<string>,
    limit: number
  ): void {
    const tagCounts = this.collectMatchingTags(nodes, paths, query);
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
          highlight: this.highlightMatch(tag, query),
        });
      }
    }
  }

  /**
   * Collect matching tags from nodes and paths with their occurrence counts.
   */
  private collectMatchingTags(
    nodes: ContentIndexEntry[],
    paths: PathIndexEntry[],
    query: string
  ): Map<string, number> {
    const tagCounts = new Map<string, number>();

    for (const node of nodes) {
      this.countMatchingTags(node.tags ?? [], query, tagCounts);
    }

    for (const path of paths) {
      this.countMatchingTags(path.tags ?? [], query, tagCounts);
    }

    return tagCounts;
  }

  /**
   * Count matching tags and add to the tagCounts map.
   */
  private countMatchingTags(tags: string[], query: string, tagCounts: Map<string, number>): void {
    for (const tag of tags) {
      if (tag.toLowerCase().includes(query)) {
        tagCounts.set(tag, (tagCounts.get(tag) ?? 0) + 1);
      }
    }
  }

  /**
   * Get all unique tags with counts.
   */
  getTagCloud(): Observable<{ tag: string; count: number }[]> {
    return this.dataLoader.getContentIndex().pipe(
      map(rawIndex => {
        const idx = rawIndex as { nodes?: ContentIndexEntry[] };
        const tagCounts = new Map<string, number>();

        for (const node of idx.nodes ?? []) {
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
  // Helpers
  // ===========================================================================

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
