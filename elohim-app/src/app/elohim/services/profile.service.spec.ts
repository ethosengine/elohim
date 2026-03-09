import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ProfileService } from './profile.service';
import { DataLoaderService } from './data-loader.service';
import { PathService } from '@app/lamad/services/path.service';
import { AffinityTrackingService } from './affinity-tracking.service';
import { AgentService } from './agent.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { vi, Mock } from 'vitest';

describe('ProfileService', () => {
  let service: ProfileService;
  let dataLoaderMock: any;
  let pathServiceMock: any;
  let affinityServiceMock: any;
  let agentServiceMock: any;
  let sessionHumanServiceMock: any;

  beforeEach(() => {
    const dataLoaderSpy = {
      getPathIndex: vi.fn(),
      getContent: vi.fn(),
    };
    const pathServiceSpy = {
      getPath: vi.fn(),
    };
    const affinitySpy = {
      trackView: vi.fn(),
    };

    // Add affinitySubject property to the mock
    Object.defineProperty(affinitySpy, 'affinitySubject', {
      value: { value: { affinity: {} } },
      writable: true,
      configurable: true,
    });

    const agentServiceSpy = {
      getCurrentAgent: vi.fn(),
      getAgentProgress: vi.fn(),
      getAttestations: vi.fn(),
    };
    const sessionHumanSpy = {
      getSession: vi.fn(),
      getAllPathProgress: vi.fn(),
      getActivityHistory: vi.fn(),
    };

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
    dataLoaderMock = TestBed.inject(DataLoaderService) as { [K in keyof DataLoaderService]?: Mock };
    pathServiceMock = TestBed.inject(PathService) as { [K in keyof PathService]?: Mock };
    affinityServiceMock = TestBed.inject(AffinityTrackingService) as {
      [K in keyof AffinityTrackingService]?: Mock;
    };
    agentServiceMock = TestBed.inject(AgentService) as { [K in keyof AgentService]?: Mock };
    sessionHumanServiceMock = TestBed.inject(SessionHumanService) as {
      [K in keyof SessionHumanService]?: Mock;
    };
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
      agentServiceMock.getCurrentAgent.mockReturnValue(of(null));
      agentServiceMock.getAgentProgress.mockReturnValue(of([]));
      agentServiceMock.getAttestations.mockReturnValue([]);
      sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

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

    it('should return observable of profile summary', () =>
      new Promise<void>(done => {
        agentServiceMock.getCurrentAgent.mockReturnValue(of(null));
        agentServiceMock.getAgentProgress.mockReturnValue(of([]));
        agentServiceMock.getAttestations.mockReturnValue([]);
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

        service.getProfileSummary().subscribe({
          next: result => {
            expect(result).toEqual(
              expect.objectContaining({
                displayName: expect.any(String),
                isSessionBased: expect.any(Boolean),
              })
            );
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getJourneyStats', () => {
    it('should have getJourneyStats method', () => {
      expect(service.getJourneyStats).toBeDefined();
      expect(typeof service.getJourneyStats).toBe('function');
    });

    it('should return observable of journey stats', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getSession.mockReturnValue(null);
        agentServiceMock.getAgentProgress.mockReturnValue(of([]));

        service.getJourneyStats().subscribe({
          next: result => {
            expect(result).toEqual(
              expect.objectContaining({
                territoryExplored: expect.any(Number),
                journeysStarted: expect.any(Number),
                journeysCompleted: expect.any(Number),
              })
            );
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getCurrentFocus', () => {
    it('should have getCurrentFocus method', () => {
      expect(service.getCurrentFocus).toBeDefined();
      expect(typeof service.getCurrentFocus).toBe('function');
    });

    it('should return observable of current focus array', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

        service.getCurrentFocus().subscribe({
          next: result => {
            expect(Array.isArray(result)).toBe(true);
            done();
          },
          error: done.fail,
        });
      }));

    it('should return empty array when no path progress', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

        service.getCurrentFocus().subscribe({
          next: result => {
            expect(result).toEqual([]);
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getDevelopedCapabilities', () => {
    it('should have getDevelopedCapabilities method', () => {
      expect(service.getDevelopedCapabilities).toBeDefined();
      expect(typeof service.getDevelopedCapabilities).toBe('function');
    });

    it('should return observable of capabilities array', () =>
      new Promise<void>(done => {
        agentServiceMock.getAttestations.mockReturnValue(['attestation-1', 'attestation-2']);

        service.getDevelopedCapabilities().subscribe({
          next: result => {
            expect(Array.isArray(result)).toBe(true);
            expect(result.length).toBe(2);
            done();
          },
          error: done.fail,
        });
      }));

    it('should transform attestation IDs to capabilities', () =>
      new Promise<void>(done => {
        agentServiceMock.getAttestations.mockReturnValue(['test-attestation']);

        service.getDevelopedCapabilities().subscribe({
          next: result => {
            expect(result[0]).toEqual(
              expect.objectContaining({
                id: 'test-attestation',
                name: expect.any(String),
                description: expect.any(String),
              })
            );
            done();
          },
          error: done.fail,
        });
      }));

    it('should return empty array when no attestations', () =>
      new Promise<void>(done => {
        agentServiceMock.getAttestations.mockReturnValue([]);

        service.getDevelopedCapabilities().subscribe({
          next: result => {
            expect(result).toEqual([]);
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getTimeline', () => {
    it('should have getTimeline method', () => {
      expect(service.getTimeline).toBeDefined();
      expect(typeof service.getTimeline).toBe('function');
    });

    it('should return observable of timeline events array', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getActivityHistory.mockReturnValue([]);

        service.getTimeline().subscribe({
          next: result => {
            expect(Array.isArray(result)).toBe(true);
            done();
          },
          error: done.fail,
        });
      }));

    it('should accept optional limit parameter', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getActivityHistory.mockReturnValue([]);

        service.getTimeline(10).subscribe({
          next: result => {
            expect(result).toBeDefined();
            done();
          },
          error: done.fail,
        });
      }));

    it('should return empty array with no activities', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getActivityHistory.mockReturnValue([]);

        service.getTimeline().subscribe({
          next: result => {
            expect(result).toEqual([]);
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getTopEngagedContent', () => {
    it('should have getTopEngagedContent method', () => {
      expect(service.getTopEngagedContent).toBeDefined();
      expect(typeof service.getTopEngagedContent).toBe('function');
    });

    it('should return observable of engaged content array', () =>
      new Promise<void>(done => {
        service.getTopEngagedContent().subscribe({
          next: result => {
            expect(Array.isArray(result)).toBe(true);
            done();
          },
          error: done.fail,
        });
      }));

    it('should accept optional limit parameter', () =>
      new Promise<void>(done => {
        service.getTopEngagedContent(5).subscribe({
          next: result => {
            expect(Array.isArray(result)).toBe(true);
            done();
          },
          error: done.fail,
        });
      }));

    it('should return empty array when no affinity data', () =>
      new Promise<void>(done => {
        service.getTopEngagedContent().subscribe({
          next: result => {
            expect(result).toEqual([]);
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getAllNotes', () => {
    it('should have getAllNotes method', () => {
      expect(service.getAllNotes).toBeDefined();
      expect(typeof service.getAllNotes).toBe('function');
    });

    it('should return observable of notes array', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

        service.getAllNotes().subscribe({
          next: result => {
            expect(Array.isArray(result)).toBe(true);
            done();
          },
          error: done.fail,
        });
      }));

    it('should return empty array with no path progress', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

        service.getAllNotes().subscribe({
          next: result => {
            expect(result).toEqual([]);
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getResumePoint', () => {
    it('should have getResumePoint method', () => {
      expect(service.getResumePoint).toBeDefined();
      expect(typeof service.getResumePoint).toBe('function');
    });

    it('should return observable of resume point', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

        service.getResumePoint().subscribe({
          next: result => {
            expect(result).toEqual(
              expect.objectContaining({
                type: expect.any(String),
                title: expect.any(String),
              })
            );
            done();
          },
          error: done.fail,
        });
      }));

    it('should suggest exploration when no active paths', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

        service.getResumePoint().subscribe({
          next: result => {
            expect(result?.type).toBe('explore_new');
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('getPathsOverview', () => {
    it('should have getPathsOverview method', () => {
      expect(service.getPathsOverview).toBeDefined();
      expect(typeof service.getPathsOverview).toBe('function');
    });

    it('should return observable of paths overview', () =>
      new Promise<void>(done => {
        sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);
        dataLoaderMock.getPathIndex.mockReturnValue(
          of({ paths: [], lastUpdated: new Date().toISOString(), totalCount: 0 })
        );

        service.getPathsOverview().subscribe({
          next: result => {
            expect(result).toEqual(
              expect.objectContaining({
                inProgress: expect.any(Array),
                completed: expect.any(Array),
                suggested: expect.any(Array),
              })
            );
            done();
          },
          error: done.fail,
        });
      }));
  });

  describe('Observable returns', () => {
    it('getProfile should return observable', () => {
      agentServiceMock.getCurrentAgent.mockReturnValue(of(null));
      agentServiceMock.getAgentProgress.mockReturnValue(of([]));
      agentServiceMock.getAttestations.mockReturnValue([]);
      sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

      const result = service.getProfile();

      expect(result.subscribe).toBeDefined();
    });

    it('getProfileSummary should return observable', () => {
      agentServiceMock.getCurrentAgent.mockReturnValue(of(null));
      agentServiceMock.getAgentProgress.mockReturnValue(of([]));
      agentServiceMock.getAttestations.mockReturnValue([]);
      sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

      const result = service.getProfileSummary();

      expect(result.subscribe).toBeDefined();
    });

    it('getJourneyStats should return observable', () => {
      sessionHumanServiceMock.getSession.mockReturnValue(null);
      agentServiceMock.getAgentProgress.mockReturnValue(of([]));

      const result = service.getJourneyStats();

      expect(result.subscribe).toBeDefined();
    });

    it('getCurrentFocus should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

      const result = service.getCurrentFocus();

      expect(result.subscribe).toBeDefined();
    });

    it('getDevelopedCapabilities should return observable', () => {
      agentServiceMock.getAttestations.mockReturnValue([]);

      const result = service.getDevelopedCapabilities();

      expect(result.subscribe).toBeDefined();
    });

    it('getTimeline should return observable', () => {
      sessionHumanServiceMock.getActivityHistory.mockReturnValue([]);

      const result = service.getTimeline();

      expect(result.subscribe).toBeDefined();
    });

    it('getTopEngagedContent should return observable', () => {
      const result = service.getTopEngagedContent();

      expect(result.subscribe).toBeDefined();
    });

    it('getAllNotes should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

      const result = service.getAllNotes();

      expect(result.subscribe).toBeDefined();
    });

    it('getResumePoint should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);

      const result = service.getResumePoint();

      expect(result.subscribe).toBeDefined();
    });

    it('getPathsOverview should return observable', () => {
      sessionHumanServiceMock.getAllPathProgress.mockReturnValue([]);
      dataLoaderMock.getPathIndex.mockReturnValue(
        of({ paths: [], lastUpdated: new Date().toISOString(), totalCount: 0 })
      );

      const result = service.getPathsOverview();

      expect(result.subscribe).toBeDefined();
    });
  });
});
