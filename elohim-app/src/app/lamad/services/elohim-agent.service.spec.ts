import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { of } from 'rxjs';
import { ElohimAgentService } from './elohim-agent.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ElohimRequest, ElohimCapability, Agent } from '../models';

describe('ElohimAgentService', () => {
  let service: ElohimAgentService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;

  const mockAgentIndex: { agents: Agent[] } = {
    agents: [
      {
        id: 'elohim-guardian',
        displayName: 'Guardian Elohim',
        type: 'elohim',
        layer: 'community',
        capabilities: ['content-safety-review', 'attestation-recommendation'],
        visibility: 'public',
        bio: 'Test guardian',
        attestations: [],
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-01T00:00:00.000Z'
      },
      {
        id: 'elohim-family',
        displayName: 'Family Elohim',
        type: 'elohim',
        layer: 'family',
        capabilities: ['knowledge-map-synthesis'],
        visibility: 'private',
        familyId: 'family-123',
        bio: 'Family guardian',
        attestations: [],
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-01T00:00:00.000Z'
      },
      {
        id: 'human-agent',
        displayName: 'Human Agent',
        type: 'human',
        layer: 'individual',
        capabilities: [],
        visibility: 'private',
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-01T00:00:00.000Z'
      }
    ]
  };

  beforeEach(() => {
    dataLoaderSpy = jasmine.createSpyObj('DataLoaderService', ['getAgentIndex']);
    dataLoaderSpy.getAgentIndex.and.returnValue(of(mockAgentIndex));

    TestBed.configureTestingModule({
      providers: [
        ElohimAgentService,
        { provide: DataLoaderService, useValue: dataLoaderSpy }
      ]
    });

    service = TestBed.inject(ElohimAgentService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getElohimIndex', () => {
    it('should return only elohim agents', (done) => {
      service.getElohimIndex().subscribe(elohim => {
        expect(elohim.length).toBe(2);
        expect(elohim.every(e => e.id.startsWith('elohim'))).toBe(true);
        done();
      });
    });

    it('should return elohim with correct properties', (done) => {
      service.getElohimIndex().subscribe(elohim => {
        const guardian = elohim.find(e => e.id === 'elohim-guardian');
        expect(guardian).toBeTruthy();
        expect(guardian!.displayName).toBe('Guardian Elohim');
        expect(guardian!.layer).toBe('community');
        expect(guardian!.capabilities).toContain('content-safety-review');
        done();
      });
    });
  });

  describe('getElohim', () => {
    it('should return specific elohim by ID', (done) => {
      service.getElohim('elohim-guardian').subscribe(elohim => {
        expect(elohim).toBeTruthy();
        expect(elohim!.id).toBe('elohim-guardian');
        expect(elohim!.displayName).toBe('Guardian Elohim');
        done();
      });
    });

    it('should return null for non-existent elohim', (done) => {
      service.getElohim('non-existent').subscribe(elohim => {
        expect(elohim).toBeNull();
        done();
      });
    });

    it('should return null for non-elohim agents', (done) => {
      service.getElohim('human-agent').subscribe(elohim => {
        expect(elohim).toBeNull();
        done();
      });
    });

    it('should cache elohim after first fetch', (done) => {
      service.getElohim('elohim-guardian').subscribe(() => {
        dataLoaderSpy.getAgentIndex.calls.reset();

        service.getElohim('elohim-guardian').subscribe(elohim => {
          expect(elohim).toBeTruthy();
          expect(dataLoaderSpy.getAgentIndex).not.toHaveBeenCalled();
          done();
        });
      });
    });
  });

  describe('selectElohim', () => {
    it('should select elohim with matching capability', (done) => {
      service.selectElohim({
        capability: 'content-safety-review'
      }).subscribe(elohim => {
        expect(elohim).toBeTruthy();
        expect(elohim!.capabilities).toContain('content-safety-review');
        done();
      });
    });

    it('should return null if no elohim has capability', (done) => {
      service.selectElohim({
        capability: 'non-existent-capability' as ElohimCapability
      }).subscribe(elohim => {
        expect(elohim).toBeNull();
        done();
      });
    });

    it('should prefer layer-appropriate elohim', (done) => {
      service.selectElohim({
        capability: 'knowledge-map-synthesis',
        preferredLayer: 'family'
      }).subscribe(elohim => {
        expect(elohim).toBeTruthy();
        expect(elohim!.layer).toBe('family');
        done();
      });
    });

    it('should prefer family elohim for family context', (done) => {
      service.selectElohim({
        capability: 'knowledge-map-synthesis',
        contextFamilyId: 'family-123'
      }).subscribe(elohim => {
        expect(elohim).toBeTruthy();
        expect(elohim!.layer).toBe('family');
        done();
      });
    });
  });

  describe('invoke', () => {
    it('should decline if no elohim available', fakeAsync((done: DoneFn) => {
      const request: ElohimRequest = {
        requestId: 'test-req-1',
        targetElohimId: 'non-existent',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'test', reviewType: 'safety' },
        requesterId: 'test-user',
        priority: 'normal',
        requestedAt: new Date().toISOString()
      };

      service.invoke(request).subscribe(response => {
        expect(response.status).toBe('declined');
        expect(response.declineReason).toBeTruthy();
      });
      tick(3000);
    }));

    it('should decline if elohim lacks capability', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'test-req-2',
        targetElohimId: 'elohim-family',
        capability: 'content-safety-review', // family elohim doesn't have this
        params: { type: 'content-review', contentId: 'test', reviewType: 'safety' },
        requesterId: 'test-user',
        priority: 'normal',
        requestedAt: new Date().toISOString()
      };

      service.invoke(request).subscribe(response => {
        expect(response.status).toBe('declined');
        expect(response.declineReason).toContain('capability');
      });
      tick(3000);
    }));

    it('should fulfill valid request', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'test-req-3',
        targetElohimId: 'elohim-guardian',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'content-1', reviewType: 'safety' },
        requesterId: 'test-user',
        priority: 'normal',
        requestedAt: new Date().toISOString()
      };

      service.invoke(request).subscribe(response => {
        expect(response.status).toBe('fulfilled');
        expect(response.elohimId).toBe('elohim-guardian');
        expect(response.constitutionalReasoning).toBeTruthy();
      });
      tick(3000);
    }));

    it('should include constitutional reasoning in response', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'test-req-4',
        targetElohimId: 'elohim-guardian',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'content-1', reviewType: 'safety' },
        requesterId: 'test-user',
        priority: 'normal',
        requestedAt: new Date().toISOString()
      };

      service.invoke(request).subscribe(response => {
        expect(response.constitutionalReasoning.primaryPrinciple).toBeTruthy();
        expect(response.constitutionalReasoning.interpretation).toBeTruthy();
        expect(response.constitutionalReasoning.valuesWeighed).toBeDefined();
        expect(response.constitutionalReasoning.confidence).toBeGreaterThan(0);
      });
      tick(3000);
    }));

    it('should auto-select elohim when targetElohimId is auto', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'test-req-5',
        targetElohimId: 'auto',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'content-1', reviewType: 'safety' },
        requesterId: 'test-user',
        priority: 'normal',
        requestedAt: new Date().toISOString()
      };

      service.invoke(request).subscribe(response => {
        expect(response.status).toBe('fulfilled');
        expect(response.elohimId).toBe('elohim-guardian');
      });
      tick(3000);
    }));

    it('should log requests', fakeAsync(() => {
      const request: ElohimRequest = {
        requestId: 'test-req-6',
        targetElohimId: 'elohim-guardian',
        capability: 'content-safety-review',
        params: { type: 'content-review', contentId: 'content-1', reviewType: 'safety' },
        requesterId: 'test-user',
        priority: 'normal',
        requestedAt: new Date().toISOString()
      };

      service.invoke(request).subscribe();
      tick(3000);

      const recentRequests = service.getRecentRequests();
      expect(recentRequests.length).toBeGreaterThan(0);
      expect(recentRequests.some(r => r.requestId === 'test-req-6')).toBe(true);
    }));
  });

  describe('requestContentReview', () => {
    it('should create and invoke content review request', fakeAsync(() => {
      service.requestContentReview('content-123', 'safety', 'user-1').subscribe(response => {
        expect(response.status).toBe('fulfilled');
        expect(response.payload).toBeTruthy();
      });
      tick(3000);
    }));

    it('should include content review result in payload', fakeAsync(() => {
      service.requestContentReview('content-123', 'accuracy', 'user-1').subscribe(response => {
        const payload = response.payload as any;
        expect(payload.type).toBe('content-review');
        expect(payload.contentId).toBe('content-123');
        expect(typeof payload.approved).toBe('boolean');
      });
      tick(3000);
    }));
  });

  describe('requestAttestationRecommendation', () => {
    it('should create and invoke attestation recommendation request', fakeAsync(() => {
      service.requestAttestationRecommendation(
        'content-456',
        'accuracy',
        'user-1',
        'Evidence text'
      ).subscribe(response => {
        expect(response.status).toBe('fulfilled');
        expect(response.payload).toBeTruthy();
      });
      tick(3000);
    }));

    it('should include attestation recommendation in payload', fakeAsync(() => {
      service.requestAttestationRecommendation(
        'content-456',
        'quality',
        'user-1'
      ).subscribe(response => {
        const payload = response.payload as any;
        expect(payload.type).toBe('attestation-recommendation');
        expect(payload.contentId).toBe('content-456');
        expect(['grant', 'deny', 'defer']).toContain(payload.recommend);
      });
      tick(3000);
    }));
  });

  describe('getRecentRequests', () => {
    it('should return limited number of requests', fakeAsync(() => {
      // Make multiple requests
      for (let i = 0; i < 15; i++) {
        service.requestContentReview(`content-${i}`, 'safety', 'user-1').subscribe();
      }
      tick(30000);

      const recent = service.getRecentRequests(5);
      expect(recent.length).toBe(5);
    }));

    it('should return all requests if limit is larger', fakeAsync(() => {
      service.clearRequestLog();
      service.requestContentReview('content-1', 'safety', 'user-1').subscribe();
      service.requestContentReview('content-2', 'safety', 'user-1').subscribe();
      tick(5000);

      const recent = service.getRecentRequests(100);
      expect(recent.length).toBe(2);
    }));
  });

  describe('clearRequestLog', () => {
    it('should clear all logged requests', fakeAsync(() => {
      service.requestContentReview('content-1', 'safety', 'user-1').subscribe();
      tick(3000);

      expect(service.getRecentRequests().length).toBeGreaterThan(0);

      service.clearRequestLog();
      expect(service.getRecentRequests().length).toBe(0);
    }));
  });
});
