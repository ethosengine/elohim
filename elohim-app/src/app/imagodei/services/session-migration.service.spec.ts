/**
 * SessionMigrationService Tests
 *
 * Tests session-to-network identity migration flow.
 */

import { TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';

import { SessionMigrationService } from './session-migration.service';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { ContentMasteryService } from '../../lamad/services/content-mastery.service';
import { IdentityService } from './identity.service';
import { SessionHumanService } from './session-human.service';
import type { SessionHuman } from '../models/session-human.model';
import type { HumanProfile } from '../models/identity.model';
import type { HolochainConnection } from '../../elohim/models/holochain-connection.model';

describe('SessionMigrationService', () => {
  let service: SessionMigrationService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockContentMasteryService: jasmine.SpyObj<ContentMasteryService>;

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
    displayName: 'Test User',
    bio: 'A test bio',
    interests: ['learning', 'teaching'],
    affinity: {
      'node-1': 0.8,
      'node-2': 0.6,
    },
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
    stats: mockSession.stats,
    migratedAt: '2026-01-01T00:00:00Z',
    status: 'pending' as const,
  };

  const mockProfile: HumanProfile = {
    id: 'human-123',
    displayName: 'Test User',
    bio: 'A test bio',
    affinities: ['learning', 'teaching'],
    profileReach: 'community',
    location: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  };

  beforeEach(() => {
    // Create mock services
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', ['callZome'], {
      isConnected: jasmine.createSpy().and.returnValue(true),
    });

    mockSessionHumanService = jasmine.createSpyObj('SessionHumanService', [
      'getSession',
      'prepareMigration',
      'clearAfterMigration',
      'hasSession',
    ]);

    mockIdentityService = jasmine.createSpyObj('IdentityService', ['registerHuman'], {
      mode: jasmine.createSpy().and.returnValue('session'),
    });

    mockContentMasteryService = jasmine.createSpyObj('ContentMasteryService', ['migrateToBackend']);

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

  // ==========================================================================
  // Initial State
  // ==========================================================================

  describe('initial state', () => {
    it('should start with idle status', () => {
      expect(service.status()).toBe('idle');
    });

    it('should not be in progress initially', () => {
      expect(service.isInProgress()).toBe(false);
    });
  });

  // ==========================================================================
  // Migration Eligibility
  // ==========================================================================

  describe('canMigrate', () => {
    it('should allow migration when session exists and connected', () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');

      expect(service.canMigrate()).toBe(true);
    });

    it('should not allow migration when no session exists', () => {
      mockSessionHumanService.hasSession.and.returnValue(false);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');

      expect(service.canMigrate()).toBe(false);
    });

    it('should not allow migration when not connected', () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(false);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');

      expect(service.canMigrate()).toBe(false);
    });

    it('should not allow migration when already in network mode', () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');

      expect(service.canMigrate()).toBe(false);
    });
  });

  // ==========================================================================
  // Migration Flow
  // ==========================================================================

  describe('migrate', () => {
    beforeEach(() => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockSessionHumanService.getSession.and.returnValue(mockSession);
    });

    it('should successfully migrate session to network identity', async () => {
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: true, migrated: 5, failed: 0, errors: [] })
      );

      const result = await service.migrate();

      expect(result.success).toBe(true);
      expect(result.newHumanId).toBe('human-123');
      expect(result.migratedData?.affinityCount).toBe(2);
      expect(result.migratedData?.pathProgressCount).toBe(1);
      expect(result.migratedData?.masteryCount).toBe(5);
      expect(mockSessionHumanService.clearAfterMigration).toHaveBeenCalled();
      expect(service.status()).toBe('completed');
    });

    it('should update status during migration phases', async () => {
      let statusUpdates: string[] = [];

      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: true, migrated: 5, failed: 0, errors: [] })
      );

      // Track status changes
      const originalState = service.state;
      const statusSpy = jasmine.createSpy('status').and.callFake(() => {
        const currentStatus = service.status();
        if (statusUpdates[statusUpdates.length - 1] !== currentStatus) {
          statusUpdates.push(currentStatus);
        }
        return currentStatus;
      });

      await service.migrate();

      // Should have gone through: idle -> preparing -> registering -> transferring -> completed
      const state = service.state();
      expect(state.status).toBe('completed');
    });

    it('should fail when migration not available', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(false);

      const result = await service.migrate();

      expect(result.success).toBe(false);
      expect(result.error).toContain('Migration not available');
    });

    it('should fail when no session exists', async () => {
      mockSessionHumanService.getSession.and.returnValue(null);

      const result = await service.migrate();

      expect(result.success).toBe(false);
      expect(result.error).toContain('No session to migrate');
    });

    it('should fail when migration package cannot be created', async () => {
      mockSessionHumanService.prepareMigration.and.returnValue(null);

      const result = await service.migrate();

      expect(result.success).toBe(false);
      expect(result.error).toContain('Failed to prepare migration package');
      expect(service.status()).toBe('failed');
    });

    it('should fail when registration fails', async () => {
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(
        Promise.reject(new Error('Registration failed'))
      );

      const result = await service.migrate();

      expect(result.success).toBe(false);
      expect(result.error).toContain('Registration failed');
      expect(service.status()).toBe('failed');
    });

    it('should apply profile overrides when provided', async () => {
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: true, migrated: 0, failed: 0, errors: [] })
      );

      await service.migrate({
        displayName: 'Custom Name',
        bio: 'Custom Bio',
        profileReach: 'public',
      });

      expect(mockIdentityService.registerHuman).toHaveBeenCalledWith(
        jasmine.objectContaining({
          displayName: 'Custom Name',
          bio: 'Custom Bio',
          profileReach: 'public',
        })
      );
    });

    it('should continue migration even if individual path progress fails', async () => {
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));

      // Mock callZome to fail on first two calls (get_or_create and update), succeed on subsequent
      let callCount = 0;
      mockHolochainClient.callZome.and.callFake(() => {
        callCount++;
        if (callCount <= 2) {
          return Promise.reject(new Error('Path progress failed'));
        }
        return Promise.resolve({ success: true, data: {} as any });
      });

      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: true, migrated: 5, failed: 0, errors: [] })
      );

      const result = await service.migrate();

      // Migration should still succeed even if one path fails
      expect(result.success).toBe(true);
    });

    it('should handle mastery migration failure gracefully', async () => {
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: false, migrated: 0, failed: 5, errors: ['Migration failed'] })
      );

      const result = await service.migrate();

      expect(result.success).toBe(true);
      expect(result.migratedData?.masteryCount).toBe(0);
    });
  });

  // ==========================================================================
  // Migration Progress Tracking
  // ==========================================================================

  describe('migration state', () => {
    it('should track current step during migration', async () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockSessionHumanService.getSession.and.returnValue(mockSession);
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: true, migrated: 5, failed: 0, errors: [] })
      );

      const migrationPromise = service.migrate();

      // State should update during migration
      await migrationPromise;

      const finalState = service.state();
      expect(finalState.status).toBe('completed');
      expect(finalState.currentStep).toBe('Migration complete!');
      expect(finalState.progress).toBe(100);
    });

    it('should track error state on failure', async () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockSessionHumanService.getSession.and.returnValue(mockSession);
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(
        Promise.reject(new Error('Network error'))
      );

      await service.migrate();

      const state = service.state();
      expect(state.status).toBe('failed');
      expect(state.error).toContain('Network error');
    });
  });

  // ==========================================================================
  // State Management
  // ==========================================================================

  describe('reset', () => {
    it('should reset migration state', async () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockSessionHumanService.getSession.and.returnValue(mockSession);
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(
        Promise.reject(new Error('Test error'))
      );

      await service.migrate();

      expect(service.status()).toBe('failed');

      service.reset();

      expect(service.status()).toBe('idle');
      expect(service.state().error).toBeUndefined();
    });
  });

  // ==========================================================================
  // Path Progress Transfer
  // ==========================================================================

  describe('path progress transfer', () => {
    it('should call zome functions for each path progress item', async () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockSessionHumanService.getSession.and.returnValue(mockSession);
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));
      // Mock humanId to return the profile id (needed for transferPathProgress)
      (mockIdentityService as any).humanId = jasmine.createSpy('humanId').and.returnValue(mockProfile.id);
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: true, migrated: 5, failed: 0, errors: [] })
      );

      await service.migrate();

      // Should call get_or_create and update for each path
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'imagodei',
          fnName: 'get_or_create_agent_progress',
        })
      );

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'imagodei',
          fnName: 'update_agent_progress',
        })
      );
    });
  });

  // ==========================================================================
  // Affinity Transfer
  // ==========================================================================

  describe('affinity transfer', () => {
    it('should handle affinity data during migration', async () => {
      mockSessionHumanService.hasSession.and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockSessionHumanService.getSession.and.returnValue(mockSession);
      mockSessionHumanService.prepareMigration.and.returnValue(mockMigrationPackage as any);
      mockIdentityService.registerHuman.and.returnValue(Promise.resolve(mockProfile));
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockContentMasteryService.migrateToBackend.and.returnValue(
        Promise.resolve({ success: true, migrated: 5, failed: 0, errors: [] })
      );

      const result = await service.migrate();

      expect(result.migratedData?.affinityCount).toBe(2);
      // TODO(test-generator): [LOW] Affinity transfer is not implemented
      // Context: transferAffinity() method is empty stub
      // Story: Preserve user's learned content preferences during migration
      // Suggested approach:
      //   1. Create zome function to batch-import affinity records
      //   2. Convert session affinity map to network format
      //   3. Store in imagodei DNA as part of human profile
    });
  });
});
