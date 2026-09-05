/**
 * SessionMigrationService Tests
 *
 * The visitor→hosted upgrade no longer registers anyone: the account is created
 * at the doorway's own portal, and this service runs AFTER that, from the OAuth
 * callback. So these tests assert what a visitor session contributes to an
 * already-authenticated human — profile fields the portal could not know,
 * learning progress, and the retirement of the session itself.
 */

import { TestBed } from '@angular/core/testing';

import { ContentMasteryService } from '@app/lamad/services/content-mastery.service';

import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import type { SessionHuman } from '../models/session-human.model';

import { SessionMigrationService } from './session-migration.service';
import { IdentityService } from './identity.service';
import { SessionHumanService } from './session-human.service';

import { vi } from 'vitest';

describe('SessionMigrationService', () => {
  let service: SessionMigrationService;
  let mockHolochainClient: {
    callZome: ReturnType<typeof vi.fn>;
    isConnected: ReturnType<typeof vi.fn>;
  };
  let mockSessionHumanService: {
    getSession: ReturnType<typeof vi.fn>;
    prepareMigration: ReturnType<typeof vi.fn>;
    clearAfterMigration: ReturnType<typeof vi.fn>;
    markAsMigrated: ReturnType<typeof vi.fn>;
    hasSession: ReturnType<typeof vi.fn>;
  };
  let mockIdentityService: {
    humanId: ReturnType<typeof vi.fn>;
    agentPubKey: ReturnType<typeof vi.fn>;
    displayName: ReturnType<typeof vi.fn>;
    profile: ReturnType<typeof vi.fn>;
    updateProfile: ReturnType<typeof vi.fn>;
    mode: ReturnType<typeof vi.fn>;
  };
  let mockContentMasteryService: { migrateToBackend: ReturnType<typeof vi.fn> };

  const mockSession: SessionHuman = {
    sessionId: 'session-123',
    displayName: 'Test User',
    bio: 'A test bio',
    interests: ['learning', 'teaching'],
    createdAt: '2026-01-01T00:00:00Z',
    lastActiveAt: '2026-01-01T00:00:00Z',
    stats: {
      nodesViewed: 10,
      nodesWithAffinity: 5,
      pathsStarted: 2,
      pathsCompleted: 1,
      stepsCompleted: 15,
      totalSessionTime: 3600,
      averageSessionLength: 1200,
      sessionCount: 3,
    },
    accessLevel: 'visitor',
    isAnonymous: false,
    sessionState: 'active',
    linkedAgentPubKey: undefined,
    linkedHumanId: undefined,
  };

  const mockMigrationPackage = {
    sessionId: 'session-123',
    affinity: { 'node-1': 0.8, 'node-2': 0.6 },
    pathProgress: [
      {
        pathId: 'path-1',
        currentStepIndex: 3,
        completedStepIndices: [0, 1, 2],
        startedAt: '2026-01-01T00:00:00Z',
        lastActivityAt: '2026-01-01T12:00:00Z',
      },
    ],
    activities: [],
    migratedAt: '2026-01-01T00:00:00Z',
    status: 'pending' as const,
  };

  beforeEach(() => {
    mockHolochainClient = {
      callZome: vi.fn().mockResolvedValue({}),
      isConnected: vi.fn().mockReturnValue(true),
    };

    mockSessionHumanService = {
      getSession: vi.fn().mockReturnValue(mockSession),
      prepareMigration: vi.fn().mockReturnValue(mockMigrationPackage),
      clearAfterMigration: vi.fn(),
      markAsMigrated: vi.fn(),
      hasSession: vi.fn().mockReturnValue(true),
    };

    mockIdentityService = {
      humanId: vi.fn().mockReturnValue('human-123'),
      agentPubKey: vi.fn().mockReturnValue('agent-pub-key'),
      // The doorway defaulted the display name out of the identifier local part.
      displayName: vi.fn().mockReturnValue('matthew'),
      profile: vi.fn().mockReturnValue({ bio: null, affinities: [] }),
      updateProfile: vi.fn().mockResolvedValue({}),
      mode: vi.fn().mockReturnValue('hosted'),
    };

    mockContentMasteryService = {
      migrateToBackend: vi.fn().mockResolvedValue({ success: true, migrated: 4 }),
    };

    TestBed.configureTestingModule({
      providers: [
        SessionMigrationService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: ContentMasteryService, useValue: mockContentMasteryService },
      ],
    });

    service = TestBed.inject(SessionMigrationService);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ==========================================================================
  // canMigrate — no conductor socket in the predicate
  // ==========================================================================

  describe('canMigrate', () => {
    it('is true whenever there is a visitor session', () => {
      expect(service.canMigrate()).toBe(true);
    });

    it('does NOT require a conductor socket — an anonymous visitor cannot open one', () => {
      mockHolochainClient.isConnected.mockReturnValue(false);
      expect(service.canMigrate()).toBe(true);
    });

    it('is false with no session', () => {
      mockSessionHumanService.hasSession.mockReturnValue(false);
      expect(service.canMigrate()).toBe(false);
    });
  });

  // ==========================================================================
  // applySessionToProfile — the upgrade, after the portal
  // ==========================================================================

  describe('applySessionToProfile', () => {
    it('registers nobody — there is no registration path left on this service', () => {
      expect((service as unknown as Record<string, unknown>)['migrate']).toBeUndefined();
      expect(mockIdentityService).not.toHaveProperty('registerHuman');
    });

    it('applies the session display name when the doorway defaulted it from the identifier', async () => {
      await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(mockIdentityService.updateProfile).toHaveBeenCalledWith(
        expect.objectContaining({ displayName: 'Test User' })
      );
    });

    it('keeps a name the human actually chose at the portal', async () => {
      mockIdentityService.displayName.mockReturnValue('Matthew of Alpha');

      await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(mockIdentityService.updateProfile).toHaveBeenCalledWith(
        expect.not.objectContaining({ displayName: expect.anything() })
      );
    });

    it('carries the bio and interests onto an empty profile', async () => {
      await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(mockIdentityService.updateProfile).toHaveBeenCalledWith(
        expect.objectContaining({
          bio: 'A test bio',
          affinities: ['learning', 'teaching'],
        })
      );
    });

    it('never overwrites a bio or affinities the profile already holds', async () => {
      mockIdentityService.profile.mockReturnValue({
        bio: 'Written at the portal',
        affinities: ['governance'],
      });
      mockIdentityService.displayName.mockReturnValue('Matthew of Alpha');

      await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(mockIdentityService.updateProfile).not.toHaveBeenCalled();
    });

    it('transfers path progress and content mastery', async () => {
      const result = await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(mockHolochainClient.callZome).toHaveBeenCalled();
      expect(mockContentMasteryService.migrateToBackend).toHaveBeenCalled();
      expect(result.migratedData).toEqual({
        affinityCount: 2,
        pathProgressCount: 1,
        activityCount: 0,
        masteryCount: 4,
      });
    });

    it('links the session to the new identity, then clears it', async () => {
      await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(mockSessionHumanService.markAsMigrated).toHaveBeenCalledWith(
        'agent-pub-key',
        'human-123'
      );
      expect(mockSessionHumanService.clearAfterMigration).toHaveBeenCalled();
    });

    it('reports success with the authenticated human id', async () => {
      const result = await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(result.success).toBe(true);
      expect(result.newHumanId).toBe('human-123');
      expect(service.status()).toBe('completed');
    });

    it('refuses when nobody is signed in yet', async () => {
      mockIdentityService.humanId.mockReturnValue(null);

      const result = await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Not signed in');
      expect(mockSessionHumanService.clearAfterMigration).not.toHaveBeenCalled();
    });

    it('refuses when there is no visitor session', async () => {
      mockSessionHumanService.hasSession.mockReturnValue(false);

      const result = await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(result.success).toBe(false);
      expect(mockIdentityService.updateProfile).not.toHaveBeenCalled();
    });

    it('surfaces a profile-update failure without clearing the session', async () => {
      mockIdentityService.updateProfile.mockRejectedValue(new Error('Holochain not connected'));

      const result = await service.applySessionToProfile('matthew@alpha.elohim.host');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Holochain not connected');
      expect(service.status()).toBe('failed');
      expect(mockSessionHumanService.clearAfterMigration).not.toHaveBeenCalled();
    });

    it('resets state back to idle', async () => {
      await service.applySessionToProfile('matthew@alpha.elohim.host');
      service.reset();

      expect(service.status()).toBe('idle');
    });
  });
});
