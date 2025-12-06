import { TestBed } from '@angular/core/testing';
import { SessionHumanService } from './session-human.service';
import {
  SessionHuman,
  SessionActivity,
  SessionPathProgress,
  HolochainUpgradePrompt,
} from '../models';

describe('SessionHumanService', () => {
  let service: SessionHumanService;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;

  beforeEach(() => {
    // Mock localStorage
    localStorageMock = {};

    // Create a complete Storage mock
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => { localStorageMock[key] = value; },
      removeItem: (key: string) => { delete localStorageMock[key]; },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() { return Object.keys(localStorageMock).length; },
      clear: () => { localStorageMock = {}; }
    };

    // Replace global localStorage with our mock
    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockStorage);

    TestBed.configureTestingModule({});
    service = TestBed.inject(SessionHumanService);
  });

  afterEach(() => {
    localStorageMock = {};
  });

  describe('Session Lifecycle', () => {
    it('should create the service', () => {
      expect(service).toBeTruthy();
    });

    it('should initialize a new session if none exists', () => {
      const session = service.getSession();
      expect(session).toBeTruthy();
      expect(session?.sessionId).toMatch(/^session-/);
      expect(session?.displayName).toBe('Traveler');
      expect(session?.isAnonymous).toBe(true);
      expect(session?.accessLevel).toBe('visitor');
      expect(session?.stats.sessionCount).toBe(1);
    });

    it('should restore existing session', () => {
      const existingSession: SessionHuman = {
        sessionId: 'session-test-123',
        displayName: 'Test User',
        isAnonymous: true,
        accessLevel: 'visitor',
        createdAt: '2025-01-01T00:00:00.000Z',
        lastActiveAt: '2025-01-01T00:00:00.000Z',
        stats: {
          nodesViewed: 5,
          nodesWithAffinity: 3,
          pathsStarted: 1,
          pathsCompleted: 0,
          stepsCompleted: 10,
          totalSessionTime: 3600,
          averageSessionLength: 1800,
          sessionCount: 2,
        },
      };

      localStorageMock['lamad-session'] = JSON.stringify(existingSession);

      // Recreate service to trigger initialization
      service = new SessionHumanService();

      const session = service.getSession();
      expect(session?.sessionId).toBe('session-test-123');
      expect(session?.displayName).toBe('Test User');
      expect(session?.stats.sessionCount).toBe(3); // Incremented
    });

    it('should check if session exists', () => {
      expect(service.hasSession()).toBe(true);
    });

    it('should return session ID', () => {
      const sessionId = service.getSessionId();
      expect(sessionId).toMatch(/^session-/);
    });

    it('should update display name', () => {
      service.setDisplayName('New Name');
      const session = service.getSession();
      expect(session?.displayName).toBe('New Name');
    });

    it('should trim display name', () => {
      service.setDisplayName('  Spaced Name  ');
      const session = service.getSession();
      expect(session?.displayName).toBe('Spaced Name');
    });

    it('should default to Traveler if empty name provided', () => {
      service.setDisplayName('');
      const session = service.getSession();
      expect(session?.displayName).toBe('Traveler');
    });

    it('should update last active timestamp with touch', () => {
      const session = service.getSession();
      const oldTimestamp = session?.lastActiveAt;

      setTimeout(() => {
        service.touch();
        const updatedSession = service.getSession();
        expect(updatedSession?.lastActiveAt).not.toBe(oldTimestamp);
      }, 10);
    });
  });

  describe('Profile Management', () => {
    it('should set avatar URL', () => {
      service.setAvatarUrl('https://example.com/avatar.jpg');
      const session = service.getSession();
      expect(session?.avatarUrl).toBe('https://example.com/avatar.jpg');
    });

    it('should trim avatar URL', () => {
      service.setAvatarUrl('  https://example.com/avatar.jpg  ');
      const session = service.getSession();
      expect(session?.avatarUrl).toBe('https://example.com/avatar.jpg');
    });

    it('should clear avatar URL when empty', () => {
      service.setAvatarUrl('https://example.com/avatar.jpg');
      service.setAvatarUrl('');
      const session = service.getSession();
      expect(session?.avatarUrl).toBeUndefined();
    });

    it('should set bio', () => {
      service.setBio('A curious learner');
      const session = service.getSession();
      expect(session?.bio).toBe('A curious learner');
    });

    it('should trim bio', () => {
      service.setBio('  Spaced bio  ');
      const session = service.getSession();
      expect(session?.bio).toBe('Spaced bio');
    });

    it('should clear bio when empty', () => {
      service.setBio('Some bio');
      service.setBio('');
      const session = service.getSession();
      expect(session?.bio).toBeUndefined();
    });

    it('should set locale', () => {
      service.setLocale('es_ES');
      const session = service.getSession();
      expect(session?.locale).toBe('es_ES');
    });

    it('should trim locale', () => {
      service.setLocale('  en_US  ');
      const session = service.getSession();
      expect(session?.locale).toBe('en_US');
    });

    it('should clear locale when empty', () => {
      service.setLocale('en_US');
      service.setLocale('');
      const session = service.getSession();
      expect(session?.locale).toBeUndefined();
    });

    it('should set interests', () => {
      service.setInterests(['faith', 'technology', 'ethics']);
      const session = service.getSession();
      expect(session?.interests).toEqual(['faith', 'technology', 'ethics']);
    });

    it('should trim interests', () => {
      service.setInterests(['  faith  ', '  tech  ']);
      const session = service.getSession();
      expect(session?.interests).toEqual(['faith', 'tech']);
    });

    it('should filter empty interests', () => {
      service.setInterests(['faith', '', '  ', 'ethics']);
      const session = service.getSession();
      expect(session?.interests).toEqual(['faith', 'ethics']);
    });

    it('should clear interests when all empty', () => {
      service.setInterests(['faith']);
      service.setInterests(['', '  ']);
      const session = service.getSession();
      expect(session?.interests).toBeUndefined();
    });

    it('should get storage key prefix', () => {
      const prefix = service.getStorageKeyPrefix();
      expect(prefix).toContain('lamad-session-');
      expect(prefix).toContain(service.getSessionId());
    });
  });

  describe('Activity Tracking', () => {
    it('should record content view', () => {
      service.recordContentView('node-1');
      const session = service.getSession();
      expect(session?.stats.nodesViewed).toBe(1);

      const activities = service.getActivityHistory();
      expect(activities.length).toBe(1);
      expect(activities[0].type).toBe('view');
      expect(activities[0].resourceId).toBe('node-1');
    });

    it('should record affinity change', () => {
      service.recordAffinityChange('node-1', 0.5);
      const session = service.getSession();
      expect(session?.stats.nodesWithAffinity).toBe(1);

      const activities = service.getActivityHistory();
      expect(activities.length).toBe(1);
      expect(activities[0].type).toBe('affinity');
      expect(activities[0].metadata?.['value']).toBe(0.5);
    });

    it('should record path started', () => {
      service.recordPathStarted('path-1');
      const session = service.getSession();
      expect(session?.stats.pathsStarted).toBe(1);

      const activities = service.getActivityHistory();
      expect(activities.length).toBe(1);
      expect(activities[0].type).toBe('path-start');
    });

    it('should record step completed', () => {
      service.recordStepCompleted('path-1', 2);
      const session = service.getSession();
      expect(session?.stats.stepsCompleted).toBe(1);

      const activities = service.getActivityHistory();
      expect(activities[0].metadata?.['stepIndex']).toBe(2);
    });

    it('should record path completed', () => {
      service.recordPathCompleted('path-1');
      const session = service.getSession();
      expect(session?.stats.pathsCompleted).toBe(1);
    });

    it('should record exploration', () => {
      service.recordExploration('node-1');
      const activities = service.getActivityHistory();
      expect(activities[0].type).toBe('explore');
    });

    it('should limit activity history to ACTIVITY_LIMIT', () => {
      // Record more than limit
      for (let i = 0; i < 1100; i++) {
        service.recordContentView(`node-${i}`);
      }

      const activities = service.getActivityHistory();
      expect(activities.length).toBe(1000);
    });

    it('should return empty array if no activities', () => {
      service.resetSession();
      const activities = service.getActivityHistory();
      expect(activities).toEqual([]);
    });
  });

  describe('Path Progress', () => {
    it('should save and retrieve path progress', () => {
      const progress: SessionPathProgress = {
        pathId: 'path-1',
        currentStepIndex: 2,
        completedStepIndices: [0, 1],
        stepAffinity: {},
        stepNotes: {},
        startedAt: '2025-01-01T00:00:00.000Z',
        lastActivityAt: '2025-01-02T00:00:00.000Z'
      };

      service.savePathProgress(progress);
      const retrieved = service.getPathProgress('path-1');

      expect(retrieved).toBeTruthy();
      expect(retrieved?.pathId).toBe('path-1');
      expect(retrieved?.currentStepIndex).toBe(2);
      expect(retrieved?.completedStepIndices).toEqual([0, 1]);
    });

    it('should return null if no progress exists', () => {
      const progress = service.getPathProgress('non-existent');
      expect(progress).toBeNull();
    });

    it('should get all path progress records', () => {
      const progress1: SessionPathProgress = {
        pathId: 'path-1',
        currentStepIndex: 2,
        completedStepIndices: [0, 1],
        stepAffinity: {},
        stepNotes: {},
        startedAt: '2025-01-01T00:00:00.000Z',
        lastActivityAt: '2025-01-02T00:00:00.000Z'
      };

      const progress2: SessionPathProgress = {
        pathId: 'path-2',
        currentStepIndex: 1,
        completedStepIndices: [0],
        stepAffinity: {},
        stepNotes: {},
        startedAt: '2025-01-01T00:00:00.000Z',
        lastActivityAt: '2025-01-02T00:00:00.000Z'
      };

      service.savePathProgress(progress1);
      service.savePathProgress(progress2);

      const allProgress = service.getAllPathProgress();
      expect(allProgress.length).toBe(2);
    });
  });

  describe('Affinity Storage Key', () => {
    it('should return affinity storage key for session', () => {
      const key = service.getAffinityStorageKey();
      expect(key).toContain('lamad-session-');
      expect(key).toContain('-affinity');
    });
  });

  describe('Upgrade Prompts', () => {
    it('should trigger upgrade prompt on first affinity', () => {
      service.recordAffinityChange('node-1', 0.5);
      const prompts = service.getActiveUpgradePrompts();
      expect(prompts.length).toBeGreaterThan(0);
      expect(prompts[0].trigger).toBe('first-affinity');
    });

    it('should trigger upgrade prompt on path started', () => {
      service.recordPathStarted('path-1');
      const prompts = service.getActiveUpgradePrompts();
      const pathPrompt = prompts.find(p => p.trigger === 'path-started');
      expect(pathPrompt).toBeTruthy();
    });

    it('should trigger upgrade prompt on path completed', () => {
      service.recordPathCompleted('path-1');
      const prompts = service.getActiveUpgradePrompts();
      const completePrompt = prompts.find(p => p.trigger === 'path-completed');
      expect(completePrompt).toBeTruthy();
    });

    it('should trigger upgrade prompt on notes saved', () => {
      service.recordNotesSaved('path-1', 1);
      const prompts = service.getActiveUpgradePrompts();
      const notesPrompt = prompts.find(p => p.trigger === 'notes-saved');
      expect(notesPrompt).toBeTruthy();
    });

    it('should dismiss upgrade prompt', () => {
      service.recordAffinityChange('node-1', 0.5);
      const prompts = service.getActiveUpgradePrompts();
      const promptId = prompts[0].id;

      service.dismissUpgradePrompt(promptId);
      const activePrompts = service.getActiveUpgradePrompts();
      expect(activePrompts.length).toBe(0);
    });

    it('should not show dismissed prompts again', () => {
      service.recordAffinityChange('node-1', 0.5);
      const prompts = service.getActiveUpgradePrompts();
      const promptId = prompts[0].id;

      service.dismissUpgradePrompt(promptId);
      service.triggerUpgradePrompt('first-affinity');

      const activePrompts = service.getActiveUpgradePrompts();
      expect(activePrompts.length).toBe(0);
    });
  });

  describe('Migration', () => {
    it('should prepare migration package', () => {
      service.recordContentView('node-1');
      service.recordAffinityChange('node-1', 0.5);

      const progress: SessionPathProgress = {
        pathId: 'path-1',
        currentStepIndex: 2,
        completedStepIndices: [0, 1],
        stepAffinity: {},
        stepNotes: {},
        startedAt: '2025-01-01T00:00:00.000Z',
        lastActivityAt: '2025-01-02T00:00:00.000Z'
      };
      service.savePathProgress(progress);

      const migration = service.prepareMigration();
      expect(migration).toBeTruthy();
      expect(migration?.sessionId).toBeTruthy();
      expect(migration?.status).toBe('pending');
      expect(migration?.activities.length).toBeGreaterThan(0);
      expect(migration?.pathProgress.length).toBe(1);
    });

    it('should clear session after migration', () => {
      service.clearAfterMigration();
      const session = service.getSession();
      expect(session).toBeNull();
    });
  });

  describe('Content Access Control', () => {
    it('should always return visitor access level', () => {
      const level = service.getAccessLevel();
      expect(level).toBe('visitor');
    });

    it('should allow access to open content', () => {
      const result = service.checkContentAccess({ accessLevel: 'open' });
      expect(result.canAccess).toBe(true);
    });

    it('should allow access to undefined access metadata', () => {
      const result = service.checkContentAccess();
      expect(result.canAccess).toBe(true);
    });

    it('should deny access to gated content', () => {
      const result = service.checkContentAccess({
        accessLevel: 'gated',
        restrictionReason: 'Test restriction',
        requirements: {
          minLevel: 'member'
        }
      });
      expect(result.canAccess).toBe(false);
      expect(result.reason).toBe('not-authenticated');
      expect(result.actionRequired?.type).toBe('install-holochain');
    });

    it('should deny access to protected content', () => {
      const result = service.checkContentAccess({
        accessLevel: 'protected',
        requirements: {
          minLevel: 'attested',
          requiredAttestations: ['att-1'],
          requiredPaths: ['path-1']
        }
      });
      expect(result.canAccess).toBe(false);
      expect(result.missingAttestations).toEqual(['att-1']);
      expect(result.missingPaths).toEqual(['path-1']);
    });

    it('should check if content is accessible', () => {
      expect(service.canAccessContent({ accessLevel: 'open' })).toBe(true);
      expect(service.canAccessContent({
        accessLevel: 'gated',
        requirements: { minLevel: 'member' }
      })).toBe(false);
    });

    it('should trigger upgrade prompt on gated content access', () => {
      service.onGatedContentAccess('content-1', 'Test Content');
      const prompts = service.getActiveUpgradePrompts();
      const networkPrompt = prompts.find(p => p.trigger === 'network-feature');
      expect(networkPrompt).toBeTruthy();
    });
  });

  describe('Session Reset', () => {
    it('should reset session', () => {
      service.recordContentView('node-1');
      service.resetSession();

      const session = service.getSession();
      expect(session?.stats.nodesViewed).toBe(0);
      expect(service.getActiveUpgradePrompts().length).toBe(0);
    });
  });
});
