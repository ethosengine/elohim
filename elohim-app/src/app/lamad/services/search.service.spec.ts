import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { SearchService } from './search.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { TrustBadgeService } from '@app/elohim/services/trust-badge.service';
import { SearchQuery } from '../models/search.model';

describe('SearchService', () => {
  let service: SearchService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let trustBadgeSpy: jasmine.SpyObj<TrustBadgeService>;

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
        updatedAt: '2025-01-02T00:00:00.000Z'
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
        updatedAt: '2025-01-03T00:00:00.000Z'
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
        updatedAt: '2025-01-04T00:00:00.000Z'
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
        updatedAt: '2025-01-05T00:00:00.000Z'
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
        updatedAt: '2025-01-06T00:00:00.000Z'
      }
    ],
    lastUpdated: '2025-01-06T00:00:00.000Z'
  };

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', ['getContentIndex']);
    const trustBadgeSpyObj = jasmine.createSpyObj('TrustBadgeService', ['getTrustBadges']);

    TestBed.configureTestingModule({
      providers: [
        SearchService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: TrustBadgeService, useValue: trustBadgeSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    trustBadgeSpy = TestBed.inject(TrustBadgeService) as jasmine.SpyObj<TrustBadgeService>;

    dataLoaderSpy.getContentIndex.and.returnValue(of(mockContentIndex));

    service = TestBed.inject(SearchService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // Basic Search
  // =========================================================================

  describe('search', () => {
    it('should return all results when no text query', (done) => {
      service.search({ text: '' }).subscribe(results => {
        expect(results.totalCount).toBe(5);
        expect(results.results.length).toBe(5);
        done();
      });
    });

    it('should filter by text query', (done) => {
      service.search({ text: 'governance' }).subscribe(results => {
        expect(results.totalCount).toBeGreaterThan(0);
        // Should match title "Governance Framework" and tag "governance"
        expect(results.results.some(r => r.title.includes('Governance'))).toBe(true);
        done();
      });
    });

    it('should score title matches higher than description', (done) => {
      service.search({ text: 'governance' }).subscribe(results => {
        // "Governance Framework" should be ranked higher than "Protocol Implementation"
        // which only has governance in tags and description
        const governanceIdx = results.results.findIndex(r => r.title === 'Governance Framework');
        const protocolIdx = results.results.findIndex(r => r.title === 'Protocol Implementation');

        if (governanceIdx >= 0 && protocolIdx >= 0) {
          expect(governanceIdx).toBeLessThan(protocolIdx);
        }
        done();
      });
    });

    it('should include relevance score', (done) => {
      service.search({ text: 'governance' }).subscribe(results => {
        for (const result of results.results) {
          expect(result.relevanceScore).toBeDefined();
          expect(result.relevanceScore).toBeGreaterThan(0);
        }
        done();
      });
    });

    it('should include matched fields', (done) => {
      service.search({ text: 'governance' }).subscribe(results => {
        const governance = results.results.find(r => r.title === 'Governance Framework');
        expect(governance?.matchedFields.some(f => f.field === 'title')).toBe(true);
        done();
      });
    });

    it('should include highlights', (done) => {
      service.search({ text: 'governance' }).subscribe(results => {
        const governance = results.results.find(r => r.title === 'Governance Framework');
        expect(governance?.highlights.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should handle multi-word queries', (done) => {
      service.search({ text: 'governance design' }).subscribe(results => {
        // Should match content with both words
        expect(results.results.some(r =>
          r.title.toLowerCase().includes('governance') ||
          r.title.toLowerCase().includes('design')
        )).toBe(true);
        done();
      });
    });
  });

  // =========================================================================
  // Filtering
  // =========================================================================

  describe('filtering', () => {
    it('should filter by content type', (done) => {
      service.search({ text: '', contentTypes: ['epic', 'feature'] }).subscribe(results => {
        for (const result of results.results) {
          expect(['epic', 'feature']).toContain(result.contentType);
        }
        done();
      });
    });

    it('should filter by reach level', (done) => {
      service.search({ text: '', reachLevels: ['commons', 'regional'] }).subscribe(results => {
        for (const result of results.results) {
          expect(['commons', 'regional']).toContain(result.reach);
        }
        done();
      });
    });

    it('should filter by tags (OR logic)', (done) => {
      service.search({ text: '', tags: ['governance', 'trust'] }).subscribe(results => {
        for (const result of results.results) {
          const hasMatchingTag = result.tags.some(t =>
            t.toLowerCase() === 'governance' || t.toLowerCase() === 'trust'
          );
          expect(hasMatchingTag).toBe(true);
        }
        done();
      });
    });

    it('should filter by required tags (AND logic)', (done) => {
      service.search({ text: '', requiredTags: ['governance', 'design'] }).subscribe(results => {
        for (const result of results.results) {
          const tags = result.tags.map(t => t.toLowerCase());
          expect(tags).toContain('governance');
          expect(tags).toContain('design');
        }
        done();
      });
    });

    it('should filter by minimum trust score', (done) => {
      service.search({ text: '', minTrustScore: 0.8 }).subscribe(results => {
        for (const result of results.results) {
          expect(result.trustScore).toBeGreaterThanOrEqual(0.8);
        }
        done();
      });
    });

    it('should exclude flagged content', (done) => {
      service.search({ text: '', excludeFlagged: true }).subscribe(results => {
        for (const result of results.results) {
          expect(result.hasFlags).toBe(false);
        }
        done();
      });
    });

    it('should combine text search with filters', (done) => {
      service.search({
        text: 'governance',
        contentTypes: ['epic']
      }).subscribe(results => {
        expect(results.totalCount).toBeGreaterThan(0);
        for (const result of results.results) {
          expect(result.contentType).toBe('epic');
        }
        done();
      });
    });
  });

  // =========================================================================
  // Sorting
  // =========================================================================

  describe('sorting', () => {
    it('should sort by relevance (default)', (done) => {
      service.search({ text: 'governance' }).subscribe(results => {
        // Results should be in descending relevance order
        for (let i = 0; i < results.results.length - 1; i++) {
          expect(results.results[i].relevanceScore)
            .toBeGreaterThanOrEqual(results.results[i + 1].relevanceScore);
        }
        done();
      });
    });

    it('should sort by title ascending', (done) => {
      service.search({ text: '', sortBy: 'title', sortDirection: 'asc' }).subscribe(results => {
        for (let i = 0; i < results.results.length - 1; i++) {
          expect(results.results[i].title.localeCompare(results.results[i + 1].title))
            .toBeLessThanOrEqual(0);
        }
        done();
      });
    });

    it('should sort by trust score descending', (done) => {
      service.search({ text: '', sortBy: 'trustScore', sortDirection: 'desc' }).subscribe(results => {
        for (let i = 0; i < results.results.length - 1; i++) {
          expect(results.results[i].trustScore)
            .toBeGreaterThanOrEqual(results.results[i + 1].trustScore);
        }
        done();
      });
    });

    it('should sort by newest', (done) => {
      service.search({ text: '', sortBy: 'newest', sortDirection: 'desc' }).subscribe(results => {
        for (let i = 0; i < results.results.length - 1; i++) {
          expect(results.results[i].createdAt! >= results.results[i + 1].createdAt!).toBe(true);
        }
        done();
      });
    });
  });

  // =========================================================================
  // Pagination
  // =========================================================================

  describe('pagination', () => {
    it('should return first page by default', (done) => {
      service.search({ text: '' }).subscribe(results => {
        expect(results.page).toBe(1);
        done();
      });
    });

    it('should paginate results', (done) => {
      service.search({ text: '', pageSize: 2, page: 1 }).subscribe(results => {
        expect(results.results.length).toBe(2);
        expect(results.pageSize).toBe(2);
        expect(results.hasMore).toBe(true);
        done();
      });
    });

    it('should return correct page', (done) => {
      service.search({ text: '', pageSize: 2, page: 2 }).subscribe(results => {
        expect(results.page).toBe(2);
        expect(results.results.length).toBeLessThanOrEqual(2);
        done();
      });
    });

    it('should calculate total pages correctly', (done) => {
      service.search({ text: '', pageSize: 2 }).subscribe(results => {
        expect(results.totalPages).toBe(3); // 5 items / 2 per page = 3 pages
        done();
      });
    });

    it('should set hasMore correctly on last page', (done) => {
      service.search({ text: '', pageSize: 2, page: 3 }).subscribe(results => {
        expect(results.hasMore).toBe(false);
        done();
      });
    });
  });

  // =========================================================================
  // Facets
  // =========================================================================

  describe('facets', () => {
    it('should return facets with results', (done) => {
      service.search({ text: '' }).subscribe(results => {
        expect(results.facets).toBeDefined();
        expect(results.facets.byContentType.length).toBeGreaterThan(0);
        expect(results.facets.byReach.length).toBeGreaterThan(0);
        expect(results.facets.byTag.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should count content types correctly', (done) => {
      service.search({ text: '' }).subscribe(results => {
        const epicCount = results.facets.byContentType.find(f => f.value === 'epic');
        expect(epicCount?.count).toBe(1);
        done();
      });
    });

    it('should count tags correctly', (done) => {
      service.search({ text: '' }).subscribe(results => {
        const governanceTag = results.facets.byTag.find(f => f.value === 'governance');
        expect(governanceTag?.count).toBe(3); // 3 items have governance tag
        done();
      });
    });

    it('should count flag status', (done) => {
      service.search({ text: '' }).subscribe(results => {
        expect(results.facets.byFlagStatus.flagged).toBe(1);
        expect(results.facets.byFlagStatus.unflagged).toBe(4);
        done();
      });
    });

    it('should mark selected facet values', (done) => {
      service.search({ text: '', contentTypes: ['epic'] }).subscribe(results => {
        const epicFacet = results.facets.byContentType.find(f => f.value === 'epic');
        expect(epicFacet?.selected).toBe(true);
        done();
      });
    });
  });

  // =========================================================================
  // Suggestions
  // =========================================================================

  describe('suggest', () => {
    it('should return empty for short queries', (done) => {
      service.suggest('g').subscribe(suggestions => {
        expect(suggestions.suggestions.length).toBe(0);
        done();
      });
    });

    it('should return title suggestions', (done) => {
      service.suggest('gov').subscribe(suggestions => {
        const titleSuggestions = suggestions.suggestions.filter(s => s.type === 'title');
        expect(titleSuggestions.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should return tag suggestions', (done) => {
      service.suggest('gov').subscribe(suggestions => {
        const tagSuggestions = suggestions.suggestions.filter(s => s.type === 'tag');
        expect(tagSuggestions.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should include result count for tag suggestions', (done) => {
      service.suggest('gov').subscribe(suggestions => {
        const tagSuggestion = suggestions.suggestions.find(s => s.type === 'tag');
        expect(tagSuggestion?.resultCount).toBeGreaterThan(0);
        done();
      });
    });

    it('should highlight matched text', (done) => {
      service.suggest('gov').subscribe(suggestions => {
        const suggestion = suggestions.suggestions[0];
        expect(suggestion?.highlight).toContain('<mark>');
        done();
      });
    });

    it('should respect limit parameter', (done) => {
      service.suggest('co', 2).subscribe(suggestions => {
        expect(suggestions.suggestions.length).toBeLessThanOrEqual(2);
        done();
      });
    });

    it('should handle empty query', (done) => {
      service.suggest('').subscribe(suggestions => {
        expect(suggestions.suggestions.length).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Tag Cloud
  // =========================================================================

  describe('getTagCloud', () => {
    it('should return all tags with counts', (done) => {
      service.getTagCloud().subscribe(cloud => {
        expect(cloud.length).toBeGreaterThan(0);
        for (const item of cloud) {
          expect(item.tag).toBeDefined();
          expect(item.count).toBeGreaterThan(0);
        }
        done();
      });
    });

    it('should sort by count descending', (done) => {
      service.getTagCloud().subscribe(cloud => {
        for (let i = 0; i < cloud.length - 1; i++) {
          expect(cloud[i].count).toBeGreaterThanOrEqual(cloud[i + 1].count);
        }
        done();
      });
    });

    it('should have governance as most common tag', (done) => {
      service.getTagCloud().subscribe(cloud => {
        expect(cloud[0].tag).toBe('governance');
        expect(cloud[0].count).toBe(3);
        done();
      });
    });
  });

  // =========================================================================
  // Error Handling
  // =========================================================================

  describe('error handling', () => {
    it('should handle data loader errors gracefully', (done) => {
      dataLoaderSpy.getContentIndex.and.returnValue(throwError(() => new Error('Network error')));

      service.search({ text: 'test' }).subscribe(results => {
        expect(results.totalCount).toBe(0);
        expect(results.results.length).toBe(0);
        done();
      });
    });

    it('should include execution time in results', (done) => {
      service.search({ text: '' }).subscribe(results => {
        expect(results.executionTimeMs).toBeDefined();
        expect(results.executionTimeMs).toBeGreaterThanOrEqual(0);
        done();
      });
    });

    it('should handle empty content index', (done) => {
      dataLoaderSpy.getContentIndex.and.returnValue(of({ nodes: [] }));

      service.search({ text: 'test' }).subscribe(results => {
        expect(results.totalCount).toBe(0);
        expect(results.results.length).toBe(0);
        done();
      });
    });

    it('should handle undefined nodes', (done) => {
      dataLoaderSpy.getContentIndex.and.returnValue(of({}));

      service.search({ text: '' }).subscribe(results => {
        expect(results.totalCount).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Match Types
  // =========================================================================

  describe('match scoring', () => {
    it('should score exact matches higher than contains', (done) => {
      // Add a node with exact word "trust" and one with "trustworthy"
      const indexWithMatches = {
        nodes: [
          {
            id: 'exact',
            title: 'Building Trust',
            description: 'About trust',
            contentType: 'concept',
            tags: [],
            reach: 'commons',
            trustScore: 1,
            flags: []
          },
          {
            id: 'contains',
            title: 'Trustworthy Systems',
            description: 'About trustworthiness',
            contentType: 'concept',
            tags: [],
            reach: 'commons',
            trustScore: 1,
            flags: []
          }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(indexWithMatches));

      service.search({ text: 'trust' }).subscribe(results => {
        const exactMatch = results.results.find(r => r.id === 'exact');
        const containsMatch = results.results.find(r => r.id === 'contains');

        if (exactMatch && containsMatch) {
          expect(exactMatch.relevanceScore).toBeGreaterThanOrEqual(containsMatch.relevanceScore);
        }
        done();
      });
    });
  });
});
