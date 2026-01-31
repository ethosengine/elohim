/**
 * TauriAuthService Tests
 *
 * Tests native OAuth handling for Tauri desktop app.
 */

import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';

import { TauriAuthService } from './tauri-auth.service';
import { AuthService } from './auth.service';
import { DoorwayRegistryService } from './doorway-registry.service';

describe('TauriAuthService', () => {
  let service: TauriAuthService;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let mockTauriWindow: any;
  let fetchSpy: jasmine.Spy;
  let originalWindow: any;

  const mockDoorway = {
    doorway: {
      id: 'doorway-1',
      url: 'https://doorway.example.com',
      displayName: 'Test Doorway',
    },
  };

  const mockSession = {
    id: 'session-123',
    humanId: 'human-123',
    agentPubKey: 'agent-pub-key-123',
    doorwayUrl: 'https://doorway.example.com',
    doorwayId: 'doorway-1',
    identifier: 'user@example.com',
    displayName: 'Test User',
    profileImageHash: undefined,
    isActive: 1,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    lastSyncedAt: undefined,
    bootstrapUrl: undefined,
  };

  beforeEach(() => {
    // Save original window __TAURI__ property
    originalWindow = (window as any).__TAURI__;

    // Mock Router
    mockRouter = jasmine.createSpyObj('Router', ['navigate']);

    // Mock AuthService
    mockAuthService = jasmine.createSpyObj('AuthService', ['setTauriSession', 'logout']);

    // Mock DoorwayRegistryService
    mockDoorwayRegistry = jasmine.createSpyObj('DoorwayRegistryService', [], {
      selected: jasmine.createSpy().and.returnValue(mockDoorway),
    });

    // Mock Tauri window - will be set by individual tests as needed
    mockTauriWindow = {
      __TAURI__: {
        event: {
          listen: jasmine.createSpy('listen').and.returnValue(Promise.resolve(() => {})),
        },
      },
    };

    // Mock fetch
    fetchSpy = jasmine.createSpy('fetch');
    (window as any).fetch = fetchSpy;

    TestBed.configureTestingModule({
      providers: [
        TauriAuthService,
        { provide: Router, useValue: mockRouter },
        { provide: AuthService, useValue: mockAuthService },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
      ],
    });

    service = TestBed.inject(TauriAuthService);
  });

  afterEach(() => {
    // Restore original window.__TAURI__ property
    if (originalWindow === undefined) {
      delete (window as any).__TAURI__;
    } else {
      (window as any).__TAURI__ = originalWindow;
    }
  });

  // ==========================================================================
  // Environment Detection
  // ==========================================================================

  describe('environment detection', () => {
    it('should detect Tauri environment when __TAURI__ exists', () => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;

      expect(service.isTauriEnvironment()).toBe(true);
    });

    it('should not detect Tauri environment in browser', () => {
      delete (window as any).__TAURI__;

      expect(service.isTauriEnvironment()).toBe(false);
    });

    it('should expose isTauri as computed signal', () => {
      expect(service.isTauri()).toBeDefined();
    });
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  describe('initial state', () => {
    it('should start with idle status', () => {
      expect(service.status()).toBe('idle');
    });

    it('should have no current session initially', () => {
      expect(service.currentSession()).toBeNull();
    });

    it('should not be authenticated initially', () => {
      expect(service.isAuthenticated()).toBe(false);
    });

    it('should not need login initially', () => {
      expect(service.needsLogin()).toBe(false);
    });
  });

  // ==========================================================================
  // Initialization
  // ==========================================================================

  describe('initialize', () => {
    it('should skip initialization in non-Tauri environment', async () => {
      delete (window as any).__TAURI__;

      await service.initialize();

      expect(service.status()).toBe('idle');
    });

    it('should check for existing session on initialization', async () => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;

      fetchSpy.and.returnValue(Promise.resolve({
        ok: true,
        status: 200,
        json: () => Promise.resolve(mockSession),
      } as Response));

      await service.initialize();

      expect(service.status()).toBe('authenticated');
      expect(service.currentSession()).toEqual(mockSession);
      expect(mockAuthService.setTauriSession).toHaveBeenCalledWith(mockSession);
    });

    it('should set needs_login when no session exists', async () => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;

      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 404,
      } as Response));

      await service.initialize();

      expect(service.status()).toBe('needs_login');
      expect(service.currentSession()).toBeNull();
    });

    it('should handle initialization errors', async () => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;

      // getActiveSession() catches errors internally and returns null,
      // so initialize() will set status to 'needs_login', not 'error'
      fetchSpy.and.returnValue(Promise.reject(new Error('Network error')));

      await service.initialize();

      expect(service.status()).toBe('needs_login');
    });

    it('should set up event listeners', async () => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;

      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 404,
      } as Response));

      await service.initialize();

      expect(mockTauriWindow.__TAURI__.event.listen).toHaveBeenCalledWith(
        'oauth-callback',
        jasmine.any(Function)
      );

      expect(mockTauriWindow.__TAURI__.event.listen).toHaveBeenCalledWith(
        'deep-link-error',
        jasmine.any(Function)
      );
    });
  });

  // ==========================================================================
  // Get Active Session
  // ==========================================================================

  describe('getActiveSession', () => {
    it('should fetch active session from storage', async () => {
      fetchSpy.and.returnValue(Promise.resolve({
        ok: true,
        status: 200,
        json: () => Promise.resolve(mockSession),
      } as Response));

      const session = await service.getActiveSession();

      expect(session).toEqual(mockSession);
      expect(fetchSpy).toHaveBeenCalledWith('http://localhost:8090/session');
    });

    it('should return null when no session exists (404)', async () => {
      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 404,
      } as Response));

      const session = await service.getActiveSession();

      expect(session).toBeNull();
    });

    it('should return null on network errors', async () => {
      fetchSpy.and.returnValue(Promise.reject(new Error('Network error')));

      const session = await service.getActiveSession();

      expect(session).toBeNull();
    });

    it('should handle non-404 errors', async () => {
      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 500,
      } as Response));

      const session = await service.getActiveSession();

      // Should return null and not throw
      expect(session).toBeNull();
    });
  });

  // ==========================================================================
  // OAuth Callback Handling
  // ==========================================================================

  describe('handleOAuthCallback', () => {
    const mockTokenResponse = {
      access_token: 'access-token-123',
      token_type: 'Bearer',
      expires_in: 3600,
    };

    const mockHandoffResponse = {
      humanId: 'human-123',
      identifier: 'user@example.com',
      doorwayId: 'doorway-1',
      doorwayUrl: 'https://doorway.example.com',
      displayName: 'Test User',
    };

    beforeEach(() => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;
    });

    it('should handle OAuth callback successfully', async () => {
      // Mock fetch responses in order
      fetchSpy.and.returnValues(
        // 1. initialize() → getActiveSession() (no existing session)
        Promise.resolve({
          ok: false,
          status: 404,
        } as Response),
        // 2. Token exchange
        Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockTokenResponse),
        } as Response),
        // 3. Native handoff
        Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockHandoffResponse),
        } as Response),
        // 4. Create session
        Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockSession),
        } as Response)
      );

      // Set up event listener and capture the callback
      let oauthCallbackHandler: any;
      mockTauriWindow.__TAURI__.event.listen.and.callFake((event: string, handler: any) => {
        if (event === 'oauth-callback') {
          oauthCallbackHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      await service.initialize();

      // Simulate OAuth callback event
      await oauthCallbackHandler({
        payload: {
          code: 'auth-code-123',
          state: 'test-state',
          url: 'elohim://auth/callback?code=auth-code-123&state=test-state',
        },
      });

      expect(service.status()).toBe('authenticated');
      expect(service.currentSession()).toBeTruthy();
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad']);
    });

    it('should handle token exchange failure', async () => {
      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 400,
        text: () => Promise.resolve('Invalid grant'),
      } as Response));

      let oauthCallbackHandler: any;
      mockTauriWindow.__TAURI__.event.listen.and.callFake((event: string, handler: any) => {
        if (event === 'oauth-callback') {
          oauthCallbackHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      await service.initialize();

      await oauthCallbackHandler({
        payload: {
          code: 'invalid-code',
          state: 'test-state',
          url: 'elohim://auth/callback',
        },
      });

      expect(service.status()).toBe('error');
      expect(service.errorMessage()).toContain('Token exchange failed');
    });

    it('should handle missing doorway selection', async () => {
      (mockDoorwayRegistry.selected as jasmine.Spy).and.returnValue(null);

      let oauthCallbackHandler: any;
      mockTauriWindow.__TAURI__.event.listen.and.callFake((event: string, handler: any) => {
        if (event === 'oauth-callback') {
          oauthCallbackHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      await service.initialize();

      await oauthCallbackHandler({
        payload: {
          code: 'auth-code-123',
          state: 'test-state',
          url: 'elohim://auth/callback',
        },
      });

      expect(service.status()).toBe('error');
      expect(service.errorMessage()).toContain('No doorway selected');
    });
  });

  // ==========================================================================
  // Deep Link Error Handling
  // ==========================================================================

  describe('deep link error handling', () => {
    beforeEach(() => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;
    });

    it('should handle deep link errors', async () => {
      let errorHandler: any;
      mockTauriWindow.__TAURI__.event.listen.and.callFake((event: string, handler: any) => {
        if (event === 'deep-link-error') {
          errorHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 404,
      } as Response));

      await service.initialize();

      // Simulate deep link error
      errorHandler({
        payload: {
          message: 'Invalid deep link',
          url: 'elohim://invalid',
        },
      });

      expect(service.status()).toBe('error');
      expect(service.errorMessage()).toBe('Invalid deep link');
    });
  });

  // ==========================================================================
  // Logout
  // ==========================================================================

  describe('logout', () => {
    it('should delete session and redirect to identity page', async () => {
      fetchSpy.and.returnValue(Promise.resolve({
        ok: true,
      } as Response));

      service.currentSession.set(mockSession);
      service.status.set('authenticated');

      await service.logout();

      expect(fetchSpy).toHaveBeenCalledWith('http://localhost:8090/session', {
        method: 'DELETE',
      });
      expect(service.currentSession()).toBeNull();
      expect(service.status()).toBe('needs_login');
      expect(mockAuthService.logout).toHaveBeenCalled();
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity']);
    });

    it('should continue logout even if session deletion fails', async () => {
      fetchSpy.and.returnValue(Promise.reject(new Error('Network error')));

      service.currentSession.set(mockSession);
      service.status.set('authenticated');

      await service.logout();

      expect(service.currentSession()).toBeNull();
      expect(service.status()).toBe('needs_login');
    });
  });

  // ==========================================================================
  // Navigate to Login
  // ==========================================================================

  describe('navigateToLogin', () => {
    it('should navigate to identity page', () => {
      service.navigateToLogin();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity']);
    });
  });

  // ==========================================================================
  // Cleanup
  // ==========================================================================

  describe('destroy', () => {
    beforeEach(() => {
      (window as any).__TAURI__ = mockTauriWindow.__TAURI__;
    });

    it('should unsubscribe from event listeners', async () => {
      const unsubscribeOAuth = jasmine.createSpy('unsubscribeOAuth');
      const unsubscribeError = jasmine.createSpy('unsubscribeError');

      mockTauriWindow.__TAURI__.event.listen.and.returnValues(
        Promise.resolve(unsubscribeOAuth),
        Promise.resolve(unsubscribeError)
      );

      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 404,
      } as Response));

      await service.initialize();

      service.destroy();

      expect(unsubscribeOAuth).toHaveBeenCalled();
      expect(unsubscribeError).toHaveBeenCalled();
    });

    it('should not error if listeners were not set up', () => {
      expect(() => service.destroy()).not.toThrow();
    });
  });

  // ==========================================================================
  // Storage URL Configuration
  // ==========================================================================

  describe('storage URL configuration', () => {
    it('should use default storage URL', async () => {
      fetchSpy.and.returnValue(Promise.resolve({
        ok: false,
        status: 404,
      } as Response));

      await service.getActiveSession();

      expect(fetchSpy).toHaveBeenCalledWith('http://localhost:8090/session');
    });

    // TODO(test-generator): [MEDIUM] Storage URL is hardcoded to localhost:8090
    // Context: getStorageUrl() returns environment.client?.storageUrl with localhost fallback
    // Story: Support configurable storage URLs for different deployment contexts
    // Suggested approach:
    //   1. Make storageUrl configurable via environment files
    //   2. Support dynamic discovery via Tauri config
    //   3. Add URL validation and health check
  });
});
