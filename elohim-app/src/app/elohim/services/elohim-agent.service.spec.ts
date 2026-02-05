import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { of } from 'rxjs';

import { ElohimAgentService } from './elohim-agent.service';
import { DataLoaderService } from './data-loader.service';
import { ElohimRequest, ElohimResponse, ElohimIndexEntry, ElohimAgent } from '../models/elohim-agent.model';

describe('ElohimAgentService', () => {
  let service: ElohimAgentService;
  let mockDataLoader: jasmine.SpyObj<DataLoaderService>;

  const mockElohimIndexEntry: ElohimIndexEntry = {
    id: 'elohim-1',
    displayName: 'Wisdom Guardian',
    layer: 'family',
    capabilities: ['content-safety-review', 'attestation-recommendation'],
    visibility: 'public',
  };

  const mockElohimAgent: ElohimAgent = {
    id: 'elohim-1',
    displayName: 'Wisdom Guardian',
    layer: 'family',
    bio: 'Protects content integrity',
    attestations: [],
    capabilities: ['content-safety-review', 'attestation-recommendation'],
    visibility: 'public',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  beforeEach(() => {
    mockDataLoader = jasmine.createSpyObj('DataLoaderService', ['getAgentIndex']);
    mockDataLoader.getAgentIndex.and.returnValue(
      of({
        agents: [
          {
            id: 'elohim-1',
            type: 'elohim',
            displayName: 'Wisdom Guardian',
            layer: 'family',
            bio: 'Protects content integrity',
            attestations: [],
            capabilities: ['content-safety-review', 'attestation-recommendation'],
            visibility: 'public',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        ],
      })
    );

    TestBed.configureTestingModule({
      providers: [
        ElohimAgentService,
        { provide: DataLoaderService, useValue: mockDataLoader },
      ],
    });

    service = TestBed.inject(ElohimAgentService);
  });

  // ===========================================================================
  // Service Creation
  // ===========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should have getElohimIndex method', () => {
      expect(service.getElohimIndex).toBeDefined();
      expect(typeof service.getElohimIndex).toBe('function');
    });

    it('should have getElohim method', () => {
      expect(service.getElohim).toBeDefined();
      expect(typeof service.getElohim).toBe('function');
    });

    it('should have selectElohim method', () => {
      expect(service.selectElohim).toBeDefined();
      expect(typeof service.selectElohim).toBe('function');
    });

    it('should have invoke method', () => {
      expect(service.invoke).toBeDefined();
      expect(typeof service.invoke).toBe('function');
    });

    it('should have requestContentReview method', () => {
      expect(service.requestContentReview).toBeDefined();
      expect(typeof service.requestContentReview).toBe('function');
    });

    it('should have requestAttestationRecommendation method', () => {
      expect(service.requestAttestationRecommendation).toBeDefined();
      expect(typeof service.requestAttestationRecommendation).toBe('function');
    });

    it('should have getRecentRequests method', () => {
      expect(service.getRecentRequests).toBeDefined();
      expect(typeof service.getRecentRequests).toBe('function');
    });

    it('should have clearRequestLog method', () => {
      expect(service.clearRequestLog).toBeDefined();
      expect(typeof service.clearRequestLog).toBe('function');
    });
  });

  // ===========================================================================
  // getElohimIndex
  // ===========================================================================

  describe('getElohimIndex', () => {
    it('should return observable of Elohim index entries', (done) => {
      service.getElohimIndex().subscribe((index) => {
        expect(index).toBeDefined();
        expect(Array.isArray(index)).toBe(true);
        done();
      });
    });

    it('should filter only elohim agents', (done) => {
      service.getElohimIndex().subscribe((index) => {
        expect(index.length).toBeGreaterThan(0);
        expect(index[0].id).toBe('elohim-1');
        done();
      });
    });

    it('should return index with required fields', (done) => {
      service.getElohimIndex().subscribe((index) => {
        const entry = index[0];
        expect(entry.id).toBeDefined();
        expect(entry.displayName).toBeDefined();
        expect(entry.layer).toBeDefined();
        expect(entry.capabilities).toBeDefined();
        expect(entry.visibility).toBeDefined();
        done();
      });
    });

    it('should return empty array when no Elohim available', (done) => {
      mockDataLoader.getAgentIndex.and.returnValue(of({ agents: [] }));

      service.getElohimIndex().subscribe((index) => {
        expect(index).toEqual([]);
        done();
      });
    });
  });

  // ===========================================================================
  // getElohim
  // ===========================================================================

  describe('getElohim', () => {
    it('should return Elohim by ID', (done) => {
      service.getElohim('elohim-1').subscribe((elohim) => {
        expect(elohim).toBeDefined();
        expect(elohim?.id).toBe('elohim-1');
        done();
      });
    });

    it('should return Elohim with full agent data', (done) => {
      service.getElohim('elohim-1').subscribe((elohim) => {
        expect(elohim?.displayName).toBe('Wisdom Guardian');
        expect(elohim?.layer).toBe('family');
        expect(elohim?.bio).toBeDefined();
        expect(elohim?.capabilities).toBeDefined();
        done();
      });
    });

    it('should return null for non-existent Elohim', (done) => {
      service.getElohim('nonexistent').subscribe((elohim) => {
        expect(elohim).toBeNull();
        done();
      });
    });

    it('should cache Elohim after first fetch', (done) => {
      service.getElohim('elohim-1').subscribe(() => {
        expect(mockDataLoader.getAgentIndex).toHaveBeenCalledTimes(1);

        // Second call should use cache
        service.getElohim('elohim-1').subscribe(() => {
          expect(mockDataLoader.getAgentIndex).toHaveBeenCalledTimes(1);
          done();
        });
      });
    });

    it('should not cache non-existent Elohim', (done) => {
      service.getElohim('nonexistent').subscribe(() => {
        service.getElohim('nonexistent').subscribe(() => {
          expect(mockDataLoader.getAgentIndex).toHaveBeenCalledTimes(2);
          done();
        });
      });
    });
  });

  // ===========================================================================
  // selectElohim
  // ===========================================================================

  describe('selectElohim', () => {
    it('should select Elohim by capability', (done) => {
      service.selectElohim({ capability: 'content-safety-review' }).subscribe((elohim) => {
        expect(elohim).toBeDefined();
        expect(elohim?.id).toBe('elohim-1');
        done();
      });
    });

    it('should return null if no Elohim has capability', (done) => {
      service
        .selectElohim({ capability: 'unknown-capability' as any })
        .subscribe((elohim) => {
          expect(elohim).toBeNull();
          done();
        });
    });

    it('should prefer specified layer', (done) => {
      mockDataLoader.getAgentIndex.and.returnValue(
        of({
          agents: [
            {
              id: 'elohim-family',
              type: 'elohim',
              displayName: 'Family Guardian',
              layer: 'family',
              capabilities: ['content-safety-review'],
              visibility: 'public',
            },
            {
              id: 'elohim-global',
              type: 'elohim',
              displayName: 'Global Guardian',
              layer: 'global',
              capabilities: ['content-safety-review'],
              visibility: 'public',
            },
          ],
        } as any)
      );

      service
        .selectElohim({
          capability: 'content-safety-review',
          preferredLayer: 'global',
        })
        .subscribe((elohim) => {
          expect(elohim?.layer).toBe('global');
          done();
        });
    });

    it('should prefer family layer for family context', (done) => {
      mockDataLoader.getAgentIndex.and.returnValue(
        of({
          agents: [
            {
              id: 'elohim-family',
              type: 'elohim',
              displayName: 'Family Guardian',
              layer: 'family',
              capabilities: ['content-safety-review'],
              visibility: 'public',
            },
            {
              id: 'elohim-global',
              type: 'elohim',
              displayName: 'Global Guardian',
              layer: 'global',
              capabilities: ['content-safety-review'],
              visibility: 'public',
            },
          ],
        } as any)
      );

      service
        .selectElohim({
          capability: 'content-safety-review',
          contextFamilyId: 'family-123',
        })
        .subscribe((elohim) => {
          expect(elohim?.layer).toBe('family');
          done();
        });
    });
  });

  // ===========================================================================
  // invoke (Core Method)
  // ===========================================================================

  describe('invoke', () => {
    it('should invoke Elohim with request', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'req-123',
        targetElohimId: 'elohim-1',
        capability: 'content-safety-review',
        params: {
          type: 'content-review',
          contentId: 'content-123',
          reviewType: 'safety',
        },
        requesterId: 'user-123',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      let response: ElohimResponse | undefined;
      service.invoke(request).subscribe((r) => {
        response = r;
      });

      tick(2000); // Processing time

      expect(response).toBeDefined();
      expect(response?.requestId).toBe('req-123');
    }));

    it('should log request to request log', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'req-123',
        targetElohimId: 'elohim-1',
        capability: 'content-safety-review',
        params: {
          type: 'content-review',
          contentId: 'content-123',
          reviewType: 'safety',
        },
        requesterId: 'user-123',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      service.invoke(request).subscribe();
      tick(2000);

      const recentRequests = service.getRecentRequests();
      expect(recentRequests.length).toBeGreaterThan(0);
      expect(recentRequests[0].requestId).toBe('req-123');
    }));

    it('should handle auto-selection of Elohim', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'req-123',
        targetElohimId: 'auto',
        capability: 'content-safety-review',
        params: {
          type: 'content-review',
          contentId: 'content-123',
          reviewType: 'safety',
        },
        requesterId: 'user-123',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      let response: ElohimResponse | undefined;
      service.invoke(request).subscribe((r) => {
        response = r;
      });

      tick(2000);

      expect(response).toBeDefined();
    }));

    it('should decline if Elohim not found', fakeAsync(() => {
      mockDataLoader.getAgentIndex.and.returnValue(of({ agents: [] }));

      const request: ElohimRequest = {
        requestId: 'req-123',
        targetElohimId: 'nonexistent',
        capability: 'content-safety-review',
        params: {
          type: 'content-review',
          contentId: 'content-123',
          reviewType: 'safety',
        },
        requesterId: 'user-123',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      let response: ElohimResponse | undefined;
      service.invoke(request).subscribe((r) => {
        response = r;
      });

      tick(2000);

      expect(response?.status).toBe('declined');
      expect(response?.declineReason).toBeDefined();
    }));

    it('should decline if Elohim lacks capability', fakeAsync(() => {
      mockDataLoader.getAgentIndex.and.returnValue(
        of({
          agents: [
            {
              id: 'elohim-1',
              type: 'elohim',
              displayName: 'Limited Guardian',
              layer: 'family',
              capabilities: ['other-capability'],
              visibility: 'public',
            },
          ],
        } as any)
      );

      const request: ElohimRequest = {
        requestId: 'req-123',
        targetElohimId: 'elohim-1',
        capability: 'content-safety-review',
        params: {
          type: 'content-review',
          contentId: 'content-123',
          reviewType: 'safety',
        },
        requesterId: 'user-123',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      let response: ElohimResponse | undefined;
      service.invoke(request).subscribe((r) => {
        response = r;
      });

      tick(2000);

      expect(response?.status).toBe('declined');
      expect(response?.declineReason).toContain('does not have');
    }));
  });

  // ===========================================================================
  // requestContentReview (Convenience Method)
  // ===========================================================================

  describe('requestContentReview', () => {
    it('should create content review request', fakeAsync(() => {
      let response: ElohimResponse | undefined;

      service
        .requestContentReview('content-123', 'safety', 'user-123')
        .subscribe((r) => {
          response = r;
        });

      tick(2000);

      expect(response).toBeDefined();
      expect(response?.status).toBeDefined();
    }));

    it('should set correct capability', fakeAsync(() => {
      service.requestContentReview('content-123', 'safety', 'user-123').subscribe();

      tick(2000);

      const recentRequests = service.getRecentRequests();
      expect(recentRequests[0].capability).toBe('content-safety-review');
    }));

    it('should include review parameters', fakeAsync(() => {
      service.requestContentReview('content-123', 'accuracy', 'user-123').subscribe();

      tick(2000);

      const recentRequests = service.getRecentRequests();
      const params = recentRequests[0].params as any;
      expect(params.contentId).toBe('content-123');
      expect(params.reviewType).toBe('accuracy');
    }));
  });

  // ===========================================================================
  // requestAttestationRecommendation (Convenience Method)
  // ===========================================================================

  describe('requestAttestationRecommendation', () => {
    it('should create attestation recommendation request', fakeAsync(() => {
      let response: ElohimResponse | undefined;

      service
        .requestAttestationRecommendation('content-123', 'fact-check', 'user-123')
        .subscribe((r) => {
          response = r;
        });

      tick(2000);

      expect(response).toBeDefined();
      expect(response?.status).toBeDefined();
    }));

    it('should set correct capability', fakeAsync(() => {
      service
        .requestAttestationRecommendation('content-123', 'fact-check', 'user-123')
        .subscribe();

      tick(2000);

      const recentRequests = service.getRecentRequests();
      expect(recentRequests[0].capability).toBe('attestation-recommendation');
    }));

    it('should include optional evidence', fakeAsync(() => {
      service
        .requestAttestationRecommendation(
          'content-123',
          'fact-check',
          'user-123',
          'peer reviewed'
        )
        .subscribe();

      tick(2000);

      const recentRequests = service.getRecentRequests();
      const params = recentRequests[0].params as any;
      expect(params.evidence).toBe('peer reviewed');
    }));
  });

  // ===========================================================================
  // Request Log Management
  // ===========================================================================

  describe('request log management', () => {
    it('should return recent requests with default limit', fakeAsync(() => {
      const request1: ElohimRequest = {
        requestId: 'req-1',
        targetElohimId: 'elohim-1',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'c1', reviewType: 'safety' },
        requesterId: 'user-1',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      service.invoke(request1).subscribe();
      tick(2000);

      const recent = service.getRecentRequests();
      expect(recent.length).toBeGreaterThan(0);
    }));

    it('should clear request log', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'req-123',
        targetElohimId: 'elohim-1',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'c1', reviewType: 'safety' },
        requesterId: 'user-1',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      service.invoke(request).subscribe();
      tick(2000);

      service.clearRequestLog();
      const recent = service.getRecentRequests();
      expect(recent.length).toBe(0);
    }));

    it('should respect limit parameter in getRecentRequests', fakeAsync(() => {
      service.clearRequestLog();

      const request: ElohimRequest = {
        requestId: 'req-123',
        targetElohimId: 'elohim-1',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'c1', reviewType: 'safety' },
        requesterId: 'user-1',
        priority: 'normal',
        requestedAt: new Date().toISOString(),
      };

      service.invoke(request).subscribe();
      tick(2000);

      const recent = service.getRecentRequests(1);
      expect(recent.length).toBeLessThanOrEqual(1);
    }));
  });
});
