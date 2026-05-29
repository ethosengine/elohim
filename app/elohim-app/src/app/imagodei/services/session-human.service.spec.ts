import { vi } from 'vitest';
/**
 * Session Human Service Tests (post M-AGGR-1 slimming)
 *
 * Covers: session lifecycle, profile updates, path progress, content access,
 * upgrade intent, Holochain linking, migration helpers, and dismissal shim.
 *
 * Removed: activity tracking tests (record* methods deleted — M-AGGR-1).
 *          Upgrade prompt trigger tests (triggerUpgradePrompt deleted — M-AGGR-1).
 *          getActivityHistory tests (method deleted — M-AGGR-1).
 */

import { TestBed } from '@angular/core/testing';
import { SessionHumanService } from './session-human.service';
import { SessionHuman, SessionPathProgress } from '../models/session-human.model';

describe('SessionHumanService', () => {
  let service: SessionHumanService;
  let localStorageMock: { [key: string]: string };

  beforeEach(() => {
    localStorageMock = {};

    // Use Storage.prototype to intercept all localStorage calls (works in jsdom/vitest)
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(
      (key: string) => localStorageMock[key] ?? null
    );
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation((key: string, value: string) => {
      localStorageMock[key] = value;
    });
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation((key: string) => {
      delete localStorageMock[key];
    });
    vi.spyOn(Storage.prototype, 'key').mockImplementation((index: number) => {
      return Object.keys(localStorageMock)[index] ?? null;
    });

    // Provide SessionHumanService explicitly so each test gets a fresh instance
    TestBed.configureTestingModule({
      providers: [SessionHumanService],
    });

    service = TestBed.inject(SessionHumanService);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('initialization', () => {
    it('should create a new session if none exists', () => {
      expect(service.hasSession()).toBe(true);
      expect(service.getSession()).not.toBeNull();
    });

    it('should generate a unique session ID', () => {
      const session = service.getSession();
      expect(session?.sessionId).toMatch(/^session-[a-z0-9]+-[a-z0-9]+$/);
    });

    it('should set default display name', () => {
      const session = service.getSession();
      expect(session?.displayName).toBe('Traveler');
    });

    it('should initialize stats to zero', () => {
      const session = service.getSession();
      expect(session?.stats.nodesViewed).toBe(0);
      expect(session?.stats.pathsStarted).toBe(0);
      expect(session?.stats.pathsCompleted).toBe(0);
    });

    it('should set session as anonymous', () => {
      const session = service.getSession();
      expect(session?.isAnonymous).toBe(true);
      expect(session?.accessLevel).toBe('visitor');
    });
  });

  describe('session restoration', () => {
    it('should restore existing session from localStorage', () => {
      const existingSession: SessionHuman = {
        sessionId: 'session-existing-123',
        displayName: 'Restored User',
        createdAt: new Date().toISOString(),
        lastActiveAt: new Date().toISOString(),
        stats: {
          nodesViewed: 10,
          nodesWithAffinity: 5,
          pathsStarted: 2,
          pathsCompleted: 1,
          stepsCompleted: 15,
          totalSessionTime: 3600,
          averageSessionLength: 1800,
          sessionCount: 3,
        },
        isAnonymous: true,
        accessLevel: 'visitor',
        sessionState: 'active',
      };

      localStorageMock['lamad-session'] = JSON.stringify(existingSession);

      // Create new instance to trigger initialization
      const newService = new SessionHumanService();
      const restored = newService.getSession();

      expect(restored?.sessionId).toBe('session-existing-123');
      expect(restored?.displayName).toBe('Restored User');
      expect(restored?.stats.nodesViewed).toBe(10);
      expect(restored?.stats.sessionCount).toBe(3); // M-AGGR-1: sessionCount is no longer incremented on restore; stat is substrate-derived
    });
  });

  describe('display name', () => {
    it('should update display name', () => {
      service.setDisplayName('New Name');
      expect(service.getSession()?.displayName).toBe('New Name');
    });

    it('should trim display name', () => {
      service.setDisplayName('  Trimmed  ');
      expect(service.getSession()?.displayName).toBe('Trimmed');
    });

    it('should default to Traveler for empty name', () => {
      service.setDisplayName('');
      expect(service.getSession()?.displayName).toBe('Traveler');
    });
  });

  describe('profile updates', () => {
    it('should set avatar URL', () => {
      service.setAvatarUrl('https://example.com/avatar.png');
      expect(service.getSession()?.avatarUrl).toBe('https://example.com/avatar.png');
    });

    it('should set bio', () => {
      service.setBio('Test bio');
      expect(service.getSession()?.bio).toBe('Test bio');
    });

    it('should set locale', () => {
      service.setLocale('en-US');
      expect(service.getSession()?.locale).toBe('en-US');
    });

    it('should set interests', () => {
      service.setInterests(['tech', 'science', 'philosophy']);
      expect(service.getSession()?.interests).toEqual(['tech', 'science', 'philosophy']);
    });

    it('should filter empty interests', () => {
      service.setInterests(['valid', '', '  ', 'also-valid']);
      expect(service.getSession()?.interests).toEqual(['valid', 'also-valid']);
    });
  });

  describe('path progress', () => {
    it('should save and retrieve path progress', () => {
      const progress: SessionPathProgress = {
        pathId: 'path-123',
        currentStepIndex: 2,
        completedStepIndices: [0, 1],
        stepAffinity: {},
        stepNotes: {},
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
      };

      service.savePathProgress(progress);

      const retrieved = service.getPathProgress('path-123');
      expect(retrieved?.pathId).toBe('path-123');
      expect(retrieved?.currentStepIndex).toBe(2);
      expect(retrieved?.completedStepIndices).toEqual([0, 1]);
    });

    it('should return null for unknown path', () => {
      expect(service.getPathProgress('unknown-path')).toBeNull();
    });

    it('should get all path progress', () => {
      service.savePathProgress({
        pathId: 'path-1',
        currentStepIndex: 0,
        completedStepIndices: [],
        stepAffinity: {},
        stepNotes: {},
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
      });

      service.savePathProgress({
        pathId: 'path-2',
        currentStepIndex: 1,
        completedStepIndices: [0],
        stepAffinity: {},
        stepNotes: {},
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
      });

      const progress1 = service.getPathProgress('path-1');
      const progress2 = service.getPathProgress('path-2');
      expect(progress1).not.toBeNull();
      expect(progress2).not.toBeNull();
      expect(progress1?.pathId).toBe('path-1');
      expect(progress2?.pathId).toBe('path-2');
    });
  });

  describe('upgrade prompts (M-AGGR-1 substrate-driven)', () => {
    it('should not expose getActiveUpgradePrompts (method removed, prompts are substrate-derived)', () => {
      // M-AGGR-1: getActiveUpgradePrompts() was deleted; active prompts come from
      // GET /api/v1/identity/{agentId}/upgrade-prompts (UpgradePromptView).
      // The service retains only the dismissal shim so the UI can suppress already-seen prompts.
      expect((service as any).getActiveUpgradePrompts).toBeUndefined();
    });

    it('should record dismissal in localStorage', () => {
      service.dismissUpgradePrompt('prompt-first-affinity');
      const dismissed = service.getDismissedPromptIds();
      expect(dismissed).toContain('prompt-first-affinity');
    });

    it('should not duplicate dismissed ids', () => {
      service.dismissUpgradePrompt('prompt-1');
      service.dismissUpgradePrompt('prompt-1');
      const dismissed = service.getDismissedPromptIds();
      expect(dismissed.filter(id => id === 'prompt-1').length).toBe(1);
    });
  });

  describe('Holochain linking', () => {
    it('should link to Holochain identity', () => {
      service.linkToHolochainIdentity('agent-pub-key-123', 'human-id-456');

      const session = service.getSession();
      expect(session?.linkedAgentPubKey).toBe('agent-pub-key-123');
      expect(session?.linkedHumanId).toBe('human-id-456');
      expect(session?.sessionState).toBe('linked');
      expect(session?.isAnonymous).toBe(false);
    });

    it('should check if linked to Holochain', () => {
      expect(service.isLinkedToHolochain()).toBe(false);

      service.linkToHolochainIdentity('agent-123', 'human-456');

      expect(service.isLinkedToHolochain()).toBe(true);
    });

    it('should get linked agent pubkey', () => {
      expect(service.getLinkedAgentPubKey()).toBeNull();

      service.linkToHolochainIdentity('agent-123', 'human-456');

      expect(service.getLinkedAgentPubKey()).toBe('agent-123');
    });
  });

  describe('upgrade intent', () => {
    it('should start upgrade intent', () => {
      service.startUpgradeIntent('hosted');

      const intent = service.getUpgradeIntent();
      expect(intent).not.toBeNull();
      expect(intent?.targetStage).toBe('hosted');
      expect(intent?.currentStep).toBe('initiated');
      expect(service.isUpgrading()).toBe(true);
    });

    it('should update upgrade progress', () => {
      service.startUpgradeIntent('app-steward');
      service.updateUpgradeProgress('verify-email', 'initiated');

      const intent = service.getUpgradeIntent();
      expect(intent?.currentStep).toBe('verify-email');
      expect(intent?.completedSteps).toContain('initiated');
    });

    it('should pause upgrade', () => {
      service.startUpgradeIntent('hosted');
      expect(service.isUpgrading()).toBe(true);

      service.pauseUpgrade('user-cancelled');

      expect(service.isUpgrading()).toBe(false);
      expect(service.getUpgradeIntent()?.paused).toBe(true);
    });

    it('should resume upgrade', () => {
      service.startUpgradeIntent('hosted');
      service.pauseUpgrade();
      expect(service.isUpgrading()).toBe(false);

      service.resumeUpgrade();

      expect(service.isUpgrading()).toBe(true);
    });

    it('should cancel upgrade', () => {
      service.startUpgradeIntent('hosted');
      service.cancelUpgrade();

      expect(service.getUpgradeIntent()).toBeNull();
      expect(service.getSessionState()).toBe('active');
    });
  });

  describe('migration', () => {
    it('should prepare migration package with empty activities (M-AGGR-1)', () => {
      const migration = service.prepareMigration();

      expect(migration).not.toBeNull();
      expect(migration?.sessionId).toBe(service.getSessionId());
      expect(migration?.status).toBe('pending');
      // M-AGGR-1: activities are substrate-derived; always empty in migration package
      expect(migration?.activities).toEqual([]);
    });

    it('should mark as migrated', () => {
      service.markAsMigrated('agent-123', 'human-456');

      const session = service.getSession();
      expect(session?.sessionState).toBe('migrated');
      expect(session?.linkedAgentPubKey).toBe('agent-123');
    });

    it('should clear after migration', () => {
      service.clearAfterMigration();

      expect(service.getSession()).toBeNull();
    });
  });

  describe('content access', () => {
    it('should allow access to open content', () => {
      const result = service.checkContentAccess({ accessLevel: 'open' });
      expect(result.canAccess).toBe(true);
    });

    it('should allow access when no metadata', () => {
      const result = service.checkContentAccess(undefined);
      expect(result.canAccess).toBe(true);
    });

    it('should deny access to gated content', () => {
      const result = service.checkContentAccess({
        accessLevel: 'gated',
        restrictionReason: 'Members only',
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
          requiredPaths: ['training-path'],
          requiredAttestations: ['certified'],
        },
      });

      expect(result.canAccess).toBe(false);
      expect(result.missingPaths).toContain('training-path');
      expect(result.missingAttestations).toContain('certified');
    });

    it('should return visitor access level', () => {
      expect(service.getAccessLevel()).toBe('visitor');
    });
  });

  describe('storage key helpers', () => {
    it('should return session-scoped storage key prefix', () => {
      const prefix = service.getStorageKeyPrefix();
      expect(prefix).toContain('lamad-session-');
      expect(prefix).toContain(service.getSessionId());
    });

    it('should return affinity storage key', () => {
      const key = service.getAffinityStorageKey();
      expect(key).toContain('affinity');
      expect(key).toContain(service.getSessionId());
    });
  });

  describe('session observable', () => {
    it('should emit session changes', () =>
      new Promise<void>(done => {
        const emissions: (SessionHuman | null)[] = [];

        service.session$.subscribe(session => {
          emissions.push(session);
          if (emissions.length === 2) {
            expect(emissions[1]?.displayName).toBe('Updated Name');
            done();
          }
        });

        service.setDisplayName('Updated Name');
      }));
  });

  describe('touch', () => {
    it('should update lastActiveAt', () => {
      const before = service.getSession()?.lastActiveAt;

      setTimeout(() => {
        service.touch();
        const after = service.getSession()?.lastActiveAt;
        expect(after).not.toBe(before);
      }, 10);
    });
  });

  describe('resetSession', () => {
    it('should create fresh session', () => {
      service.setDisplayName('Custom Name');
      const oldId = service.getSessionId();

      service.resetSession();

      expect(service.getSessionId()).not.toBe(oldId);
      expect(service.getSession()?.displayName).toBe('Traveler');
    });
  });
});
