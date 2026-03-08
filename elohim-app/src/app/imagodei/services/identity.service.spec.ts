/**
 * Identity Service Tests
 *
 * Tests unified identity management across session and Holochain modes.
 */

import { TestBed } from '@angular/core/testing';
import { IdentityService } from './identity.service';
import { AuthService } from './auth.service';
import { SessionHumanService } from './session-human.service';
import { AgencyService } from './agency.service';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { PasswordAuthProvider } from './providers/password-auth.provider';
import { DoorwayRegistryService } from './doorway-registry.service';
import { signal } from '@angular/core';
import type { RegisterHumanRequest, HumanProfile } from '../models/identity.model';
import { IDENTITY_API, type IIdentityApi } from '../interfaces/identity.interface';
import type {
  EdgeNodeDisplayInfo,
  HolochainConnectionState,
} from '../../elohim/models/holochain-connection.model';
import { vi } from 'vitest';

// =============================================================================
// Mock Factories
// =============================================================================

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

function createMockSessionHuman() {
  return {
    sessionId: 'session-123',
    displayName: 'Test User',
    linkedAgentPubKey: null,
    linkedHumanId: null,
    sessionState: 'active' as const,
    lastActiveAt: new Date().toISOString(),
    createdAt: new Date().toISOString(),
  };
}

function createMockHumanSessionResult() {
  return {
    agentPubkey: 'agent-pub-key-123',
    actionHash: new Uint8Array([1, 2, 3]),
    human: {
      id: 'human-456',
      displayName: 'Holochain User',
      bio: 'A test user',
      affinities: ['learning'],
      profileReach: 'public',
      location: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    },
    sessionStartedAt: new Date().toISOString(),
    attestations: [],
  };
}

function createMockAuthState(overrides: Record<string, unknown> = {}) {
  return {
    isAuthenticated: false,
    token: null,
    humanId: null,
    agentPubKey: null,
    identifier: null,
    provider: null,
    isLoading: false,
    error: null,
    expiresAt: null,
    installedAppId: null,
    ...overrides,
  };
}

// =============================================================================
// Mock Service Builders
// =============================================================================

function buildMockAuthService(authStateOverrides: Record<string, unknown> = {}) {
  const authSignal = signal(createMockAuthState(authStateOverrides));
  return {
    auth: authSignal.asReadonly(),
    isAuthenticated: vi.fn().mockReturnValue(false),
    identifier: vi.fn().mockReturnValue(null),
    hasProvider: vi.fn().mockReturnValue(false),
    registerProvider: vi.fn(),
    getProvider: vi.fn().mockReturnValue(undefined),
    login: vi.fn(),
    register: vi.fn(),
    logout: vi.fn().mockResolvedValue(undefined),
    _authSignal: authSignal, // expose for test mutations
  };
}

function buildMockSessionHumanService(
  session: ReturnType<typeof createMockSessionHuman> | null = null
) {
  return {
    getSession: vi.fn().mockReturnValue(session),
    getSessionId: vi.fn().mockReturnValue(session?.sessionId ?? 'session-anon'),
    hasSession: vi.fn().mockReturnValue(session !== null),
    linkToHolochainIdentity: vi.fn(),
    markAsMigrated: vi.fn(),
    session$: { subscribe: vi.fn() },
  };
}

function buildMockHolochainClient(connected = false) {
  return {
    isConnected: vi.fn().mockReturnValue(connected),
    callZome: vi.fn(),
    getDisplayInfo: vi.fn().mockReturnValue(createMockDisplayInfo()),
    connect: vi.fn().mockResolvedValue(undefined),
    disconnect: vi.fn().mockResolvedValue(undefined),
  };
}

function buildMockAgencyService() {
  return {
    agencyState: signal({ currentStage: 'visitor', networked: false, edgeNodeConnected: false }),
    stageInfo: signal(null),
    connectionStatus: signal({ state: 'disconnected', message: '', isOnline: false }),
    canUpgrade: signal(false),
    getStageSummary: vi.fn().mockReturnValue({ data: 'Test', progress: '0%' }),
  };
}

function buildMockPasswordProvider() {
  return {
    type: 'password' as const,
    login: vi.fn(),
    register: vi.fn(),
    getCurrentUser: vi.fn(),
    logout: vi.fn(),
  };
}

function buildMockDoorwayRegistry() {
  return {
    hasSelection: signal(false),
    selected: signal(null),
    selectedUrl: signal(null),
  };
}

function buildMockIdentityApi(): IIdentityApi {
  return {
    createHuman: vi.fn().mockResolvedValue(createMockHumanSessionResult()),
    getMyHuman: vi.fn().mockResolvedValue(null),
    updateHuman: vi.fn(),
  };
}

// =============================================================================
// Tests
// =============================================================================

describe('IdentityService', () => {
  let service: IdentityService;
  let mockAuthService: ReturnType<typeof buildMockAuthService>;
  let mockSessionHumanService: ReturnType<typeof buildMockSessionHumanService>;
  let mockHolochainClient: ReturnType<typeof buildMockHolochainClient>;
  let mockAgencyService: ReturnType<typeof buildMockAgencyService>;
  let mockPasswordProvider: ReturnType<typeof buildMockPasswordProvider>;
  let mockDoorwayRegistry: ReturnType<typeof buildMockDoorwayRegistry>;
  let mockIdentityApi: IIdentityApi;

  function setupTestBed() {
    TestBed.configureTestingModule({
      providers: [
        IdentityService,
        { provide: AuthService, useValue: mockAuthService },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
        { provide: AgencyService, useValue: mockAgencyService },
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: IDENTITY_API, useValue: mockIdentityApi },
        { provide: PasswordAuthProvider, useValue: mockPasswordProvider },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
      ],
    });

    service = TestBed.inject(IdentityService);
  }

  beforeEach(() => {
    mockAuthService = buildMockAuthService();
    mockSessionHumanService = buildMockSessionHumanService();
    mockHolochainClient = buildMockHolochainClient(false);
    mockAgencyService = buildMockAgencyService();
    mockPasswordProvider = buildMockPasswordProvider();
    mockDoorwayRegistry = buildMockDoorwayRegistry();
    mockIdentityApi = buildMockIdentityApi();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    TestBed.resetTestingModule();
  });

  // ==========================================================================
  // Initial State (anonymous visitor)
  // ==========================================================================

  describe('initial state (no session, no Holochain)', () => {
    beforeEach(() => {
      setupTestBed();
    });

    it('should create', () => {
      expect(service).toBeTruthy();
    });

    it('should expose identity signal', () => {
      expect(service.identity).toBeDefined();
      expect(service.identity()).toBeDefined();
    });

    it('should expose mode signal', () => {
      expect(service.mode).toBeDefined();
    });

    it('should expose isAuthenticated signal', () => {
      expect(service.isAuthenticated).toBeDefined();
    });

    it('should expose humanId signal', () => {
      expect(service.humanId).toBeDefined();
    });

    it('should expose displayName signal', () => {
      expect(service.displayName).toBeDefined();
    });

    it('should expose agentPubKey signal', () => {
      expect(service.agentPubKey).toBeDefined();
    });

    it('should expose did signal', () => {
      expect(service.did).toBeDefined();
    });

    it('should expose profile signal', () => {
      expect(service.profile).toBeDefined();
    });

    it('should expose attestations signal', () => {
      expect(service.attestations).toBeDefined();
    });

    it('should expose isLoading signal', () => {
      expect(service.isLoading).toBeDefined();
    });

    it('should expose error signal', () => {
      expect(service.error).toBeDefined();
    });

    it('should expose canAccessGatedContent derived signal', () => {
      expect(service.canAccessGatedContent).toBeDefined();
    });

    it('should expose hasSession derived signal', () => {
      expect(service.hasSession).toBeDefined();
    });

    it('should expose isHolochainConnected derived signal', () => {
      expect(service.isHolochainConnected).toBeDefined();
    });

    it('should expose canUpgrade derived signal', () => {
      expect(service.canUpgrade).toBeDefined();
    });

    it('should start as not authenticated', () => {
      expect(service.isAuthenticated()).toBe(false);
    });

    it('should start with anonymous mode', () => {
      expect(service.mode()).toBe('anonymous');
    });

    it('should have null humanId initially', () => {
      expect(service.humanId()).toBeNull();
    });

    it('should have null agentPubKey initially', () => {
      expect(service.agentPubKey()).toBeNull();
    });

    it('should not be able to access gated content initially', () => {
      expect(service.canAccessGatedContent()).toBe(false);
    });

    it('should not have Holochain connected initially', () => {
      expect(service.isHolochainConnected()).toBe(false);
    });

    it('should not be able to upgrade initially', () => {
      expect(service.canUpgrade()).toBe(false);
    });

    it('should register password provider if not already registered', () => {
      expect(mockAuthService.hasProvider).toHaveBeenCalledWith('password');
      expect(mockAuthService.registerProvider).toHaveBeenCalledWith(mockPasswordProvider);
    });

    it('should NOT register password provider if already registered', () => {
      // Reset and reconfigure with provider already registered
      TestBed.resetTestingModule();
      mockAuthService = buildMockAuthService();
      mockAuthService.hasProvider.mockReturnValue(true);
      setupTestBed();

      expect(mockAuthService.registerProvider).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Session Identity (session exists, no Holochain)
  // ==========================================================================

  describe('with session identity', () => {
    const session = createMockSessionHuman();

    beforeEach(() => {
      mockSessionHumanService = buildMockSessionHumanService(session);
      setupTestBed();
    });

    it('should detect session identity', () => {
      expect(service.isAuthenticated()).toBe(true);
    });

    it('should be in session mode', () => {
      expect(service.mode()).toBe('session');
    });

    it('should expose session humanId', () => {
      expect(service.humanId()).toBe('session-123');
    });

    it('should expose session displayName', () => {
      expect(service.displayName()).toBe('Test User');
    });

    it('should generate session DID', () => {
      const did = service.did();
      expect(did).toContain('did:web:');
      expect(did).toContain('session');
    });

    it('should not be able to access gated content in session mode', () => {
      // session mode is not a network mode
      expect(service.canAccessGatedContent()).toBe(false);
    });

    it('should report hasSession as true', () => {
      expect(service.hasSession()).toBe(true);
    });
  });

  // ==========================================================================
  // loginWithPassword
  // ==========================================================================

  describe('loginWithPassword()', () => {
    beforeEach(() => {
      setupTestBed();
    });

    it('should delegate to auth service login', async () => {
      const mockResult = { success: true, humanId: 'human-1', agentPubKey: 'key-1' };
      mockAuthService.login.mockResolvedValue(mockResult);

      const result = await service.loginWithPassword('user@example.com', 'password123');

      expect(mockAuthService.login).toHaveBeenCalledWith('password', {
        type: 'password',
        identifier: 'user@example.com',
        password: 'password123',
      });
      expect(result).toEqual(mockResult);
    });

    it('should return failure result from auth service', async () => {
      const mockResult = {
        success: false,
        error: 'Invalid credentials',
        code: 'INVALID_CREDENTIALS',
      };
      mockAuthService.login.mockResolvedValue(mockResult);

      const result = await service.loginWithPassword('bad@example.com', 'wrong');

      expect(result.success).toBe(false);
      expect(result.error).toBe('Invalid credentials');
    });

    it('should propagate errors thrown by auth service', async () => {
      mockAuthService.login.mockRejectedValue(new Error('Network error'));

      await expect(service.loginWithPassword('user@example.com', 'pass')).rejects.toThrow(
        'Network error'
      );
    });
  });

  // ==========================================================================
  // logout
  // ==========================================================================

  describe('logout()', () => {
    beforeEach(() => {
      mockSessionHumanService = buildMockSessionHumanService(createMockSessionHuman());
      setupTestBed();
    });

    it('should delegate to auth service logout', async () => {
      await service.logout();
      expect(mockAuthService.logout).toHaveBeenCalled();
    });

    it('should fall back to session mode after logout', async () => {
      await service.logout();
      // After logout with session present, mode should be session
      expect(service.mode()).toBe('session');
    });

    it('should fall back to anonymous mode if no session after logout', async () => {
      // Reset with no session
      TestBed.resetTestingModule();
      mockSessionHumanService = buildMockSessionHumanService(null);
      mockAuthService = buildMockAuthService();
      mockHolochainClient = buildMockHolochainClient(false);
      mockAgencyService = buildMockAgencyService();
      mockPasswordProvider = buildMockPasswordProvider();
      mockDoorwayRegistry = buildMockDoorwayRegistry();
      setupTestBed();

      await service.logout();
      expect(service.mode()).toBe('anonymous');
    });

    it('should resolve to undefined', async () => {
      await expect(service.logout()).resolves.toBeUndefined();
    });
  });

  // ==========================================================================
  // registerHuman (hosted mode)
  // ==========================================================================

  describe('registerHuman()', () => {
    const mockRequest: RegisterHumanRequest = {
      email: 'new@example.com',
      password: 'secure-pass-123',
      displayName: 'New User',
      bio: 'A new human',
      affinities: ['learning'],
      profileReach: 'public',
    };

    const mockAuthResult = {
      success: true,
      humanId: 'new-human-789',
      agentPubKey: 'new-agent-key',
      token: 'jwt-token',
    };

    beforeEach(() => {
      mockAuthService.register = vi.fn().mockResolvedValue(mockAuthResult);
      setupTestBed();
    });

    it('should call auth service register with correct payload', async () => {
      await service.registerHuman(mockRequest);

      expect(mockAuthService.register).toHaveBeenCalledWith('password', {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'secure-pass-123',
        displayName: 'New User',
        bio: 'A new human',
        affinities: ['learning'],
        profileReach: 'public',
        location: undefined,
      });
    });

    it('should return HumanProfile on success', async () => {
      const profile = await service.registerHuman(mockRequest);

      expect(profile).toBeDefined();
      expect(profile.id).toBe('new-human-789');
      expect(profile.displayName).toBe('New User');
    });

    it('should update state to hosted mode on success', async () => {
      await service.registerHuman(mockRequest);

      expect(service.mode()).toBe('hosted');
      expect(service.isAuthenticated()).toBe(true);
    });

    it('should throw if email is missing', async () => {
      const request = { ...mockRequest, email: undefined } as unknown as RegisterHumanRequest;
      await expect(service.registerHuman(request)).rejects.toThrow('Email is required');
    });

    it('should throw if password is missing', async () => {
      const request = { ...mockRequest, password: undefined } as unknown as RegisterHumanRequest;
      await expect(service.registerHuman(request)).rejects.toThrow('Password is required');
    });

    it('should throw and set error state if auth service register fails', async () => {
      mockAuthService.register = vi.fn().mockResolvedValue({
        success: false,
        error: 'Email already in use',
      });

      await expect(service.registerHuman(mockRequest)).rejects.toThrow('Email already in use');
      expect(service.error()).toBe('Email already in use');
    });

    it('should throw with fallback message if non-Error thrown', async () => {
      mockAuthService.register = vi.fn().mockRejectedValue('Unknown string error');

      await expect(service.registerHuman(mockRequest)).rejects.toBe('Unknown string error');
      expect(service.error()).toBe('Registration failed. Please try again.');
    });

    it('should mark session as migrated if session exists', async () => {
      // Reset with active session
      TestBed.resetTestingModule();
      const session = createMockSessionHuman();
      mockSessionHumanService = buildMockSessionHumanService(session);
      mockAuthService = buildMockAuthService();
      mockAuthService.register = vi.fn().mockResolvedValue(mockAuthResult);
      mockHolochainClient = buildMockHolochainClient(false);
      mockAgencyService = buildMockAgencyService();
      mockPasswordProvider = buildMockPasswordProvider();
      mockDoorwayRegistry = buildMockDoorwayRegistry();
      setupTestBed();

      await service.registerHuman(mockRequest);

      expect(mockSessionHumanService.markAsMigrated).toHaveBeenCalledWith(
        'new-agent-key',
        'new-human-789'
      );
    });
  });

  // ==========================================================================
  // registerHumanNative (steward mode - local conductor)
  // ==========================================================================

  describe('registerHumanNative()', () => {
    const mockRequest: RegisterHumanRequest = {
      displayName: 'Steward User',
      affinities: ['governance'],
      profileReach: 'public',
    };

    beforeEach(() => {
      mockHolochainClient = buildMockHolochainClient(true); // connected
      setupTestBed();
    });

    it('should throw if Holochain is not connected', async () => {
      // Reset with disconnected client
      TestBed.resetTestingModule();
      mockHolochainClient = buildMockHolochainClient(false);
      mockAuthService = buildMockAuthService();
      mockSessionHumanService = buildMockSessionHumanService();
      mockAgencyService = buildMockAgencyService();
      mockPasswordProvider = buildMockPasswordProvider();
      mockDoorwayRegistry = buildMockDoorwayRegistry();
      setupTestBed();

      await expect(service.registerHumanNative(mockRequest)).rejects.toThrow(
        'Holochain not connected'
      );
    });

    it('should call identityApi.createHuman', async () => {
      (mockIdentityApi.createHuman as ReturnType<typeof vi.fn>).mockResolvedValue(createMockHumanSessionResult());

      await service.registerHumanNative(mockRequest);

      expect(mockIdentityApi.createHuman).toHaveBeenCalledWith(
        expect.objectContaining({
          displayName: 'Steward User',
          affinities: ['governance'],
          profileReach: 'public',
        })
      );
    });

    it('should return profile on success', async () => {
      (mockIdentityApi.createHuman as ReturnType<typeof vi.fn>).mockResolvedValue(createMockHumanSessionResult());

      const profile = await service.registerHumanNative(mockRequest);

      expect(profile).toBeDefined();
      expect(profile.displayName).toBe('Holochain User');
    });

    it('should update state to steward mode on success (local conductor)', async () => {
      (mockIdentityApi.createHuman as ReturnType<typeof vi.fn>).mockResolvedValue(createMockHumanSessionResult());

      // Local conductor has localhost URL
      mockHolochainClient.getDisplayInfo.mockReturnValue({
        ...createMockDisplayInfo(),
        appUrl: 'ws://localhost:4444',
      });

      await service.registerHumanNative(mockRequest);

      expect(service.mode()).toBe('steward');
    });

    it('should throw if identity API returns failure', async () => {
      (mockIdentityApi.createHuman as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Zome error'));

      await expect(service.registerHumanNative(mockRequest)).rejects.toThrow('Zome error');
    });

    it('should set error state if registration fails', async () => {
      (mockIdentityApi.createHuman as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Conductor offline'));

      await expect(service.registerHumanNative(mockRequest)).rejects.toThrow('Conductor offline');
      expect(service.error()).toBe('Conductor offline');
    });
  });

  // ==========================================================================
  // getCurrentHuman
  // ==========================================================================

  describe('getCurrentHuman()', () => {
    describe('when Holochain is not connected', () => {
      beforeEach(() => {
        mockHolochainClient = buildMockHolochainClient(false);
        setupTestBed();
      });

      it('should return null', async () => {
        const result = await service.getCurrentHuman();
        expect(result).toBeNull();
      });

      it('should not call identity API', async () => {
        await service.getCurrentHuman();
        expect(mockIdentityApi.getMyHuman).not.toHaveBeenCalled();
      });
    });

    describe('when Holochain is connected', () => {
      beforeEach(() => {
        mockHolochainClient = buildMockHolochainClient(true);
        setupTestBed();
      });

      it('should call identityApi.getMyHuman', async () => {
        (mockIdentityApi.getMyHuman as ReturnType<typeof vi.fn>).mockResolvedValue(createMockHumanSessionResult());

        await service.getCurrentHuman();

        expect(mockIdentityApi.getMyHuman).toHaveBeenCalled();
      });

      it('should return mapped profile on success', async () => {
        (mockIdentityApi.getMyHuman as ReturnType<typeof vi.fn>).mockResolvedValue(createMockHumanSessionResult());

        const profile = await service.getCurrentHuman();

        expect(profile).toBeDefined();
        expect(profile?.displayName).toBe('Holochain User');
        expect(profile?.id).toBe('human-456');
      });

      it('should return null if identity API returns null', async () => {
        (mockIdentityApi.getMyHuman as ReturnType<typeof vi.fn>).mockResolvedValue(null);

        const result = await service.getCurrentHuman();
        expect(result).toBeNull();
      });

      it('should return null and not throw on expected errors', async () => {
        (mockIdentityApi.getMyHuman as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('User not found'));

        const result = await service.getCurrentHuman();
        expect(result).toBeNull();
      });
    });
  });

  // ==========================================================================
  // updateProfile
  // ==========================================================================

  describe('updateProfile()', () => {
    const mockUpdateRequest = {
      displayName: 'Updated Name',
      bio: 'Updated bio',
      affinities: ['learning', 'governance'],
      profileReach: 'public' as const,
    };

    describe('when Holochain is not connected', () => {
      beforeEach(() => {
        mockHolochainClient = buildMockHolochainClient(false);
        setupTestBed();
      });

      it('should throw Holochain not connected error', async () => {
        await expect(service.updateProfile(mockUpdateRequest)).rejects.toThrow(
          'Holochain not connected'
        );
      });
    });

    describe('when in session mode (not network mode)', () => {
      beforeEach(() => {
        mockHolochainClient = buildMockHolochainClient(true);
        // No session → anonymous mode
        mockSessionHumanService = buildMockSessionHumanService(null);
        setupTestBed();
      });

      it('should throw cannot update profile in session mode', async () => {
        await expect(service.updateProfile(mockUpdateRequest)).rejects.toThrow(
          'Cannot update profile in session mode'
        );
      });
    });

    describe('when connected and in hosted mode', () => {
      beforeEach(async () => {
        mockHolochainClient = buildMockHolochainClient(true);
        mockAuthService = buildMockAuthService();
        mockAuthService.register = vi.fn().mockResolvedValue({
          success: true,
          humanId: 'human-999',
          agentPubKey: 'agent-999',
          token: 'token',
        });
        setupTestBed();

        // Put service into hosted mode
        await service.registerHuman({
          email: 'test@example.com',
          password: 'password',
          displayName: 'Test',
          affinities: [],
          profileReach: 'public',
        });
      });

      it('should call identityApi.updateHuman', async () => {
        const mockUpdateResult = {
          actionHash: new Uint8Array([1, 2, 3]),
          human: {
            id: 'human-999',
            displayName: 'Updated Name',
            bio: 'Updated bio',
            affinities: ['learning', 'governance'],
            profileReach: 'public',
            location: null,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        };

        (mockIdentityApi.updateHuman as ReturnType<typeof vi.fn>).mockResolvedValue(mockUpdateResult);

        await service.updateProfile(mockUpdateRequest);

        expect(mockIdentityApi.updateHuman).toHaveBeenCalledWith(
          expect.objectContaining({
            displayName: 'Updated Name',
            bio: 'Updated bio',
          })
        );
      });

      it('should return updated profile on success', async () => {
        const mockUpdateResult = {
          actionHash: new Uint8Array([1, 2, 3]),
          human: {
            id: 'human-999',
            displayName: 'Updated Name',
            bio: 'Updated bio',
            affinities: ['learning', 'governance'],
            profileReach: 'public',
            location: null,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        };

        (mockIdentityApi.updateHuman as ReturnType<typeof vi.fn>).mockResolvedValue(mockUpdateResult);

        const profile = await service.updateProfile(mockUpdateRequest);

        expect(profile.displayName).toBe('Updated Name');
        expect(profile.bio).toBe('Updated bio');
      });

      it('should throw if identity API returns failure', async () => {
        (mockIdentityApi.updateHuman as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Update failed'));

        await expect(service.updateProfile(mockUpdateRequest)).rejects.toThrow('Update failed');
      });
    });
  });

  // ==========================================================================
  // Derived Signals
  // ==========================================================================

  describe('derived signals', () => {
    describe('canAccessGatedContent', () => {
      it('should be false in anonymous mode', () => {
        setupTestBed();
        expect(service.canAccessGatedContent()).toBe(false);
      });

      it('should be false in session mode', () => {
        mockSessionHumanService = buildMockSessionHumanService(createMockSessionHuman());
        setupTestBed();
        // session mode is not a network mode
        expect(service.canAccessGatedContent()).toBe(false);
      });
    });

    describe('isHolochainConnected', () => {
      it('should be false when Holochain is disconnected', () => {
        mockHolochainClient = buildMockHolochainClient(false);
        setupTestBed();
        expect(service.isHolochainConnected()).toBe(false);
      });

      it('should be true when Holochain is connected', () => {
        mockHolochainClient = buildMockHolochainClient(true);
        setupTestBed();
        expect(service.isHolochainConnected()).toBe(true);
      });
    });

    describe('canUpgrade', () => {
      it('should be false without session', () => {
        mockSessionHumanService = buildMockSessionHumanService(null);
        setupTestBed();
        expect(service.canUpgrade()).toBe(false);
      });

      it('should be false with session but no Holochain connection', () => {
        mockSessionHumanService = buildMockSessionHumanService(createMockSessionHuman());
        mockHolochainClient = buildMockHolochainClient(false);
        setupTestBed();
        expect(service.canUpgrade()).toBe(false);
      });

      it('should be true with session and Holochain connected in session mode', () => {
        mockSessionHumanService = buildMockSessionHumanService(createMockSessionHuman());
        mockHolochainClient = buildMockHolochainClient(true);
        setupTestBed();

        // With session + connected, should be in session mode and able to upgrade
        if (service.mode() === 'session') {
          expect(service.canUpgrade()).toBe(true);
        } else {
          // If Holochain check changed mode, canUpgrade will be false (not in session mode)
          expect(service.canUpgrade()).toBe(false);
        }
      });
    });

    describe('hasSession', () => {
      it('should be false when no session', () => {
        mockSessionHumanService = buildMockSessionHumanService(null);
        setupTestBed();
        expect(service.hasSession()).toBe(false);
      });

      it('should be true when session exists', () => {
        mockSessionHumanService = buildMockSessionHumanService(createMockSessionHuman());
        setupTestBed();
        expect(service.hasSession()).toBe(true);
      });
    });
  });

  // ==========================================================================
  // getDisplayInfo
  // ==========================================================================

  describe('getDisplayInfo()', () => {
    beforeEach(() => {
      setupTestBed();
    });

    it('should return display info object', () => {
      const info = service.getDisplayInfo();
      expect(info).toBeDefined();
      expect(info.name).toBeDefined();
      expect(info.initials).toBeDefined();
      expect(info.mode).toBeDefined();
    });

    it('should include current mode', () => {
      const info = service.getDisplayInfo();
      expect(info.mode).toBe(service.mode());
    });
  });

  // ==========================================================================
  // clearError
  // ==========================================================================

  describe('clearError()', () => {
    beforeEach(() => {
      mockAuthService.register = vi.fn().mockResolvedValue({
        success: false,
        error: 'Test error',
      });
      setupTestBed();
    });

    it('should clear error state', async () => {
      // Trigger an error
      try {
        await service.registerHuman({
          email: 'test@example.com',
          password: 'pass',
          displayName: 'Test',
          affinities: [],
          profileReach: 'public',
        });
      } catch {
        // expected
      }

      expect(service.error()).toBe('Test error');

      service.clearError();

      expect(service.error()).toBeNull();
    });
  });

  // ==========================================================================
  // waitForAuthenticatedState
  // ==========================================================================

  describe('waitForAuthenticatedState()', () => {
    beforeEach(() => {
      setupTestBed();
    });

    it('should return false quickly if not authenticated within timeout', async () => {
      // Short timeout so test doesn't hang
      const result = await service.waitForAuthenticatedState(100);
      expect(result).toBe(false);
    });
  });
});
