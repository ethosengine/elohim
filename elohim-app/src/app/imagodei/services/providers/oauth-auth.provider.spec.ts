/**
 * OAuthAuthProvider Tests
 *
 * Tests OAuth 2.0 Authorization Code flow for doorway authentication.
 */

import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';

import { OAuthAuthProvider } from './oauth-auth.provider';
import { DoorwayRegistryService } from '../doorway-registry.service';
import type { OAuthCredentials } from '../../models/auth.model';

describe('OAuthAuthProvider', () => {
  let provider: OAuthAuthProvider;
  let httpMock: HttpTestingController;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;

  const OAUTH_STATE_KEY = 'elohim-oauth-state';

  beforeEach(() => {
    // Clear sessionStorage
    sessionStorage.clear();

    // Create mock doorway registry
    mockDoorwayRegistry = jasmine.createSpyObj('DoorwayRegistryService', [], {
      selectedUrl: jasmine.createSpy().and.returnValue('https://doorway.example.com'),
    });

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        OAuthAuthProvider,
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
      ],
    });

    provider = TestBed.inject(OAuthAuthProvider);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
    sessionStorage.clear();
  });

  // ==========================================================================
  // Provider Type
  // ==========================================================================

  describe('provider type', () => {
    it('should have oauth type', () => {
      expect(provider.type).toBe('oauth');
    });
  });

  // ==========================================================================
  // Initiate Login
  // ==========================================================================

  // Skipped: These tests set window.location.href which causes Karma to navigate/hang
  xdescribe('initiateLogin', () => {
    let mockLocation: Partial<Location>;

    beforeEach(() => {
      mockLocation = { href: '', origin: 'http://localhost:4200' };
      Object.defineProperty(window, 'location', {
        value: mockLocation,
        writable: true,
        configurable: true,
      });
    });

    afterEach(() => {
      // No need to restore - each test gets a fresh window object
    });

    it('should redirect to doorway authorize endpoint', () => {
      const doorwayUrl = 'https://doorway.example.com';

      provider.initiateLogin(doorwayUrl);

      expect(window.location.href).toContain(`${doorwayUrl}/auth/authorize`);
      expect(window.location.href).toContain('clientId=elohim-app');
      expect(window.location.href).toContain('responseType=code');
      expect(window.location.href).toContain('state=');
    });

    it('should store OAuth state in sessionStorage', () => {
      const doorwayUrl = 'https://doorway.example.com';

      provider.initiateLogin(doorwayUrl);

      const storedState = sessionStorage.getItem(OAUTH_STATE_KEY);
      expect(storedState).toBeTruthy();

      const state = JSON.parse(storedState!);
      expect(state.doorwayUrl).toBe(doorwayUrl);
      expect(state.state).toBeTruthy();
      expect(state.timestamp).toBeTruthy();
    });

    it('should use custom return URL if provided', () => {
      const doorwayUrl = 'https://doorway.example.com';
      const returnUrl = 'http://localhost:4200/custom/callback';

      provider.initiateLogin(doorwayUrl, returnUrl);

      expect(window.location.href).toContain(`redirectUri=${encodeURIComponent(returnUrl)}`);

      const storedState = sessionStorage.getItem(OAUTH_STATE_KEY);
      const state = JSON.parse(storedState!);
      expect(state.redirectUri).toBe(returnUrl);
    });

    it('should set flow in progress flag', () => {
      expect(provider.isFlowInProgress()).toBe(false);

      provider.initiateLogin('https://doorway.example.com');

      expect(provider.isFlowInProgress()).toBe(true);
    });
  });

  // ==========================================================================
  // Handle Callback
  // ==========================================================================

  describe('handleCallback', () => {
    const mockTokenResponse = {
      accessToken: 'access-token-123',
      tokenType: 'Bearer',
      expiresIn: 3600,
      humanId: 'human-123',
      agentPubKey: 'agent-pub-key-123',
      identifier: 'user@example.com',
    };

    beforeEach(() => {
      // Set up valid OAuth state
      const state = {
        state: 'test-state-123',
        doorwayUrl: 'https://doorway.example.com',
        redirectUri: 'http://localhost:4200/auth/callback',
        timestamp: Date.now(),
      };
      sessionStorage.setItem(OAUTH_STATE_KEY, JSON.stringify(state));
    });

    it('should successfully exchange code for token', async () => {
      const result = provider.handleCallback('auth-code-123', 'test-state-123');

      const req = httpMock.expectOne('https://doorway.example.com/auth/token');
      expect(req.request.method).toBe('POST');
      expect(req.request.body.code).toBe('auth-code-123');
      expect(req.request.body.grantType).toBe('authorization_code');

      req.flush(mockTokenResponse);

      const authResult = await result;
      expect(authResult.success).toBe(true);
      if (authResult.success) {
        expect(authResult.token).toBe('access-token-123');
        expect(authResult.humanId).toBe('human-123');
        expect(authResult.agentPubKey).toBe('agent-pub-key-123');
        expect(authResult.identifier).toBe('user@example.com');
      }
    });

    it('should clear stored state on success', async () => {
      const result = provider.handleCallback('auth-code-123', 'test-state-123');

      const req = httpMock.expectOne('https://doorway.example.com/auth/token');
      req.flush(mockTokenResponse);

      await result;

      expect(sessionStorage.getItem(OAUTH_STATE_KEY)).toBeNull();
      expect(provider.isFlowInProgress()).toBe(false);
    });

    it('should fail when no stored state exists', async () => {
      sessionStorage.removeItem(OAUTH_STATE_KEY);

      const result = await provider.handleCallback('auth-code-123', 'test-state-123');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('OAuth session not found');
        expect(result.code).toBe('VALIDATION_ERROR');
      }
    });

    it('should fail when stored state is invalid JSON', async () => {
      sessionStorage.setItem(OAUTH_STATE_KEY, 'invalid-json');

      const result = await provider.handleCallback('auth-code-123', 'test-state-123');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('Invalid OAuth session');
        expect(result.code).toBe('VALIDATION_ERROR');
      }
      expect(sessionStorage.getItem(OAUTH_STATE_KEY)).toBeNull();
    });

    it('should fail when state mismatch (CSRF protection)', async () => {
      const result = await provider.handleCallback('auth-code-123', 'wrong-state');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('state mismatch');
        expect(result.code).toBe('VALIDATION_ERROR');
      }
      expect(sessionStorage.getItem(OAUTH_STATE_KEY)).toBeNull();
    });

    it('should fail when state is expired', async () => {
      const expiredState = {
        state: 'test-state-123',
        doorwayUrl: 'https://doorway.example.com',
        redirectUri: 'http://localhost:4200/auth/callback',
        timestamp: Date.now() - 11 * 60 * 1000, // 11 minutes ago
      };
      sessionStorage.setItem(OAUTH_STATE_KEY, JSON.stringify(expiredState));

      const result = await provider.handleCallback('auth-code-123', 'test-state-123');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('expired');
        expect(result.code).toBe('TOKEN_EXPIRED');
      }
    });

    it('should handle network errors', async () => {
      const result = provider.handleCallback('auth-code-123', 'test-state-123');

      const req = httpMock.expectOne('https://doorway.example.com/auth/token');
      req.error(new ProgressEvent('Network error'));

      const authResult = await result;
      expect(authResult.success).toBe(false);
      if (!authResult.success) {
        expect(authResult.code).toBe('NETWORK_ERROR');
      }
    });

    it('should handle 401 unauthorized', async () => {
      const result = provider.handleCallback('auth-code-123', 'test-state-123');

      const req = httpMock.expectOne('https://doorway.example.com/auth/token');
      req.flush({ error: 'invalid_grant' }, { status: 401, statusText: 'Unauthorized' });

      const authResult = await result;
      expect(authResult.success).toBe(false);
      if (!authResult.success) {
        expect(authResult.code).toBe('INVALID_CREDENTIALS');
      }
    });

    it('should handle OAuth-specific errors', async () => {
      const result = provider.handleCallback('auth-code-123', 'test-state-123');

      const req = httpMock.expectOne('https://doorway.example.com/auth/token');
      req.flush(
        {
          error: 'invalid_grant',
          errorDescription: 'The authorization code is invalid or expired',
        },
        { status: 400, statusText: 'Bad Request' }
      );

      const authResult = await result;
      expect(authResult.success).toBe(false);
      if (!authResult.success) {
        expect(authResult.error).toContain('invalid or expired');
      }
    });
  });

  // ==========================================================================
  // Direct Login (via credentials)
  // ==========================================================================

  describe('login', () => {
    it('should reject non-oauth credentials', async () => {
      const credentials = {
        type: 'password',
        identifier: 'user@example.com',
        password: 'password123',
      } as const;

      const result = await provider.login(credentials);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('Invalid credentials type');
      }
    });

    it('should exchange code when called with oauth credentials', async () => {
      const credentials: OAuthCredentials = {
        type: 'oauth',
        provider: 'https://doorway.example.com',
        token: 'auth-code-123', // authorization code
      };

      const mockTokenResponse = {
        accessToken: 'access-token-123',
        tokenType: 'Bearer',
        expiresIn: 3600,
        humanId: 'human-123',
        agentPubKey: 'agent-pub-key-123',
        identifier: 'user@example.com',
      };

      const resultPromise = provider.login(credentials);

      const req = httpMock.expectOne('https://doorway.example.com/auth/token');
      req.flush(mockTokenResponse);

      const result = await resultPromise;
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.token).toBe('access-token-123');
      }
    });
  });

  // ==========================================================================
  // Logout
  // ==========================================================================

  describe('logout', () => {
    it('should clear OAuth state from sessionStorage', async () => {
      sessionStorage.setItem(OAUTH_STATE_KEY, JSON.stringify({ state: 'test' }));
      provider.isFlowInProgress.set(true);

      await provider.logout();

      expect(sessionStorage.getItem(OAUTH_STATE_KEY)).toBeNull();
      expect(provider.isFlowInProgress()).toBe(false);
    });
  });

  // ==========================================================================
  // Refresh Token
  // ==========================================================================

  describe('refreshToken', () => {
    it('should call doorway refresh endpoint', async () => {
      const mockRefreshResponse = {
        accessToken: 'refreshed-token-456',
        tokenType: 'Bearer',
        expiresIn: 3600,
        humanId: 'human-123',
        agentPubKey: 'agent-pub-key-123',
        identifier: 'user@example.com',
      };

      const resultPromise = provider.refreshToken('old-token-123');

      const req = httpMock.expectOne('https://doorway.example.com/auth/refresh');
      expect(req.request.method).toBe('POST');
      expect(req.request.headers.get('Authorization')).toBe('Bearer old-token-123');

      req.flush(mockRefreshResponse);

      const result = await resultPromise;
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.token).toBe('refreshed-token-456');
      }
    });

    it('should fail when no doorway selected', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(null);

      const result = await provider.refreshToken('old-token-123');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('No doorway selected');
      }
    });

    it('should handle refresh errors', async () => {
      const resultPromise = provider.refreshToken('old-token-123');

      const req = httpMock.expectOne('https://doorway.example.com/auth/refresh');
      req.flush({ error: 'invalid_token' }, { status: 401, statusText: 'Unauthorized' });

      const result = await resultPromise;
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.code).toBe('TOKEN_EXPIRED');
      }
    });
  });

  // ==========================================================================
  // Callback Detection
  // ==========================================================================

  // Skipped: These tests manipulate window.location which causes Karma to hang
  xdescribe('callback detection', () => {
    let originalLocation: Location;

    beforeEach(() => {
      originalLocation = window.location;
      sessionStorage.setItem(
        OAUTH_STATE_KEY,
        JSON.stringify({ state: 'test-state', doorwayUrl: 'https://doorway.example.com' })
      );
    });

    afterEach(() => {
      window.location = originalLocation;
    });

    it('should detect pending callback from URL params', () => {
      delete (window as { location?: Location }).location;
      window.location = {
        href: 'http://localhost:4200/auth/callback?code=abc123&state=test-state',
      } as Location;

      expect(provider.hasPendingCallback()).toBe(true);
    });

    it('should not detect callback without code param', () => {
      delete (window as { location?: Location }).location;
      window.location = {
        href: 'http://localhost:4200/auth/callback?state=test-state',
      } as Location;

      expect(provider.hasPendingCallback()).toBe(false);
    });

    it('should not detect callback without state param', () => {
      delete (window as { location?: Location }).location;
      window.location = {
        href: 'http://localhost:4200/auth/callback?code=abc123',
      } as Location;

      expect(provider.hasPendingCallback()).toBe(false);
    });

    it('should not detect callback without stored state', () => {
      sessionStorage.removeItem(OAUTH_STATE_KEY);
      delete (window as { location?: Location }).location;
      window.location = {
        href: 'http://localhost:4200/auth/callback?code=abc123&state=test-state',
      } as Location;

      expect(provider.hasPendingCallback()).toBe(false);
    });

    it('should extract callback params from URL', () => {
      delete (window as { location?: Location }).location;
      window.location = {
        href: 'http://localhost:4200/auth/callback?code=abc123&state=test-state',
      } as Location;

      const params = provider.getCallbackParams();
      expect(params).toEqual({ code: 'abc123', state: 'test-state' });
    });

    it('should return null when error param present', () => {
      delete (window as { location?: Location }).location;
      window.location = {
        href: 'http://localhost:4200/auth/callback?error=access_denied',
      } as Location;

      const params = provider.getCallbackParams();
      expect(params).toBeNull();
    });

    it('should clear callback params from URL', () => {
      const mockReplaceState = jasmine.createSpy('replaceState');
      window.history.replaceState = mockReplaceState;

      delete (window as { location?: Location }).location;
      window.location = {
        href: 'http://localhost:4200/auth/callback?code=abc123&state=test-state',
      } as Location;

      provider.clearCallbackParams();

      expect(mockReplaceState).toHaveBeenCalled();
      const newUrl = mockReplaceState.calls.mostRecent().args[2];
      expect(newUrl).not.toContain('code=');
      expect(newUrl).not.toContain('state=');
    });
  });

  // ==========================================================================
  // State Generation
  // ==========================================================================

  // Skipped: Calls initiateLogin() which sets window.location.href, causing Karma to navigate
  xdescribe('state generation', () => {
    it('should generate unique state values', () => {
      const doorwayUrl = 'https://doorway.example.com';

      provider.initiateLogin(doorwayUrl);
      const state1Json = sessionStorage.getItem(OAUTH_STATE_KEY);
      const state1 = JSON.parse(state1Json!);

      sessionStorage.clear();

      provider.initiateLogin(doorwayUrl);
      const state2Json = sessionStorage.getItem(OAUTH_STATE_KEY);
      const state2 = JSON.parse(state2Json!);

      expect(state1.state).not.toBe(state2.state);
    });
  });
});
