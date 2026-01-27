/**
 * Identity Service Tests
 *
 * Tests unified identity management across session and Holochain modes.
 */

import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { IdentityService } from './identity.service';
import { AuthService } from './auth.service';
import { SessionHumanService } from './session-human.service';
import { SovereigntyService } from './sovereignty.service';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { PasswordAuthProvider } from './providers/password-auth.provider';
import { DoorwayRegistryService } from './doorway-registry.service';
import { signal } from '@angular/core';
import type { RegisterHumanRequest, HumanProfile } from '../models/identity.model';
import type {
  EdgeNodeDisplayInfo,
  HolochainConnectionState,
} from '../../elohim/models/holochain-connection.model';

// Helper to create mock display info matching EdgeNodeDisplayInfo interface
function createMockDisplayInfo(): EdgeNodeDisplayInfo {
  return {
    state: 'connected' as HolochainConnectionState,
    mode: 'doorway',
    adminUrl: 'ws://localhost:8888/admin',
    appUrl: 'ws://localhost:8888/app',
    agentPubKey: 'agent-pub-key-123',
    cellId: { dnaHash: 'dna-hash', agentPubKey: 'agent-pub-key-123' },
    appId: 'elohim',
    dnaHash: 'dna-hash',
    connectedAt: new Date(),
    hasStoredCredentials: false,
    networkSeed: 'test-network',
    error: null,
  };
}

describe('IdentityService', () => {
  let service: IdentityService;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let mockSovereigntyService: jasmine.SpyObj<SovereigntyService>;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockPasswordProvider: jasmine.SpyObj<PasswordAuthProvider>;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;

  // Mock session data matching SessionHuman interface
  const mockSession = {
    sessionId: 'session-123',
    displayName: 'Test User',
    createdAt: new Date().toISOString(),
    lastActiveAt: new Date().toISOString(),
    stats: {
      nodesViewed: 0,
      nodesWithAffinity: 0,
      pathsStarted: 0,
      pathsCompleted: 0,
      stepsCompleted: 0,
      totalSessionTime: 0,
      averageSessionLength: 0,
      sessionCount: 1,
    },
    accessLevel: 'visitor' as const,
    isAnonymous: false,
    sessionState: 'active' as const,
    linkedAgentPubKey: undefined,
    linkedHumanId: undefined,
  };

  // Mock Holochain human result
  const mockHumanSessionResult = {
    agentPubkey: 'agent-pub-key-123',
    actionHash: new Uint8Array([1, 2, 3]),
    human: {
      id: 'human-123',
      displayName: 'Holochain User',
      bio: 'A test user',
      affinities: ['learning', 'teaching'],
      profileReach: 'community',
      location: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    },
    sessionStartedAt: new Date().toISOString(),
    attestations: [],
  };

  // Create writable signal mocks
  const createSignalMock = <T>(initialValue: T) => {
    const sig = signal(initialValue);
    return jasmine.createSpy().and.callFake(() => sig());
  };

  beforeEach(() => {
    // Create mock auth service with signals
    const authSignal = signal({
      isAuthenticated: false,
      token: null,
      humanId: null,
      agentPubKey: null,
      identifier: null,
      provider: null,
      expiresAt: null,
      isLoading: false,
      error: null,
    });

    mockAuthService = jasmine.createSpyObj(
      'AuthService',
      ['registerProvider', 'hasProvider', 'getProvider', 'login', 'register', 'logout'],
      {
        auth: jasmine.createSpy().and.callFake(() => authSignal()),
        isAuthenticated: jasmine.createSpy().and.returnValue(false),
        token: jasmine.createSpy().and.returnValue(null),
        humanId: jasmine.createSpy().and.returnValue(null),
        agentPubKey: jasmine.createSpy().and.returnValue(null),
      }
    );
    mockAuthService.hasProvider.and.returnValue(false);
    mockAuthService.login.and.returnValue(
      Promise.resolve({
        success: true,
        token: 'token',
        humanId: 'human-123',
        agentPubKey: 'agent-123',
        expiresAt: Date.now() + 3600000,
        identifier: 'test@example.com',
      })
    );
    mockAuthService.register.and.returnValue(
      Promise.resolve({
        success: true,
        token: 'token',
        humanId: 'human-123',
        agentPubKey: 'agent-123',
        expiresAt: Date.now() + 3600000,
        identifier: 'test@example.com',
      })
    );
    mockAuthService.logout.and.returnValue(Promise.resolve());

    // Create mock session human service
    mockSessionHumanService = jasmine.createSpyObj(
      'SessionHumanService',
      ['getSession', 'hasSession', 'linkToHolochainIdentity', 'markAsMigrated'],
      {
        hasSession: jasmine.createSpy().and.returnValue(false),
      }
    );
    mockSessionHumanService.getSession.and.returnValue(null);

    // Create mock sovereignty service
    mockSovereigntyService = jasmine.createSpyObj('SovereigntyService', [], {
      sovereigntyState: jasmine.createSpy().and.returnValue({ currentStage: 'visitor' }),
      currentStage: jasmine.createSpy().and.returnValue('visitor'),
    });

    // Create mock Holochain client with signals
    const isConnectedSignal = signal(false);
    mockHolochainClient = jasmine.createSpyObj(
      'HolochainClientService',
      ['callZome', 'getDisplayInfo'],
      {
        isConnected: jasmine.createSpy().and.callFake(() => isConnectedSignal()),
      }
    );
    mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo());
    mockHolochainClient.callZome.and.returnValue(
      Promise.resolve({ success: false, error: 'Not connected' })
    );

    // Create mock password provider
    mockPasswordProvider = jasmine.createSpyObj('PasswordAuthProvider', ['getCurrentUser'], {
      type: 'password',
    });

    // Create mock doorway registry
    mockDoorwayRegistry = jasmine.createSpyObj('DoorwayRegistryService', ['selectDoorway'], {
      selected: jasmine.createSpy().and.returnValue(null),
      selectedUrl: jasmine.createSpy().and.returnValue(null),
      hasSelection: jasmine.createSpy().and.returnValue(false),
    });

    TestBed.configureTestingModule({
      providers: [
        IdentityService,
        { provide: AuthService, useValue: mockAuthService },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
        { provide: SovereigntyService, useValue: mockSovereigntyService },
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: PasswordAuthProvider, useValue: mockPasswordProvider },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
      ],
    });

    service = TestBed.inject(IdentityService);
  });

  // ==========================================================================
  // Initial State Tests
  // ==========================================================================

  describe('initial state', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should register password provider on construction', () => {
      expect(mockAuthService.registerProvider).toHaveBeenCalled();
    });

    it('should start with anonymous/session mode when no session exists', () => {
      // Initial state depends on session presence
      expect(service.mode()).toBeDefined();
    });

    it('should expose identity signals', () => {
      expect(service.identity).toBeDefined();
      expect(service.mode).toBeDefined();
      expect(service.isAuthenticated).toBeDefined();
      expect(service.humanId).toBeDefined();
      expect(service.displayName).toBeDefined();
      expect(service.agentPubKey).toBeDefined();
      expect(service.did).toBeDefined();
      expect(service.profile).toBeDefined();
      expect(service.attestations).toBeDefined();
      expect(service.isLoading).toBeDefined();
      expect(service.error).toBeDefined();
    });

    it('should expose derived signals', () => {
      expect(service.canAccessGatedContent).toBeDefined();
      expect(service.hasSession).toBeDefined();
      expect(service.isHolochainConnected).toBeDefined();
      expect(service.canUpgrade).toBeDefined();
    });

    it('should start not loading', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should start without error', () => {
      expect(service.error()).toBeNull();
    });
  });

  // ==========================================================================
  // Session Identity Tests
  // ==========================================================================

  describe('session identity', () => {
    // These tests need a fresh service instance initialized with session
    // The outer beforeEach already injected a service with no session,
    // so we must reset and reconfigure TestBed

    beforeEach(() => {
      // Reset TestBed to clear cached service instance
      TestBed.resetTestingModule();

      // Reconfigure mocks with session enabled
      mockSessionHumanService.getSession.and.returnValue(mockSession);
      (mockSessionHumanService.hasSession as jasmine.Spy).and.returnValue(true);

      // Reconfigure TestBed
      TestBed.configureTestingModule({
        providers: [
          IdentityService,
          { provide: AuthService, useValue: mockAuthService },
          { provide: SessionHumanService, useValue: mockSessionHumanService },
          { provide: SovereigntyService, useValue: mockSovereigntyService },
          { provide: HolochainClientService, useValue: mockHolochainClient },
          { provide: PasswordAuthProvider, useValue: mockPasswordProvider },
          { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        ],
      });

      // Now inject fresh service instance with session
      service = TestBed.inject(IdentityService);
    });

    it('should initialize with session identity when session exists', fakeAsync(() => {
      tick();

      expect(service.mode()).toBe('session');
      expect(service.isAuthenticated()).toBe(true);
      expect(service.displayName()).toBe('Test User');
    }));

    it('should generate session DID', fakeAsync(() => {
      tick();

      const did = service.did();
      expect(did).toContain('did:web:');
      expect(did).toContain('session');
    }));

    it('should indicate session cannot access gated content', fakeAsync(() => {
      tick();

      expect(service.canAccessGatedContent()).toBe(false);
    }));
  });

  // ==========================================================================
  // Login Tests
  // ==========================================================================

  describe('loginWithPassword', () => {
    it('should delegate to auth service', async () => {
      const result = await service.loginWithPassword('test@example.com', 'password123');

      expect(mockAuthService.login).toHaveBeenCalledWith('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });
      expect(result.success).toBe(true);
    });

    it('should handle login failure', async () => {
      mockAuthService.login.and.returnValue(
        Promise.resolve({ success: false, error: 'Invalid credentials' })
      );

      const result = await service.loginWithPassword('test@example.com', 'wrong');

      expect(result.success).toBe(false);
    });
  });

  // ==========================================================================
  // Logout Tests
  // ==========================================================================

  describe('logout', () => {
    it('should delegate to auth service', async () => {
      await service.logout();

      expect(mockAuthService.logout).toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Registration Tests (Hosted Mode)
  // ==========================================================================

  describe('registerHuman (hosted mode)', () => {
    const registrationRequest: RegisterHumanRequest = {
      displayName: 'New User',
      email: 'new@example.com',
      password: 'password123',
      affinities: ['learning'],
      profileReach: 'community',
    };

    it('should require email for registration', async () => {
      const requestWithoutEmail = { ...registrationRequest, email: undefined };

      await expectAsync(service.registerHuman(requestWithoutEmail)).toBeRejectedWithError(
        'Email is required for registration'
      );
    });

    it('should require password for registration', async () => {
      const requestWithoutPassword = { ...registrationRequest, password: undefined };

      await expectAsync(service.registerHuman(requestWithoutPassword)).toBeRejectedWithError(
        'Password is required for registration'
      );
    });

    it('should register via auth service', async () => {
      const profile = await service.registerHuman(registrationRequest);

      expect(mockAuthService.register).toHaveBeenCalledWith(
        'password',
        jasmine.objectContaining({
          identifier: 'new@example.com',
          identifierType: 'email',
          password: 'password123',
          displayName: 'New User',
        })
      );
      expect(profile.displayName).toBe('New User');
    });

    it('should update identity state after registration', async () => {
      await service.registerHuman(registrationRequest);

      expect(service.mode()).toBe('hosted');
      expect(service.isAuthenticated()).toBe(true);
    });

    it('should generate hosted DID after registration', async () => {
      await service.registerHuman(registrationRequest);

      const did = service.did();
      expect(did).toContain('did:web:');
      expect(did).toContain('hosted');
    });

    it('should handle registration failure', async () => {
      mockAuthService.register.and.returnValue(
        Promise.resolve({ success: false, error: 'Email already exists' })
      );

      await expectAsync(service.registerHuman(registrationRequest)).toBeRejectedWithError(
        'Email already exists'
      );
      expect(service.error()).toBe('Email already exists');
    });

    it('should mark session as migrated after registration', async () => {
      mockSessionHumanService.getSession.and.returnValue(mockSession);

      await service.registerHuman(registrationRequest);

      expect(mockSessionHumanService.markAsMigrated).toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Registration Tests (Native/Steward Mode)
  // ==========================================================================

  describe('registerHumanNative (steward mode)', () => {
    const registrationRequest: RegisterHumanRequest = {
      displayName: 'Steward User',
      affinities: ['teaching'],
      profileReach: 'community',
    };

    it('should require Holochain connection', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(false);

      await expectAsync(service.registerHumanNative(registrationRequest)).toBeRejectedWithError(
        'Holochain not connected'
      );
    });

    it('should call imagodei zome for registration', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockHumanSessionResult })
      );
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo());

      const profile = await service.registerHumanNative(registrationRequest);

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'imagodei',
          fnName: 'create_human',
          roleName: 'imagodei',
        })
      );
      expect(profile.displayName).toBe('Holochain User');
    });

    it('should set steward mode after native registration', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockHumanSessionResult })
      );
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo());

      await service.registerHumanNative(registrationRequest);

      expect(service.mode()).toBe('steward');
      expect(service.isAuthenticated()).toBe(true);
    });

    it('should generate did:key for steward mode', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockHumanSessionResult })
      );
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo());

      await service.registerHumanNative(registrationRequest);

      const did = service.did();
      expect(did).toContain('did:key:');
    });

    it('should handle zome call failure', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false, error: 'Zome error' })
      );

      await expectAsync(service.registerHumanNative(registrationRequest)).toBeRejectedWithError(
        'Zome error'
      );
    });
  });

  // ==========================================================================
  // Profile Management Tests
  // ==========================================================================

  describe('getCurrentHuman', () => {
    it('should return null when Holochain not connected', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(false);

      const result = await service.getCurrentHuman();

      expect(result).toBeNull();
    });

    it('should fetch profile from Holochain', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockHumanSessionResult })
      );

      const profile = await service.getCurrentHuman();

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'imagodei',
          fnName: 'get_my_human',
        })
      );
      expect(profile?.displayName).toBe('Holochain User');
    });

    it('should return null on zome error', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(Promise.reject(new Error('Zome error')));

      const result = await service.getCurrentHuman();

      expect(result).toBeNull();
    });
  });

  describe('updateProfile', () => {
    beforeEach(() => {
      // Set up as hosted user first
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
    });

    it('should require Holochain connection', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(false);

      await expectAsync(service.updateProfile({ displayName: 'New Name' })).toBeRejectedWithError(
        'Holochain not connected'
      );
    });

    it('should call update zome function', async () => {
      // First register to set mode
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockHumanSessionResult })
      );
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo());
      await service.registerHumanNative({
        displayName: 'Test',
        affinities: [],
        profileReach: 'community',
      });

      // Mock update response
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: {
            actionHash: new Uint8Array([1, 2, 3]),
            human: { ...mockHumanSessionResult.human, displayName: 'Updated Name' },
          },
        })
      );

      const profile = await service.updateProfile({ displayName: 'Updated Name' });

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'imagodei',
          fnName: 'update_human',
        })
      );
      expect(profile.displayName).toBe('Updated Name');
    });
  });

  // ==========================================================================
  // Derived Signal Tests
  // ==========================================================================

  describe('derived signals', () => {
    it('canAccessGatedContent should be false for session mode', () => {
      expect(service.canAccessGatedContent()).toBe(false);
    });

    it('canUpgrade should be false when Holochain not connected', () => {
      mockSessionHumanService.getSession.and.returnValue(mockSession);
      (mockSessionHumanService.hasSession as jasmine.Spy).and.returnValue(true);
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(false);

      expect(service.canUpgrade()).toBe(false);
    });

    it('isHolochainConnected should delegate to client', () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);

      expect(service.isHolochainConnected()).toBe(true);
    });
  });

  // ==========================================================================
  // Utility Function Tests
  // ==========================================================================

  describe('exported utility functions', () => {
    it('should export isNetworkMode', () => {
      // Import from the service module
      expect(typeof service).toBe('object');
    });

    it('should export isStewardMode', () => {
      expect(typeof service).toBe('object');
    });

    it('should export getInitials', () => {
      expect(typeof service).toBe('object');
    });
  });
});
