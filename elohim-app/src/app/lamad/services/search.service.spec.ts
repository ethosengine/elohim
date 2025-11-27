import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { SearchService } from './search.service';
import { DataLoaderService } from './data-loader.service';
import { TrustBadgeService } from './trust-badge.service';
import { SearchQuery } from '../models/search.model';

describe('SearchService', () => {
  let service: SearchService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;
  let trustBadgeMock: jasmine.SpyObj<TrustBadgeService>;

  const mockContentIndex = {
    nodes: [
      {
        id: 'node-1',
        title: 'Governance Protocol',
        description: 'A governance system for communities',
        contentType: 'epic',
        tags: ['governance', 'protocol'],
        reach: 'commons',
        trustScore: 0.8
      },
      {
        id: 'node-2',
        title: 'Authentication Feature',
        description: 'User authentication system',
        contentType: 'feature',
        tags: ['auth', 'security'],
        reach: 'community',
        trustScore: 0.6
      },
      {
        id: 'node-3',
        title: 'Login Scenario',
        description: 'Login flow scenario',
        contentType: 'scenario',
        tags: ['auth', 'login'],
        reach: 'commons',
        trustScore: 0.9
      }
    ]
  };

  beforeEach(() => {
    dataLoaderMock = jasmine.createSpyObj('DataLoaderService', ['getContentIndex']);
    trustBadgeMock = jasmine.createSpyObj('TrustBadgeService', ['getTrustBadge']);

    dataLoaderMock.getContentIndex.and.returnValue(of(mockContentIndex));
    trustBadgeMock.getTrustBadge.and.returnValue(of({
      level: 'high',
      score: 0.8,
      label: 'Trusted',
      color: 'green',
      icon: 'check'
    }));

    TestBed.configureTestingModule({
      providers: [
        SearchService,
        { provide: DataLoaderService, useValue: dataLoaderMock },
        { provide: TrustBadgeService, useValue: trustBadgeMock }
      ]
    });
    service = TestBed.inject(SearchService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('search', () => {
    it('should return results for text search', (done) => {
      const query: SearchQuery = { text: 'governance' };

      service.search(query).subscribe(results => {
        expect(results.results.length).toBeGreaterThan(0);
        expect(results.results[0].node.title).toContain('Governance');
        done();
      });
    });

    it('should return empty results for non-matching query', (done) => {
      const query: SearchQuery = { text: 'xyznonexistent' };

      service.search(query).subscribe(results => {
        expect(results.results.length).toBe(0);
        done();
      });
    });

    it('should filter by content type', (done) => {
      const query: SearchQuery = {
        text: 'auth',
        contentTypes: ['feature']
      };

      service.search(query).subscribe(results => {
        results.results.forEach(r => {
          expect(r.node.contentType).toBe('feature');
        });
        done();
      });
    });

    it('should return paginated results', (done) => {
      const query: SearchQuery = {
        text: '',
        page: 1,
        pageSize: 2
      };

      service.search(query).subscribe(results => {
        expect(results.page).toBe(1);
        expect(results.pageSize).toBe(2);
        expect(results.results.length).toBeLessThanOrEqual(2);
        done();
      });
    });

    it('should include facets in results', (done) => {
      const query: SearchQuery = { text: '' };

      service.search(query).subscribe(results => {
        expect(results.facets).toBeDefined();
        expect(results.facets.byType).toBeDefined();
        expect(results.facets.byReach).toBeDefined();
        done();
      });
    });

    it('should include total count', (done) => {
      const query: SearchQuery = { text: '' };

      service.search(query).subscribe(results => {
        expect(results.totalCount).toBeDefined();
        expect(results.totalCount).toBeGreaterThanOrEqual(0);
        done();
      });
    });

    it('should calculate relevance scores', (done) => {
      const query: SearchQuery = { text: 'governance' };

      service.search(query).subscribe(results => {
        if (results.results.length > 0) {
          expect(results.results[0].score).toBeDefined();
          expect(results.results[0].score).toBeGreaterThan(0);
        }
        done();
      });
    });

    it('should include execution time', (done) => {
      const query: SearchQuery = { text: 'auth' };

      service.search(query).subscribe(results => {
        expect(results.executionTimeMs).toBeDefined();
        expect(results.executionTimeMs).toBeGreaterThanOrEqual(0);
        done();
      });
    });
  });

  describe('suggest', () => {
    it('should return suggestions for partial text', (done) => {
      service.suggest('gov').subscribe(suggestions => {
        expect(suggestions).toBeDefined();
        done();
      });
    });

    it('should return empty suggestions for very short text', (done) => {
      service.suggest('').subscribe(suggestions => {
        expect(suggestions.suggestions.length).toBe(0);
        done();
      });
    });
  });
});
