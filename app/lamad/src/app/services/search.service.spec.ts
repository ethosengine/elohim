import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { SearchService } from './search.service';
import { DataLoaderService } from './data-loader.service';
import { TrustBadgeService } from './trust-badge.service';
import { ContentBackendService } from './content-backend.service';
import { vi, Mock } from 'vitest';

import type { ContentSearchView } from '../../generated/content-search-view';

describe('SearchService', () => {
  let service: SearchService;
  let dataLoaderSpy: any;
  let trustBadgeSpy: any;
  let backendSpy: any;

  const mockContentIndex = {
    nodes: [
      {
        id: 'content-1',
        title: 'Governance Framework',
        description: 'A comprehensive guide to decentralized governance principles and practices',
        contentType: 'epic',
        tags: ['governance', 'decentralization', 'principles'],
        reach: 'commons',
        trustScore: 0.95,
        flags: [],
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-02T00:00:00.000Z',
      },
      {
        id: 'content-2',
        title: 'Constitutional Design',
        description: 'How to design constitutional systems for digital communities',
        contentType: 'feature',
        tags: ['constitution', 'design', 'governance'],
        reach: 'regional',
        trustScore: 0.85,
        flags: [],
        createdAt: '2025-01-02T00:00:00.000Z',
        updatedAt: '2025-01-03T00:00:00.000Z',
      },
      {
        id: 'content-3',
        title: 'Trust Networks',
        description: 'Building trust networks in distributed systems',
        contentType: 'concept',
        tags: ['trust', 'networks', 'distributed'],
        reach: 'local',
        trustScore: 0.75,
        flags: [],
        createdAt: '2025-01-03T00:00:00.000Z',
        updatedAt: '2025-01-04T00:00:00.000Z',
      },
      {
        id: 'content-4',
        title: 'Flagged Content',
        description: 'This content has been flagged for review',
        contentType: 'concept',
        tags: ['disputed'],
        reach: 'commons',
        trustScore: 0.3,
        flags: ['accuracy-concern'],
        createdAt: '2025-01-04T00:00:00.000Z',
        updatedAt: '2025-01-05T00:00:00.000Z',
      },
      {
        id: 'content-5',
        title: 'Protocol Implementation',
        description: 'Step by step protocol implementation guide for governance modules',
        contentType: 'task',
        tags: ['protocol', 'implementation', 'governance'],
        reach: 'neighborhood',
        trustScore: 0.9,
        flags: [],
        createdAt: '2025-01-05T00:00:00.000Z',
        updatedAt: '2025-01-06T00:00:00.000Z',
      },
    ],
    lastUpdated: '2025-01-06T00:00:00.000Z',
  };

  const mockPathIndex = {
    paths: [
      {
        id: 'path-1',
        title: 'Governance Learning Path',
        description: 'Learn about governance',
        tags: ['governance'],
        difficulty: 'intermediate' as const,
        estimatedDuration: '2h',
        stepCount: 5,
        createdAt: '2025-01-01T00:00:00.000Z',
      },
    ],
    totalCount: 1,
    lastUpdated: '2025-01-06T00:00:00.000Z',
  };

  /**
   * A whole answer from the peer, in the peer's order. The service is its reader:
   * whatever this fixture says is what the results must say.
   */
  function peerAnswer(overrides: Partial<ContentSearchView> = {}): ContentSearchView {
    return {
      query: 'governance',
      rankingKnown: true,
      recipe: {
        name: 'rrf-v2',
        cid: 'bafyreianswerrecipecid0000000000000000000000000000000000',
        k: 60,
        orderOnly: true,
        producers: [{ id: 'lexical', method: 'bafyreimeasurecid000000000000' }],
      },
      lens: {
        level: 'standard',
        choiceCount: 20,
        cid: 'bafyreilenstablecid000000000',
        provenance: 'defaulted',
      },
      selection: 'showing 3 of 3 ranked under rrf-v2',
      fold: {
        state: 'present',
        value: {
          measure: 'bafyreimeasurecid000000000000',
          state: 'complete',
          attestationCid: 'bafyreiattestationcid00000000',
          at: 1_790_000_000,
        },
      },
      foldLag: {
        state: 'present',
        value: { behind: 0, limit: 200, unit: 'units', within: true },
      },
      candidates: [
        {
          contentId: 'content-5',
          title: 'Protocol Implementation',
          contentType: 'lesson',
          reach: 'commons',
          trust: 'notarized',
          tags: ['protocol', 'governance'],
          score: 0.032,
          producer: 'lexical',
          method: 'bafyreimeasurecid000000000000',
          bestSection: { title: 'head', snippet: 'governance modules, step by step' },
        },
        {
          contentId: 'content-1',
          title: 'Governance Framework',
          contentType: 'epic',
          reach: 'commons',
          trust: 'published',
          tags: ['governance', 'principles'],
          score: 0.016,
          producer: 'lexical',
          method: 'bafyreimeasurecid000000000000',
          bestSection: { title: 'tags', snippet: 'governance' },
        },
        {
          contentId: 'content-2',
          title: 'Constitutional Design',
          contentType: 'concept',
          reach: 'community',
          trust: 'unconfirmed',
          tags: ['constitution'],
          score: 0.008,
          producer: 'lexical',
          method: 'bafyreimeasurecid000000000000',
          bestSection: null,
        },
      ],
      facets: {
        contentType: [
          { value: 'lesson', count: 1 },
          { value: 'epic', count: 1 },
          { value: 'concept', count: 1 },
        ],
        reach: [
          { value: 'commons', count: 2 },
          { value: 'community', count: 1 },
        ],
        tags: [
          { value: 'governance', count: 2 },
          { value: 'protocol', count: 1 },
        ],
      },
      omissions: ['1 row withheld: reach private, reader is not a holder'],
      unresolved: [],
      totalCount: 3,
      ...overrides,
    } as ContentSearchView;
  }

  beforeEach(() => {
    const dataLoaderSpyObj = {
      getContentIndex: vi.fn(),
      getPathIndex: vi.fn(),
    };
    const trustBadgeSpyObj = {
      getTrustBadges: vi.fn(),
    };
    const backendSpyObj = {
      searchContentView: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        SearchService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: TrustBadgeService, useValue: trustBadgeSpyObj },
        { provide: ContentBackendService, useValue: backendSpyObj },
      ],
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as { [K in keyof DataLoaderService]?: Mock };
    trustBadgeSpy = TestBed.inject(TrustBadgeService) as { [K in keyof TrustBadgeService]?: Mock };
    backendSpy = TestBed.inject(ContentBackendService) as {
      [K in keyof ContentBackendService]?: Mock;
    };

    dataLoaderSpy.getContentIndex.mockReturnValue(of(mockContentIndex));
    dataLoaderSpy.getPathIndex.mockReturnValue(of(mockPathIndex));
    backendSpy.searchContentView.mockReturnValue(of(peerAnswer()));

    service = TestBed.inject(SearchService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // The route's client (plan Lane S, ruling R-S6)
  // =========================================================================

  describe('search', () => {
    it('search_calls_backend_searchContentView_with_text_and_page', () =>
      new Promise<void>(done => {
        service
          .search({
            text: 'governance',
            page: 3,
            pageSize: 5,
            contentTypes: ['epic'],
            reachLevels: ['commons'],
            tags: ['protocol'],
          })
          .subscribe(() => {
            expect(backendSpy.searchContentView).toHaveBeenCalledWith({
              q: 'governance',
              limit: 5,
              offset: 10,
              contentType: 'epic',
              reach: 'commons',
              tags: ['protocol'],
            });
            done();
          });
      }));

    it('search_maps_candidates_to_results_preserving_server_order', () =>
      new Promise<void>(done => {
        service.search({ text: 'governance' }).subscribe(results => {
          expect(results.results.map(r => r.id)).toEqual(['content-5', 'content-1', 'content-2']);
          // the fused score is carried through as the relevance, never recomputed
          expect(results.results[0].relevanceScore).toBe(0.032);
          expect(results.results[0].title).toBe('Protocol Implementation');
          expect(results.results[0].tags).toEqual(['protocol', 'governance']);
          // the matched section is where the match landed, named by the peer
          expect(results.results[0].matchedFields[0].field).toBe('title');
          expect(results.results[1].matchedFields[0].field).toBe('tags');
          expect(results.results[2].matchedFields).toEqual([]);
          // trust is the peer's label, read into the client's ladder
          expect(results.results[0].trustLevel).toBe('verified');
          expect(results.results[0].trustScore).toBe(1);
          expect(results.results[2].trustLevel).toBe('unverified');
          done();
        });
      }));

    it('search_does_not_reorder_a_lower_scoring_candidate_the_peer_ranked_first', () =>
      new Promise<void>(done => {
        const answer = peerAnswer();
        // The peer's order is authoritative even when the scores look "wrong" here:
        // a fused reciprocal rank is comparable only inside one answer.
        answer.candidates[0].score = 0.001;
        backendSpy.searchContentView.mockReturnValue(of(answer));

        service.search({ text: 'governance' }).subscribe(results => {
          expect(results.results.map(r => r.id)).toEqual(['content-5', 'content-1', 'content-2']);
          done();
        });
      }));

    it('search_uses_server_facets_and_totalCount', () =>
      new Promise<void>(done => {
        service.search({ text: 'governance', contentTypes: ['epic'] }).subscribe(results => {
          expect(results.totalCount).toBe(3);
          expect(results.facets.byContentType).toEqual([
            { value: 'lesson', count: 1, selected: false },
            { value: 'epic', count: 1, selected: true },
            { value: 'concept', count: 1, selected: false },
          ]);
          expect(results.facets.byReach.map(f => f.value)).toEqual(['commons', 'community']);
          expect(results.facets.byTag).toEqual([
            { value: 'governance', count: 2, selected: false },
            { value: 'protocol', count: 1, selected: false },
          ]);
          // trust and flag facets have no server counterpart: counted over this page only
          expect(results.facets.byTrustLevel.map(f => f.value).sort()).toEqual([
            'trusted',
            'unverified',
            'verified',
          ]);
          expect(results.facets.byFlagStatus).toEqual({ flagged: 0, unflagged: 3 });
          done();
        });
      }));

    it('search_surfaces_provenance_recipe_and_rankingKnown', () =>
      new Promise<void>(done => {
        service.search({ text: 'governance' }).subscribe(results => {
          expect(results.provenance.recipeCid).toBe(
            'bafyreianswerrecipecid0000000000000000000000000000000000'
          );
          expect(results.provenance.rankingKnown).toBe(true);
          expect(results.provenance.foldState).toBe('present');
          expect(results.provenance.unresolved).toEqual([]);
          done();
        });
      }));

    it('search_carries_the_peers_unresolved_lines_into_provenance', () =>
      new Promise<void>(done => {
        backendSpy.searchContentView.mockReturnValue(
          of(peerAnswer({ unresolved: ['recipe pin names rrf-v1; this peer ranked under rrf-v2'] }))
        );

        service.search({ text: 'governance' }).subscribe(results => {
          expect(results.provenance.unresolved).toContain(
            'recipe pin names rrf-v1; this peer ranked under rrf-v2'
          );
          done();
        });
      }));

    it('search_names_a_filter_the_route_cannot_carry_rather_than_dropping_it', () =>
      new Promise<void>(done => {
        service
          .search({ text: 'governance', minTrustScore: 0.8, sortBy: 'title' })
          .subscribe(results => {
            expect(results.provenance.unresolved.some(u => u.includes('minTrustScore'))).toBe(true);
            expect(results.provenance.unresolved.some(u => u.includes('sortBy'))).toBe(true);
            done();
          });
      }));

    it('search_reports_an_unfolded_peer_as_absent_with_no_candidates', () =>
      new Promise<void>(done => {
        backendSpy.searchContentView.mockReturnValue(
          of(
            peerAnswer({
              rankingKnown: false,
              candidates: [],
              totalCount: 0,
              fold: { state: 'absent', reason: 'observed_absent' },
              foldLag: { state: 'absent', reason: 'observed_absent' },
              facets: { contentType: [], reach: [], tags: [] },
            })
          )
        );

        service.search({ text: 'governance' }).subscribe(results => {
          expect(results.results).toEqual([]);
          expect(results.totalCount).toBe(0);
          expect(results.provenance.rankingKnown).toBe(false);
          expect(results.provenance.foldState).toBe('absent');
          done();
        });
      }));

    it('search_on_transport_error_returns_empty_with_foldState_unreachable', () =>
      new Promise<void>(done => {
        backendSpy.searchContentView.mockReturnValue(
          throwError(() => new Error('HTTP 502 - bad gateway'))
        );

        service.search({ text: 'governance' }).subscribe(results => {
          expect(results.results).toEqual([]);
          expect(results.totalCount).toBe(0);
          expect(results.provenance.foldState).toBe('unreachable');
          expect(results.provenance.rankingKnown).toBe(false);
          expect(results.executionTimeMs).toBeGreaterThanOrEqual(0);
          done();
        });
      }));

    it('search_paginates_from_the_answers_totalCount', () =>
      new Promise<void>(done => {
        backendSpy.searchContentView.mockReturnValue(of(peerAnswer({ totalCount: 7 })));

        service.search({ text: 'governance', page: 2, pageSize: 3 }).subscribe(results => {
          expect(results.page).toBe(2);
          expect(results.pageSize).toBe(3);
          expect(results.totalPages).toBe(3);
          expect(results.hasMore).toBe(true);
          done();
        });
      }));

    it('search_never_reads_the_local_content_index', () =>
      new Promise<void>(done => {
        service.search({ text: 'governance' }).subscribe(() => {
          expect(dataLoaderSpy.getContentIndex).not.toHaveBeenCalled();
          expect(dataLoaderSpy.getPathIndex).not.toHaveBeenCalled();
          done();
        });
      }));

    it('should include execution time in results', () =>
      new Promise<void>(done => {
        service.search({ text: '' }).subscribe(results => {
          expect(results.executionTimeMs).toBeDefined();
          expect(results.executionTimeMs).toBeGreaterThanOrEqual(0);
          done();
        });
      }));
  });

  // =========================================================================
  // Suggestions — still local (captured by R-S6)
  // =========================================================================

  describe('suggest', () => {
    it('suggest_still_reads_local_content_index', () =>
      new Promise<void>(done => {
        service.suggest('gov').subscribe(suggestions => {
          expect(dataLoaderSpy.getContentIndex).toHaveBeenCalled();
          expect(dataLoaderSpy.getPathIndex).toHaveBeenCalled();
          expect(backendSpy.searchContentView).not.toHaveBeenCalled();
          expect(suggestions.suggestions.length).toBeGreaterThan(0);
          done();
        });
      }));

    it('should return empty for short queries', () =>
      new Promise<void>(done => {
        service.suggest('g').subscribe(suggestions => {
          expect(suggestions.suggestions.length).toBe(0);
          done();
        });
      }));

    it('should return title suggestions', () =>
      new Promise<void>(done => {
        service.suggest('gov').subscribe(suggestions => {
          const titleSuggestions = suggestions.suggestions.filter(s => s.type === 'title');
          expect(titleSuggestions.length).toBeGreaterThan(0);
          done();
        });
      }));

    it('should return tag suggestions', () =>
      new Promise<void>(done => {
        service.suggest('gov').subscribe(suggestions => {
          const tagSuggestions = suggestions.suggestions.filter(s => s.type === 'tag');
          expect(tagSuggestions.length).toBeGreaterThan(0);
          done();
        });
      }));

    it('should include result count for tag suggestions', () =>
      new Promise<void>(done => {
        service.suggest('gov').subscribe(suggestions => {
          const tagSuggestion = suggestions.suggestions.find(s => s.type === 'tag');
          expect(tagSuggestion?.resultCount).toBeGreaterThan(0);
          done();
        });
      }));

    it('should highlight matched text', () =>
      new Promise<void>(done => {
        service.suggest('gov').subscribe(suggestions => {
          const suggestion = suggestions.suggestions[0];
          expect(suggestion?.highlight).toContain('<mark>');
          done();
        });
      }));

    it('should respect limit parameter', () =>
      new Promise<void>(done => {
        service.suggest('co', 2).subscribe(suggestions => {
          expect(suggestions.suggestions.length).toBeLessThanOrEqual(2);
          done();
        });
      }));

    it('should handle empty query', () =>
      new Promise<void>(done => {
        service.suggest('').subscribe(suggestions => {
          expect(suggestions.suggestions.length).toBe(0);
          done();
        });
      }));
  });

  // =========================================================================
  // Tag Cloud
  // =========================================================================

  describe('getTagCloud', () => {
    it('should return all tags with counts', () =>
      new Promise<void>(done => {
        service.getTagCloud().subscribe(cloud => {
          expect(cloud.length).toBeGreaterThan(0);
          for (const item of cloud) {
            expect(item.tag).toBeDefined();
            expect(item.count).toBeGreaterThan(0);
          }
          done();
        });
      }));

    it('should sort by count descending', () =>
      new Promise<void>(done => {
        service.getTagCloud().subscribe(cloud => {
          for (let i = 0; i < cloud.length - 1; i++) {
            expect(cloud[i].count).toBeGreaterThanOrEqual(cloud[i + 1].count);
          }
          done();
        });
      }));

    it('should have governance as most common tag', () =>
      new Promise<void>(done => {
        service.getTagCloud().subscribe(cloud => {
          expect(cloud[0].tag).toBe('governance');
          expect(cloud[0].count).toBe(3);
          done();
        });
      }));
  });
});
