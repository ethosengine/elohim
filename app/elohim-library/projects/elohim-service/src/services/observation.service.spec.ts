// @vitest-environment jsdom
import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ObservationService } from './observation.service';

describe('ObservationService', () => {
  let service: ObservationService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [ObservationService],
    });
    service = TestBed.inject(ObservationService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    if (http) {
      http.verify();
    }
  });

  describe('bySubject', () => {
    it('should call /api/observations/by-subject with subjectCid and kind', () => {
      service.bySubject('doorway:abc', 'infrastructure:doorway-heartbeat').subscribe();
      const req = http.expectOne(r =>
        r.url === '/api/observations/by-subject' &&
        r.params.get('subjectCid') === 'doorway:abc' &&
        r.params.get('kind') === 'infrastructure:doorway-heartbeat'
      );
      expect(req.request.method).toBe('GET');
      req.flush([]);
    });

    it('should return empty array when no observations found', () => {
      let received: any = undefined;
      service.bySubject('doorway:xyz', 'infrastructure:blob-served').subscribe(obs => {
        received = obs;
      });
      const req = http.expectOne(r => r.url === '/api/observations/by-subject');
      req.flush([]);
      expect(received).toEqual([]);
    });

    it('should return typed ObservationView array', () => {
      let received: any = undefined;
      service.bySubject('content:123', 'engagement:viewed').subscribe(obs => {
        received = obs;
      });
      const mockObservations = [
        {
          observerCid: 'agent:observer1',
          logCid: 'blob:log1',
          logOffset: 100n,
          observedAt: 1234567890n,
          seq: 1n,
          observationKind: 'engagement:viewed',
          subjectCid: 'content:123',
          subjectKind: 'content',
          payloadJson: '{}',
          observerHouseholdCid: 'household:1',
          observerCollectiveCid: null,
          observerRegion: 'north-america',
          observerArchetype: 'learner',
          observerComputeClass: 'mobile',
          signatureB64: 'sig123',
        },
      ];
      const req = http.expectOne(r => r.url === '/api/observations/by-subject');
      req.flush(mockObservations);
      expect(received).toHaveLength(1);
      expect(received[0].observationKind).toBe('engagement:viewed');
    });
  });

  describe('byObserver', () => {
    it('should pass observerCid and kind when provided', () => {
      service.byObserver('agent:a', 'infrastructure:blob-served').subscribe();
      const req = http.expectOne(r => r.url === '/api/observations/by-observer');
      expect(req.request.params.get('observerCid')).toBe('agent:a');
      expect(req.request.params.get('kind')).toBe('infrastructure:blob-served');
      req.flush([]);
    });

    it('should omit kind when undefined', () => {
      service.byObserver('agent:a').subscribe();
      const req = http.expectOne(r => r.url === '/api/observations/by-observer');
      expect(req.request.params.get('observerCid')).toBe('agent:a');
      expect(req.request.params.has('kind')).toBe(false);
      req.flush([]);
    });

    it('should return observations from specific observer', () => {
      let received: any = undefined;
      service.byObserver('household:main').subscribe(obs => {
        received = obs;
      });
      const mockObservations = [
        {
          observerCid: 'household:main',
          logCid: 'blob:log1',
          logOffset: 10n,
          observedAt: 1234567800n,
          seq: 1n,
          observationKind: 'engagement:scrolled',
          subjectCid: 'content:456',
          subjectKind: 'content',
          payloadJson: '{"depth": 75}',
          observerHouseholdCid: 'household:main',
          observerCollectiveCid: null,
          observerRegion: 'west-coast',
          observerArchetype: 'contributor',
          observerComputeClass: 'desktop',
          signatureB64: 'sig456',
        },
      ];
      const req = http.expectOne(r => r.url === '/api/observations/by-observer');
      req.flush(mockObservations);
      expect(received).toHaveLength(1);
      expect(received[0].observerCid).toBe('household:main');
    });
  });

  describe('diversity', () => {
    it('should call /api/observations/diversity with subjectCid and kind', () => {
      service.diversity('doorway:abc', 'infrastructure:doorway-heartbeat').subscribe();
      const req = http.expectOne(r => r.url === '/api/observations/diversity');
      expect(req.request.params.get('subjectCid')).toBe('doorway:abc');
      expect(req.request.params.get('kind')).toBe('infrastructure:doorway-heartbeat');
      req.flush({
        subjectCid: 'doorway:abc',
        observationKind: 'infrastructure:doorway-heartbeat',
        distinctAgents: 5n,
        distinctHouseholds: 3n,
        distinctCollectives: 1n,
        distinctRegions: 1n,
        distinctArchetypes: 2n,
        distinctComputeClasses: 1n,
        totalCount: 5n,
        firstObservedAt: 0n,
        lastObservedAt: 0n,
      });
    });

    it('should return diversity summary view shape', () => {
      let received: any = undefined;
      service.diversity('content:789', 'engagement:viewed').subscribe(s => {
        received = s;
      });
      const mockSummary = {
        subjectCid: 'content:789',
        observationKind: 'engagement:viewed',
        distinctAgents: 12n,
        distinctHouseholds: 5n,
        distinctCollectives: 2n,
        distinctRegions: 3n,
        distinctArchetypes: 4n,
        distinctComputeClasses: 2n,
        totalCount: 25n,
        firstObservedAt: 1000000000n,
        lastObservedAt: 1000001000n,
      };
      const req = http.expectOne(r => r.url === '/api/observations/diversity');
      req.flush(mockSummary);
      expect(received).toBeDefined();
      expect(received.distinctHouseholds).toBe(5n);
      expect(received.totalCount).toBe(25n);
    });

    it('should return null when no observations exist', () => {
      let received: any = undefined;
      service.diversity('content:empty', 'engagement:never-viewed').subscribe(s => {
        received = s;
      });
      const req = http.expectOne(r => r.url === '/api/observations/diversity');
      req.flush(null);
      expect(received).toBeNull();
    });
  });
});
