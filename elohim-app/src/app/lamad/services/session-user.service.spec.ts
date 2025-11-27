import { TestBed } from '@angular/core/testing';
import { SessionUserService } from './session-user.service';

describe('SessionUserService', () => {
  let service: SessionUserService;
  let localStorageMock: { [key: string]: string } = {};

  beforeEach(() => {
    localStorageMock = {};
    spyOn(localStorage, 'getItem').and.callFake((key: string) => {
      return localStorageMock[key] || null;
    });
    spyOn(localStorage, 'setItem').and.callFake((key: string, value: string) => {
      localStorageMock[key] = value;
    });
    spyOn(localStorage, 'removeItem').and.callFake((key: string) => {
      delete localStorageMock[key];
    });

    TestBed.configureTestingModule({
      providers: [SessionUserService]
    });
    service = TestBed.inject(SessionUserService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should create a new session on first load', () => {
    const session = service.getCurrentSession();
    expect(session).toBeTruthy();
    expect(session?.sessionId).toBeDefined();
    expect(session?.displayName).toBe('Traveler');
    expect(session?.isAnonymous).toBe(true);
  });

  it('should return session ID', () => {
    const sessionId = service.getSessionId();
    expect(sessionId).toBeTruthy();
    expect(typeof sessionId).toBe('string');
  });

  it('should emit session via observable', (done) => {
    service.session$.subscribe(session => {
      if (session) {
        expect(session.sessionId).toBeDefined();
        done();
      }
    });
  });

  it('should update display name', () => {
    service.updateDisplayName('TestUser');
    const session = service.getCurrentSession();
    expect(session?.displayName).toBe('TestUser');
  });

  it('should record content views', () => {
    service.recordContentView('node-1');
    const session = service.getCurrentSession();
    expect(session?.stats.contentViews).toBeGreaterThan(0);
  });

  it('should record path starts', () => {
    service.recordPathStart('path-1');
    const session = service.getCurrentSession();
    expect(session?.stats.pathsStarted).toBeGreaterThan(0);
  });

  it('should record path completions', () => {
    service.recordPathCompletion('path-1');
    const session = service.getCurrentSession();
    expect(session?.stats.pathsCompleted).toBeGreaterThan(0);
  });

  it('should record affinity changes', () => {
    service.recordAffinityChange('node-1', 0.5);
    const session = service.getCurrentSession();
    expect(session?.stats.affinityUpdates).toBeGreaterThan(0);
  });

  it('should get affinity storage key', () => {
    const key = service.getAffinityStorageKey();
    expect(key).toContain('affinity');
  });

  it('should persist session to localStorage', () => {
    service.updateDisplayName('PersistTest');
    expect(localStorage.setItem).toHaveBeenCalled();
  });

  describe('Path Progress', () => {
    it('should get path progress for new path', () => {
      const progress = service.getPathProgress('new-path');
      expect(progress).toBeDefined();
      expect(progress.pathId).toBe('new-path');
      expect(progress.currentStepIndex).toBe(0);
    });

    it('should update path progress', () => {
      service.updatePathProgress('path-1', 3);
      const progress = service.getPathProgress('path-1');
      expect(progress.currentStepIndex).toBe(3);
    });

    it('should get all path progress', () => {
      service.updatePathProgress('path-1', 1);
      service.updatePathProgress('path-2', 2);
      const allProgress = service.getAllPathProgress();
      expect(allProgress.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Activities', () => {
    it('should get recent activities', () => {
      service.recordContentView('node-1');
      const activities = service.getRecentActivities(10);
      expect(activities.length).toBeGreaterThan(0);
    });

    it('should limit activities returned', () => {
      for (let i = 0; i < 20; i++) {
        service.recordContentView(`node-${i}`);
      }
      const activities = service.getRecentActivities(5);
      expect(activities.length).toBeLessThanOrEqual(5);
    });
  });

  describe('Holochain Upgrade Prompts', () => {
    it('should emit upgrade prompts observable', (done) => {
      service.upgradePrompts$.subscribe(prompts => {
        expect(Array.isArray(prompts)).toBe(true);
        done();
      });
    });
  });

  describe('Session Stats', () => {
    it('should track session count', () => {
      const session = service.getCurrentSession();
      expect(session?.stats.sessionCount).toBeGreaterThanOrEqual(1);
    });

    it('should track total time', () => {
      const session = service.getCurrentSession();
      expect(session?.stats.totalTimeMinutes).toBeDefined();
    });
  });
});
