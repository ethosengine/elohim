import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ProfileService } from './profile.service';
import { DataLoaderService } from './data-loader.service';
import { PathService } from '@app/lamad/services/path.service';
import { AffinityTrackingService } from './affinity-tracking.service';
import { AgentService } from './agent.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';

describe('ProfileService', () => {
  let service: ProfileService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;
  let pathServiceMock: jasmine.SpyObj<PathService>;
  let affinityServiceMock: jasmine.SpyObj<AffinityTrackingService>;
  let agentServiceMock: jasmine.SpyObj<AgentService>;
  let sessionHumanServiceMock: jasmine.SpyObj<SessionHumanService>;

  beforeEach(() => {
    const dataLoaderSpy = jasmine.createSpyObj('DataLoaderService', ['getPathIndex', 'getContent']);
    const pathServiceSpy = jasmine.createSpyObj('PathService', ['getPath']);
    const affinitySpy = jasmine.createSpyObj('AffinityTrackingService', ['trackView']);

    // Add affinitySubject property to the mock
    Object.defineProperty(affinitySpy, 'affinitySubject', {
      value: { value: { affinity: {} } },
      writable: true,
      configurable: true,
    });

    const agentServiceSpy = jasmine.createSpyObj('AgentService', [
      'getCurrentAgent',
      'getAgentProgress',
      'getAttestations',
    ]);
    const sessionHumanSpy = jasmine.createSpyObj('SessionHumanService', [
      'getSession',
      'getAllPathProgress',
      'getActivityHistory',
    ]);

    TestBed.configureTestingModule({
      providers: [
        ProfileService,
        { provide: DataLoaderService, useValue: dataLoaderSpy },
        { provide: PathService, useValue: pathServiceSpy },
        { provide: AffinityTrackingService, useValue: affinitySpy },
        { provide: AgentService, useValue: agentServiceSpy },
        { provide: SessionHumanService, useValue: sessionHumanSpy },
      ],
    });

    service = TestBed.inject(ProfileService);
    dataLoaderMock = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    pathServiceMock = TestBed.inject(PathService) as jasmine.SpyObj<PathService>;
    affinityServiceMock = TestBed.inject(
      AffinityTrackingService
    ) as jasmine.SpyObj<AffinityTrackingService>;
    agentServiceMock = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;
    sessionHumanServiceMock = TestBed.inject(
      SessionHumanService
    ) as jasmine.SpyObj<SessionHumanService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getProfile', () => {
    it('should have getProfile method', () => {
      expect(service.getProfile).toBeDefined();
      expect(typeof service.getProfile).toBe('function');
    });

    it('should return observable', () => {
      agentServiceMock.getCurrentAgent.and.returnValue(of(null));
      agentServiceMock.getAgentProgress.and.returnValue(of([]));
      agentServiceMock.getAttestations.and.returnValue([]);
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      const result = service.getProfile();

      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('getProfileSummary', () => {
    it('should have getProfileSummary method', () => {
      expect(service.getProfileSummary).toBeDefined();
      expect(typeof service.getProfileSummary).toBe('function');
    });

    it('should return observable of profile summary', (done) => {
      agentServiceMock.getCurrentAgent.and.returnValue(of(null));
      agentServiceMock.getAgentProgress.and.returnValue(of([]));
      agentServiceMock.getAttestations.and.returnValue([]);
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      service.getProfileSummary().subscribe({
        next: (result) => {
          expect(result).toEqual(
            jasmine.objectContaining({
              displayName: jasmine.any(String),
              isSessionBased: jasmine.any(Boolean),
            })
          );
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getJourneyStats', () => {
    it('should have getJourneyStats method', () => {
      expect(service.getJourneyStats).toBeDefined();
      expect(typeof service.getJourneyStats).toBe('function');
    });

    it('should return observable of journey stats', (done) => {
      sessionHumanServiceMock.getSession.and.returnValue(null);
      agentServiceMock.getAgentProgress.and.returnValue(of([]));

      service.getJourneyStats().subscribe({
        next: (result) => {
          expect(result).toEqual(
            jasmine.objectContaining({
              territoryExplored: jasmine.any(Number),
              journeysStarted: jasmine.any(Number),
              journeysCompleted: jasmine.any(Number),
            })
          );
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getCurrentFocus', () => {
    it('should have getCurrentFocus method', () => {
      expect(service.getCurrentFocus).toBeDefined();
      expect(typeof service.getCurrentFocus).toBe('function');
    });

    it('should return observable of current focus array', (done) => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      service.getCurrentFocus().subscribe({
        next: (result) => {
          expect(Array.isArray(result)).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should return empty array when no path progress', (done) => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      service.getCurrentFocus().subscribe({
        next: (result) => {
          expect(result).toEqual([]);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getDevelopedCapabilities', () => {
    it('should have getDevelopedCapabilities method', () => {
      expect(service.getDevelopedCapabilities).toBeDefined();
      expect(typeof service.getDevelopedCapabilities).toBe('function');
    });

    it('should return observable of capabilities array', (done) => {
      agentServiceMock.getAttestations.and.returnValue(['attestation-1', 'attestation-2']);

      service.getDevelopedCapabilities().subscribe({
        next: (result) => {
          expect(Array.isArray(result)).toBe(true);
          expect(result.length).toBe(2);
          done();
        },
        error: done.fail,
      });
    });

    it('should transform attestation IDs to capabilities', (done) => {
      agentServiceMock.getAttestations.and.returnValue(['test-attestation']);

      service.getDevelopedCapabilities().subscribe({
        next: (result) => {
          expect(result[0]).toEqual(
            jasmine.objectContaining({
              id: 'test-attestation',
              name: jasmine.any(String),
              description: jasmine.any(String),
            })
          );
          done();
        },
        error: done.fail,
      });
    });

    it('should return empty array when no attestations', (done) => {
      agentServiceMock.getAttestations.and.returnValue([]);

      service.getDevelopedCapabilities().subscribe({
        next: (result) => {
          expect(result).toEqual([]);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getTimeline', () => {
    it('should have getTimeline method', () => {
      expect(service.getTimeline).toBeDefined();
      expect(typeof service.getTimeline).toBe('function');
    });

    it('should return observable of timeline events array', (done) => {
      sessionHumanServiceMock.getActivityHistory.and.returnValue([]);

      service.getTimeline().subscribe({
        next: (result) => {
          expect(Array.isArray(result)).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should accept optional limit parameter', (done) => {
      sessionHumanServiceMock.getActivityHistory.and.returnValue([]);

      service.getTimeline(10).subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          done();
        },
        error: done.fail,
      });
    });

    it('should return empty array with no activities', (done) => {
      sessionHumanServiceMock.getActivityHistory.and.returnValue([]);

      service.getTimeline().subscribe({
        next: (result) => {
          expect(result).toEqual([]);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getTopEngagedContent', () => {
    it('should have getTopEngagedContent method', () => {
      expect(service.getTopEngagedContent).toBeDefined();
      expect(typeof service.getTopEngagedContent).toBe('function');
    });

    it('should return observable of engaged content array', (done) => {
      service.getTopEngagedContent().subscribe({
        next: (result) => {
          expect(Array.isArray(result)).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should accept optional limit parameter', (done) => {
      service.getTopEngagedContent(5).subscribe({
        next: (result) => {
          expect(Array.isArray(result)).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should return empty array when no affinity data', (done) => {
      service.getTopEngagedContent().subscribe({
        next: (result) => {
          expect(result).toEqual([]);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getAllNotes', () => {
    it('should have getAllNotes method', () => {
      expect(service.getAllNotes).toBeDefined();
      expect(typeof service.getAllNotes).toBe('function');
    });

    it('should return observable of notes array', (done) => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      service.getAllNotes().subscribe({
        next: (result) => {
          expect(Array.isArray(result)).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should return empty array with no path progress', (done) => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      service.getAllNotes().subscribe({
        next: (result) => {
          expect(result).toEqual([]);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getResumePoint', () => {
    it('should have getResumePoint method', () => {
      expect(service.getResumePoint).toBeDefined();
      expect(typeof service.getResumePoint).toBe('function');
    });

    it('should return observable of resume point', (done) => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      service.getResumePoint().subscribe({
        next: (result) => {
          expect(result).toEqual(
            jasmine.objectContaining({
              type: jasmine.any(String),
              title: jasmine.any(String),
            })
          );
          done();
        },
        error: done.fail,
      });
    });

    it('should suggest exploration when no active paths', (done) => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      service.getResumePoint().subscribe({
        next: (result) => {
          expect(result?.type).toBe('explore_new');
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getPathsOverview', () => {
    it('should have getPathsOverview method', () => {
      expect(service.getPathsOverview).toBeDefined();
      expect(typeof service.getPathsOverview).toBe('function');
    });

    it('should return observable of paths overview', (done) => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);
      dataLoaderMock.getPathIndex.and.returnValue(of({ paths: [], lastUpdated: new Date().toISOString(), totalCount: 0 }));

      service.getPathsOverview().subscribe({
        next: (result) => {
          expect(result).toEqual(
            jasmine.objectContaining({
              inProgress: jasmine.any(Array),
              completed: jasmine.any(Array),
              suggested: jasmine.any(Array),
            })
          );
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('Observable returns', () => {
    it('getProfile should return observable', () => {
      agentServiceMock.getCurrentAgent.and.returnValue(of(null));
      agentServiceMock.getAgentProgress.and.returnValue(of([]));
      agentServiceMock.getAttestations.and.returnValue([]);
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      const result = service.getProfile();

      expect(result.subscribe).toBeDefined();
    });

    it('getProfileSummary should return observable', () => {
      agentServiceMock.getCurrentAgent.and.returnValue(of(null));
      agentServiceMock.getAgentProgress.and.returnValue(of([]));
      agentServiceMock.getAttestations.and.returnValue([]);
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      const result = service.getProfileSummary();

      expect(result.subscribe).toBeDefined();
    });

    it('getJourneyStats should return observable', () => {
      sessionHumanServiceMock.getSession.and.returnValue(null);
      agentServiceMock.getAgentProgress.and.returnValue(of([]));

      const result = service.getJourneyStats();

      expect(result.subscribe).toBeDefined();
    });

    it('getCurrentFocus should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      const result = service.getCurrentFocus();

      expect(result.subscribe).toBeDefined();
    });

    it('getDevelopedCapabilities should return observable', () => {
      agentServiceMock.getAttestations.and.returnValue([]);

      const result = service.getDevelopedCapabilities();

      expect(result.subscribe).toBeDefined();
    });

    it('getTimeline should return observable', () => {
      sessionHumanServiceMock.getActivityHistory.and.returnValue([]);

      const result = service.getTimeline();

      expect(result.subscribe).toBeDefined();
    });

    it('getTopEngagedContent should return observable', () => {
      const result = service.getTopEngagedContent();

      expect(result.subscribe).toBeDefined();
    });

    it('getAllNotes should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      const result = service.getAllNotes();

      expect(result.subscribe).toBeDefined();
    });

    it('getResumePoint should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);

      const result = service.getResumePoint();

      expect(result.subscribe).toBeDefined();
    });

    it('getPathsOverview should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.and.returnValue([]);
      dataLoaderMock.getPathIndex.and.returnValue(of({ paths: [], lastUpdated: new Date().toISOString(), totalCount: 0 }));

      const result = service.getPathsOverview();

      expect(result.subscribe).toBeDefined();
    });
  });
});
