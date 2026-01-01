import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { IdentityService } from './identity.service';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { SessionHumanService } from './session-human.service';
import { SovereigntyService } from './sovereignty.service';
import { AuthService } from './auth.service';
import { PasswordAuthProvider } from './providers/password-auth.provider';
import { signal } from '@angular/core';
import { INITIAL_IDENTITY_STATE, RegisterHumanRequest } from '../models/identity.model';

describe('IdentityService', () => {
  let service: IdentityService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let mockSovereigntyService: jasmine.SpyObj<SovereigntyService>;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockPasswordProvider: jasmine.SpyObj<PasswordAuthProvider>;

  // Signals for mock services
  let isConnectedSignal: ReturnType<typeof signal<boolean>>;
  let authSignal: ReturnType<typeof signal<any>>;

  const mockSession = {
    sessionId: 'session-123',
    displayName: 'Test User',
    isAnonymous: false,
    createdAt: new Date().toISOString(),
    lastActiveAt: new Date().toISOString(),
    sessionState: 'active' as const,
    linkedAgentPubKey: null,
    linkedHumanId: null,
  };

  const mockHumanSessionResult = {
    agent_pubkey: 'agent-pub-key-xyz',
    action_hash: new Uint8Array([1, 2, 3]),
    human: {
      id: 'human-abc',
      display_name: 'Test Human',
      bio: 'Test bio',
      affinities: ['learning', 'governance'],
      profile_reach: 'commons',
      location: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    },
    session_started_at: new Date().toISOString(),
    attestations: [],
  };

  beforeEach(() => {
    isConnectedSignal = signal(false);
    authSignal = signal({
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

    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', [
      'isConnected',
      'callZome',
      'getDisplayInfo',
    ]);
    mockHolochainClient.isConnected.and.callFake(() => isConnectedSignal());
    mockHolochainClient.getDisplayInfo.and.returnValue({
      appUrl: 'ws://localhost:8888',
      adminUrl: null,
      mode: 'direct' as const,
    });

    mockSessionHumanService = jasmine.createSpyObj('SessionHumanService', [
      'hasSession',
      'getSession',
      'linkToHolochainIdentity',
      'markAsMigrated',
    ]);
    mockSessionHumanService.hasSession.and.returnValue(true);
    mockSessionHumanService.getSession.and.returnValue(mockSession);

    mockSovereigntyService = jasmine.createSpyObj('SovereigntyService', ['getStage']);
    mockSovereigntyService.getStage.and.returnValue('visitor');

    mockAuthService = jasmine.createSpyObj('AuthService', [
      'hasProvider',
      'registerProvider',
      'login',
      'logout',
      'auth',
    ]);
    mockAuthService.hasProvider.and.returnValue(false);
    mockAuthService.auth.and.callFake(() => authSignal());
    mockAuthService.login.and.returnValue(Promise.resolve({ success: true, token: 'test-token', humanId: 'human-abc', agentPubKey: 'agent-xyz', expiresAt: new Date(Date.now() + 3600000).toISOString(), identifier: 'test@example.com' }));
    mockAuthService.logout.and.returnValue(Promise.resolve());

    mockPasswordProvider = jasmine.createSpyObj('PasswordAuthProvider', ['login', 'logout']);
    Object.defineProperty(mockPasswordProvider, 'type', { value: 'password' });

    TestBed.configureTestingModule({
      providers: [
        IdentityService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
        { provide: SovereigntyService, useValue: mockSovereigntyService },
        { provide: AuthService, useValue: mockAuthService },
        { provide: PasswordAuthProvider, useValue: mockPasswordProvider },
      ],
    });

    service = TestBed.inject(IdentityService);
  });

  it('should be created', () => {
    expect(service).toBeInstanceOf(IdentityService);
  });

  describe('Initial State', () => {
    it('should initialize with session identity when session exists', () => {
      expect(service.isAuthenticated()).toBe(true);
      expect(service.humanId()).toBe('session-123');
      expect(service.displayName()).toBe('Test User');
      expect(service.mode()).toBe('session');
    });

    it('should register password auth provider if not registered', () => {
      expect(mockAuthService.registerProvider).toHaveBeenCalled();
    });

    it('should not be loading initially', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should have no error initially', () => {
      expect(service.error()).toBeNull();
    });
  });

  describe('Computed Signals', () => {
    it('should correctly compute canAccessGatedContent', () => {
      // Session mode should not allow gated content access
      expect(service.canAccessGatedContent()).toBe(false);
    });

    it('should correctly compute hasSession', () => {
      expect(service.hasSession()).toBe(true);

      mockSessionHumanService.hasSession.and.returnValue(false);
      expect(service.hasSession()).toBe(false);
    });

    it('should correctly compute isHolochainConnected', () => {
      expect(service.isHolochainConnected()).toBe(false);

      isConnectedSignal.set(true);
      expect(service.isHolochainConnected()).toBe(true);
    });

    it('should correctly compute canUpgrade', () => {
      // Initially cannot upgrade (not connected)
      expect(service.canUpgrade()).toBe(false);

      // Connect Holochain
      isConnectedSignal.set(true);
      expect(service.canUpgrade()).toBe(true);
    });
  });

  describe('Holochain Identity Check', fakeAsync(() => {
    it('should check Holochain identity when connected', () => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: mockHumanSessionResult,
      }));

      // Simulate Holochain connection
      isConnectedSignal.set(true);
      tick();

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith({
        zomeName: 'imagodei',
        fnName: 'get_my_human',
        payload: null,
        roleName: 'imagodei',
      });
    });

    it('should update identity when Holochain identity found', fakeAsync(() => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: mockHumanSessionResult,
      }));

      isConnectedSignal.set(true);
      tick();

      expect(service.humanId()).toBe('human-abc');
      expect(service.displayName()).toBe('Test Human');
      expect(service.agentPubKey()).toBe('agent-pub-key-xyz');
      expect(service.mode()).toBe('self-sovereign'); // localhost = self-sovereign
    }));

    it('should remain in session mode when no Holochain identity', fakeAsync(() => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: null,
      }));

      isConnectedSignal.set(true);
      tick();

      expect(service.mode()).toBe('session');
      expect(service.humanId()).toBe('session-123');
    }));

    it('should handle errors gracefully', fakeAsync(() => {
      mockHolochainClient.callZome.and.returnValue(Promise.reject(new Error('No human found')));

      isConnectedSignal.set(true);
      tick();

      // Should stay in session mode, no error for expected errors
      expect(service.mode()).toBe('session');
      expect(service.error()).toBeNull();
    }));
  }));

  describe('Registration', () => {
    beforeEach(() => {
      isConnectedSignal.set(true);
    });

    it('should throw if Holochain not connected', async () => {
      isConnectedSignal.set(false);

      const request: RegisterHumanRequest = {
        displayName: 'New User',
        affinities: [],
        profileReach: 'commons',
      };

      await expectAsync(service.registerHuman(request)).toBeRejectedWithError('Holochain not connected');
    });

    it('should register human successfully', async () => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: mockHumanSessionResult,
      }));

      const request: RegisterHumanRequest = {
        displayName: 'New User',
        affinities: ['learning'],
        profileReach: 'commons',
      };

      const profile = await service.registerHuman(request);

      expect(profile.displayName).toBe('Test Human');
      expect(service.isAuthenticated()).toBe(true);
      expect(service.mode()).toBe('self-sovereign');
    });

    it('should mark session as migrated after registration', async () => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: mockHumanSessionResult,
      }));

      const request: RegisterHumanRequest = {
        displayName: 'New User',
        affinities: [],
        profileReach: 'commons',
      };

      await service.registerHuman(request);

      expect(mockSessionHumanService.markAsMigrated).toHaveBeenCalledWith(
        'agent-pub-key-xyz',
        'human-abc'
      );
    });

    it('should handle registration failure', async () => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: false,
        error: 'Registration failed',
      }));

      const request: RegisterHumanRequest = {
        displayName: 'New User',
        affinities: [],
        profileReach: 'commons',
      };

      await expectAsync(service.registerHuman(request)).toBeRejectedWithError('Registration failed');
      expect(service.error()).toBe('Registration failed');
    });
  });

  describe('Profile Management', () => {
    beforeEach(() => {
      isConnectedSignal.set(true);
    });

    it('should get current human profile', async () => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: mockHumanSessionResult,
      }));

      const profile = await service.getCurrentHuman();

      expect(profile).not.toBeNull();
      expect(profile!.displayName).toBe('Test Human');
      expect(profile!.bio).toBe('Test bio');
    });

    it('should return null if not connected', async () => {
      isConnectedSignal.set(false);

      const profile = await service.getCurrentHuman();

      expect(profile).toBeNull();
    });

    it('should return null if no profile found', async () => {
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: null,
      }));

      const profile = await service.getCurrentHuman();

      expect(profile).toBeNull();
    });
  });

  describe('Authentication', () => {
    it('should delegate login to AuthService', async () => {
      await service.loginWithPassword('test@example.com', 'password123');

      expect(mockAuthService.login).toHaveBeenCalledWith('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });
    });

    it('should logout and fall back to session', async () => {
      await service.logout();

      expect(mockAuthService.logout).toHaveBeenCalled();
      expect(service.mode()).toBe('session');
    });
  });

  describe('Display Helpers', () => {
    it('should return display info', () => {
      const info = service.getDisplayInfo();

      expect(info.name).toBe('Test User');
      expect(info.initials).toBe('TU');
      expect(info.mode).toBe('session');
    });

    it('should clear error', () => {
      // Set an error first
      service['updateState']({ error: 'Test error' });
      expect(service.error()).toBe('Test error');

      service.clearError();
      expect(service.error()).toBeNull();
    });
  });

  describe('Fallback to Session', () => {
    it('should fall back to session when Holochain disconnects', fakeAsync(() => {
      // First connect and get Holochain identity
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: mockHumanSessionResult,
      }));

      isConnectedSignal.set(true);
      tick();

      expect(service.mode()).toBe('self-sovereign');

      // Disconnect
      isConnectedSignal.set(false);
      tick();

      expect(service.mode()).toBe('session');
      expect(service.humanId()).toBe('session-123');
    }));

    it('should reset to initial state if no session', fakeAsync(() => {
      mockSessionHumanService.getSession.and.returnValue(null);

      // Trigger fallback
      service['fallbackToSession']();

      expect(service.isAuthenticated()).toBe(false);
      expect(service.mode()).toBe('anonymous');
    }));
  });

  describe('Conductor Detection', () => {
    it('should detect localhost as local conductor', () => {
      mockHolochainClient.getDisplayInfo.and.returnValue({
        appUrl: 'ws://localhost:8888',
        adminUrl: null,
        mode: 'direct' as const,
      });

      const result = service['detectConductorType']();

      expect(result.isLocal).toBe(true);
    });

    it('should detect remote URL as hosted', () => {
      mockHolochainClient.getDisplayInfo.and.returnValue({
        appUrl: 'wss://doorway.elohim.host/api',
        adminUrl: null,
        mode: 'doorway' as const,
      });

      const result = service['detectConductorType']();

      expect(result.isLocal).toBe(false);
    });

    it('should detect 127.0.0.1 as local', () => {
      mockHolochainClient.getDisplayInfo.and.returnValue({
        appUrl: 'ws://127.0.0.1:8888',
        adminUrl: null,
        mode: 'direct' as const,
      });

      const result = service['detectConductorType']();

      expect(result.isLocal).toBe(true);
    });
  });
});
