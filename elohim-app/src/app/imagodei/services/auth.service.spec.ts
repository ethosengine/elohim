/**
 * Authentication Service Tests
 */

import { TestBed } from '@angular/core/testing';
import { AuthService } from './auth.service';
import { DoorwayRegistryService } from './doorway-registry.service';
import {
  type AuthProvider,
  type AuthCredentials,
  type AuthResult,
  type RegisterCredentials,
  type AuthProviderType,
  AUTH_TOKEN_KEY,
  AUTH_PROVIDER_KEY,
  AUTH_EXPIRY_KEY,
  AUTH_IDENTIFIER_KEY,
} from '../models/auth.model';

describe('AuthService', () => {
  let service: AuthService;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let localStorageMock: { [key: string]: string };

  // Mock provider factory
  const createMockProvider = (
    type: 'password' | 'oauth' = 'password',
    overrides: Partial<AuthProvider> = {}
  ): AuthProvider => ({
    type,
    login: jasmine.createSpy('login').and.returnValue(
      Promise.resolve({
        success: true,
        token: 'test-token-123',
        humanId: 'human-123',
        agentPubKey: 'agent-pub-key-123',
        expiresAt: Date.now() + 3600000, // 1 hour from now
        identifier: 'test@example.com',
      } as AuthResult)
    ),
    logout: jasmine.createSpy('logout').and.returnValue(Promise.resolve()),
    register: jasmine.createSpy('register').and.returnValue(
      Promise.resolve({
        success: true,
        token: 'test-token-456',
        humanId: 'human-456',
        agentPubKey: 'agent-pub-key-456',
        expiresAt: Date.now() + 3600000,
        identifier: 'newuser@example.com',
      } as AuthResult)
    ),
    refreshToken: jasmine.createSpy('refreshToken').and.returnValue(
      Promise.resolve({
        success: true,
        token: 'refreshed-token-789',
        humanId: 'human-123',
        agentPubKey: 'agent-pub-key-123',
        expiresAt: Date.now() + 3600000,
        identifier: 'test@example.com',
      } as AuthResult)
    ),
    ...overrides,
  });

  beforeEach(() => {
    // Setup localStorage mock
    localStorageMock = {};
    spyOn(localStorage, 'getItem').and.callFake((key: string) => localStorageMock[key] || null);
    spyOn(localStorage, 'setItem').and.callFake((key: string, value: string) => {
      localStorageMock[key] = value;
    });
    spyOn(localStorage, 'removeItem').and.callFake((key: string) => {
      delete localStorageMock[key];
    });

    // Create mock doorway registry
    mockDoorwayRegistry = jasmine.createSpyObj(
      'DoorwayRegistryService',
      ['selectDoorway', 'clearSelection'],
      {
        selected: jasmine.createSpy().and.returnValue(null),
        selectedUrl: jasmine.createSpy().and.returnValue(null),
        hasSelection: jasmine.createSpy().and.returnValue(false),
      }
    );

    TestBed.configureTestingModule({
      providers: [AuthService, { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry }],
    });

    service = TestBed.inject(AuthService);
  });

  // ==========================================================================
  // Initial State Tests
  // ==========================================================================

  describe('initial state', () => {
    it('should start not authenticated', () => {
      expect(service.isAuthenticated()).toBe(false);
    });

    it('should start with null token', () => {
      expect(service.token()).toBeNull();
    });

    it('should start with null humanId', () => {
      expect(service.humanId()).toBeNull();
    });

    it('should start with null agentPubKey', () => {
      expect(service.agentPubKey()).toBeNull();
    });

    it('should start not loading', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should start without error', () => {
      expect(service.error()).toBeNull();
    });

    it('should expose doorway signals from registry', () => {
      expect(service.selectedDoorway).toBeDefined();
      expect(service.doorwayUrl).toBeDefined();
      expect(service.hasDoorway).toBeDefined();
    });
  });

  // ==========================================================================
  // Provider Management Tests
  // ==========================================================================

  describe('provider management', () => {
    it('should register a provider', () => {
      const provider = createMockProvider('password');

      service.registerProvider(provider);

      expect(service.hasProvider('password')).toBe(true);
    });

    it('should retrieve registered provider', () => {
      const provider = createMockProvider('password');

      service.registerProvider(provider);
      const retrieved = service.getProvider('password');

      expect(retrieved).toBe(provider);
    });

    it('should return undefined for unregistered provider', () => {
      const retrieved = service.getProvider('oauth');

      expect(retrieved).toBeUndefined();
    });

    it('should check if provider is registered', () => {
      expect(service.hasProvider('password')).toBe(false);

      const provider = createMockProvider('password');
      service.registerProvider(provider);

      expect(service.hasProvider('password')).toBe(true);
    });

    it('should allow multiple providers', () => {
      const passwordProvider = createMockProvider('password');
      const oauthProvider = createMockProvider('oauth');

      service.registerProvider(passwordProvider);
      service.registerProvider(oauthProvider);

      expect(service.hasProvider('password')).toBe(true);
      expect(service.hasProvider('oauth')).toBe(true);
    });
  });

  // ==========================================================================
  // Login Tests
  // ==========================================================================

  describe('login', () => {
    it('should return error when provider not registered', async () => {
      const credentials: AuthCredentials = {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      };

      const result = await service.login('password', credentials);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('not registered');
        expect(result.code).toBe('NOT_ENABLED');
      }
    });

    it('should authenticate with valid credentials', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      const credentials: AuthCredentials = {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      };

      const result = await service.login('password', credentials);

      expect(result.success).toBe(true);
      expect(provider.login).toHaveBeenCalledWith(credentials);
      expect(service.isAuthenticated()).toBe(true);
      expect(service.token()).toBe('test-token-123');
      expect(service.humanId()).toBe('human-123');
      expect(service.agentPubKey()).toBe('agent-pub-key-123');
      expect(service.identifier()).toBe('test@example.com');
    });

    it('should set loading state during login', async () => {
      const provider = createMockProvider('password', {
        login: jasmine.createSpy('login').and.callFake(() => {
          expect(service.isLoading()).toBe(true);
          return Promise.resolve({
            success: true,
            token: 'token',
            humanId: 'id',
            agentPubKey: 'key',
            expiresAt: Date.now() + 3600000,
            identifier: 'test@example.com',
          });
        }),
      });
      service.registerProvider(provider);

      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      expect(service.isLoading()).toBe(false);
    });

    it('should handle login failure', async () => {
      const provider = createMockProvider('password', {
        login: jasmine
          .createSpy('login')
          .and.returnValue(Promise.resolve({ success: false, error: 'Invalid credentials' })),
      });
      service.registerProvider(provider);

      const result = await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'wrong',
      });

      expect(result.success).toBe(false);
      expect(service.isAuthenticated()).toBe(false);
      expect(service.error()).toBe('Invalid credentials');
    });

    it('should handle login network error', async () => {
      const provider = createMockProvider('password', {
        login: jasmine
          .createSpy('login')
          .and.returnValue(Promise.reject(new Error('Network error'))),
      });
      service.registerProvider(provider);

      const result = await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBe('Network error');
        expect(result.code).toBe('NETWORK_ERROR');
      }
      expect(service.error()).toBe('Network error');
    });

    it('should persist auth to localStorage on successful login', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      expect(localStorage.setItem).toHaveBeenCalledWith(AUTH_TOKEN_KEY, 'test-token-123');
      expect(localStorage.setItem).toHaveBeenCalledWith(AUTH_PROVIDER_KEY, 'password');
      expect(localStorage.setItem).toHaveBeenCalledWith(AUTH_IDENTIFIER_KEY, 'test@example.com');
    });
  });

  // ==========================================================================
  // Register Tests
  // ==========================================================================

  describe('register', () => {
    it('should return error when provider not registered', async () => {
      const credentials: RegisterCredentials = {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'password123',
        displayName: 'New User',
      };

      const result = await service.register('password', credentials);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('not registered');
        expect(result.code).toBe('NOT_ENABLED');
      }
    });

    it('should return error when provider does not support registration', async () => {
      const provider = createMockProvider('password', {
        register: undefined,
      });
      service.registerProvider(provider);

      const credentials: RegisterCredentials = {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'password123',
        displayName: 'New User',
      };

      const result = await service.register('password', credentials);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('does not support registration');
        expect(result.code).toBe('NOT_ENABLED');
      }
    });

    it('should register successfully with valid credentials', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      const credentials: RegisterCredentials = {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'password123',
        displayName: 'New User',
      };

      const result = await service.register('password', credentials);

      expect(result.success).toBe(true);
      expect(provider.register).toHaveBeenCalledWith(credentials);
      expect(service.isAuthenticated()).toBe(true);
      expect(service.token()).toBe('test-token-456');
      expect(service.humanId()).toBe('human-456');
    });

    it('should handle registration failure', async () => {
      const provider = createMockProvider('password', {
        register: jasmine
          .createSpy('register')
          .and.returnValue(Promise.resolve({ success: false, error: 'User already exists' })),
      });
      service.registerProvider(provider);

      const result = await service.register('password', {
        identifier: 'existing@example.com',
        identifierType: 'email',
        password: 'password123',
        displayName: 'Existing User',
      });

      expect(result.success).toBe(false);
      expect(service.error()).toBe('User already exists');
    });

    it('should handle registration network error', async () => {
      const provider = createMockProvider('password', {
        register: jasmine
          .createSpy('register')
          .and.returnValue(Promise.reject(new Error('Connection failed'))),
      });
      service.registerProvider(provider);

      const result = await service.register('password', {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'password123',
        displayName: 'New User',
      });

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBe('Connection failed');
        expect(result.code).toBe('NETWORK_ERROR');
      }
    });
  });

  // ==========================================================================
  // Logout Tests
  // ==========================================================================

  describe('logout', () => {
    it('should clear authentication state', async () => {
      // First login
      const provider = createMockProvider('password');
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      expect(service.isAuthenticated()).toBe(true);

      // Then logout
      await service.logout();

      expect(service.isAuthenticated()).toBe(false);
      expect(service.token()).toBeNull();
      expect(service.humanId()).toBeNull();
      expect(service.agentPubKey()).toBeNull();
      expect(service.identifier()).toBeNull();
    });

    it('should call provider logout', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      await service.logout();

      expect(provider.logout).toHaveBeenCalled();
    });

    it('should clear localStorage on logout', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      await service.logout();

      expect(localStorage.removeItem).toHaveBeenCalledWith(AUTH_TOKEN_KEY);
      expect(localStorage.removeItem).toHaveBeenCalledWith(AUTH_PROVIDER_KEY);
      expect(localStorage.removeItem).toHaveBeenCalledWith(AUTH_EXPIRY_KEY);
      expect(localStorage.removeItem).toHaveBeenCalledWith(AUTH_IDENTIFIER_KEY);
    });

    it('should handle provider logout error gracefully', async () => {
      const provider = createMockProvider('password', {
        logout: jasmine
          .createSpy('logout')
          .and.returnValue(Promise.reject(new Error('Logout failed'))),
      });
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      // Should not throw, just warn
      await expectAsync(service.logout()).toBeResolved();
      expect(service.isAuthenticated()).toBe(false);
    });
  });

  // ==========================================================================
  // Token Refresh Tests
  // ==========================================================================

  describe('refreshToken', () => {
    it('should return error when not authenticated', async () => {
      const result = await service.refreshToken();

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('No active session');
        expect(result.code).toBe('TOKEN_EXPIRED');
      }
    });

    it('should return error when provider does not support refresh', async () => {
      const provider = createMockProvider('password', {
        refreshToken: undefined,
      });
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      const result = await service.refreshToken();

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain('not supported');
        expect(result.code).toBe('NOT_ENABLED');
      }
    });

    it('should refresh token successfully', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      const result = await service.refreshToken();

      expect(result.success).toBe(true);
      expect(provider.refreshToken).toHaveBeenCalledWith('test-token-123');
      expect(service.token()).toBe('refreshed-token-789');
    });

    it('should handle refresh failure', async () => {
      const provider = createMockProvider('password', {
        refreshToken: jasmine
          .createSpy('refreshToken')
          .and.returnValue(Promise.reject(new Error('Refresh token expired'))),
      });
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      const result = await service.refreshToken();

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBe('Refresh token expired');
        expect(result.code).toBe('NETWORK_ERROR');
      }
    });
  });

  // ==========================================================================
  // Session Restoration Tests
  // ==========================================================================

  describe('restoreSession', () => {
    it('should return false when no stored session', () => {
      const result = service.restoreSession();

      expect(result).toBe(false);
      expect(service.isAuthenticated()).toBe(false);
    });

    it('should restore session from localStorage', () => {
      const futureExpiry = Date.now() + 3600000; // 1 hour from now
      localStorageMock[AUTH_TOKEN_KEY] = 'stored-token';
      localStorageMock[AUTH_PROVIDER_KEY] = 'password';
      localStorageMock[AUTH_EXPIRY_KEY] = String(futureExpiry);
      localStorageMock[AUTH_IDENTIFIER_KEY] = 'stored@example.com';

      const result = service.restoreSession();

      expect(result).toBe(true);
      expect(service.isAuthenticated()).toBe(true);
      expect(service.token()).toBe('stored-token');
      expect(service.identifier()).toBe('stored@example.com');
      expect(service.provider()).toBe('password');
    });

    it('should not restore expired session', () => {
      const pastExpiry = Date.now() - 3600000; // 1 hour ago
      localStorageMock[AUTH_TOKEN_KEY] = 'expired-token';
      localStorageMock[AUTH_PROVIDER_KEY] = 'password';
      localStorageMock[AUTH_EXPIRY_KEY] = String(pastExpiry);

      const result = service.restoreSession();

      expect(result).toBe(false);
      expect(service.isAuthenticated()).toBe(false);
      expect(localStorage.removeItem).toHaveBeenCalledWith(AUTH_TOKEN_KEY);
    });

    it('should return false when token missing', () => {
      localStorageMock[AUTH_PROVIDER_KEY] = 'password';
      localStorageMock[AUTH_EXPIRY_KEY] = String(Date.now() + 3600000);

      const result = service.restoreSession();

      expect(result).toBe(false);
    });

    it('should return false when provider missing', () => {
      localStorageMock[AUTH_TOKEN_KEY] = 'stored-token';
      localStorageMock[AUTH_EXPIRY_KEY] = String(Date.now() + 3600000);

      const result = service.restoreSession();

      expect(result).toBe(false);
    });
  });

  // ==========================================================================
  // hasStoredSession Tests
  // ==========================================================================

  describe('hasStoredSession', () => {
    it('should return false when no session stored', () => {
      expect(service.hasStoredSession()).toBe(false);
    });

    it('should return true when token is stored', () => {
      localStorageMock[AUTH_TOKEN_KEY] = 'some-token';

      expect(service.hasStoredSession()).toBe(true);
    });
  });

  // ==========================================================================
  // setAuthFromResult Tests
  // ==========================================================================

  describe('setAuthFromResult', () => {
    it('should set auth state from successful result', () => {
      const result: AuthResult = {
        success: true,
        token: 'external-token',
        humanId: 'human-ext',
        agentPubKey: 'agent-ext',
        expiresAt: Date.now() + 3600000,
        identifier: 'external@example.com',
      };

      service.setAuthFromResult(result);

      expect(service.isAuthenticated()).toBe(true);
      expect(service.token()).toBe('external-token');
      expect(service.humanId()).toBe('human-ext');
      expect(service.provider()).toBe('oauth'); // default provider type
    });

    it('should set error from failed result', () => {
      const result: AuthResult = {
        success: false,
        error: 'OAuth callback failed',
      };

      service.setAuthFromResult(result);

      expect(service.isAuthenticated()).toBe(false);
      expect(service.error()).toBe('OAuth callback failed');
    });

    it('should use specified provider type', () => {
      const result: AuthResult = {
        success: true,
        token: 'external-token',
        humanId: 'human-ext',
        agentPubKey: 'agent-ext',
        expiresAt: Date.now() + 3600000,
        identifier: 'external@example.com',
      };

      service.setAuthFromResult(result, 'password');

      expect(service.provider()).toBe('password');
    });
  });

  // ==========================================================================
  // setTauriSession Tests
  // ==========================================================================

  describe('setTauriSession', () => {
    it('should set Tauri session without token', () => {
      service.setTauriSession({
        humanId: 'tauri-human',
        agentPubKey: 'tauri-agent',
        doorwayUrl: 'http://localhost:8888',
        identifier: 'tauri@local',
      });

      expect(service.isAuthenticated()).toBe(true);
      expect(service.humanId()).toBe('tauri-human');
      expect(service.agentPubKey()).toBe('tauri-agent');
      expect(service.identifier()).toBe('tauri@local');
      expect(service.token()).toBeNull(); // Tauri sessions don't use JWT
      expect(service.provider()).toBe('tauri');
    });
  });

  // ==========================================================================
  // clearError Tests
  // ==========================================================================

  describe('clearError', () => {
    it('should clear error state', async () => {
      // Create an error
      const provider = createMockProvider('password', {
        login: jasmine
          .createSpy('login')
          .and.returnValue(Promise.resolve({ success: false, error: 'Test error' })),
      });
      service.registerProvider(provider);
      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'wrong',
      });

      expect(service.error()).toBe('Test error');

      service.clearError();

      expect(service.error()).toBeNull();
    });
  });

  // ==========================================================================
  // Signal and Computed Property Tests
  // ==========================================================================

  describe('signals and computed properties', () => {
    it('should expose auth signal as readonly', () => {
      const authSignal = service.auth;
      expect(authSignal).toBeDefined();
      // Verify it's readable
      const authState = authSignal();
      expect(authState).toBeDefined();
      expect(authState.isAuthenticated).toBe(false);
    });

    it('should expose isAuthenticated computed signal', () => {
      expect(service.isAuthenticated()).toBe(false);
    });

    it('should expose token computed signal', () => {
      expect(service.token()).toBeNull();
    });

    it('should expose humanId computed signal', () => {
      expect(service.humanId()).toBeNull();
    });

    it('should expose agentPubKey computed signal', () => {
      expect(service.agentPubKey()).toBeNull();
    });

    it('should expose identifier computed signal', () => {
      expect(service.identifier()).toBeNull();
    });

    it('should expose provider computed signal', () => {
      expect(service.provider()).toBeNull();
    });

    it('should expose isLoading computed signal', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should expose error computed signal', () => {
      expect(service.error()).toBeNull();
    });

    it('should expose expiresAt computed signal', () => {
      expect(service.expiresAt()).toBeNull();
    });

    it('should expose doorway signals from registry', () => {
      expect(service.selectedDoorway).toBeDefined();
      expect(service.doorwayUrl).toBeDefined();
      expect(service.hasDoorway).toBeDefined();
    });
  });

  // ==========================================================================
  // Method Existence Tests
  // ==========================================================================

  describe('method existence', () => {
    it('should have registerProvider method', () => {
      expect(typeof service.registerProvider).toBe('function');
    });

    it('should have getProvider method', () => {
      expect(typeof service.getProvider).toBe('function');
    });

    it('should have hasProvider method', () => {
      expect(typeof service.hasProvider).toBe('function');
    });

    it('should have login method', () => {
      expect(typeof service.login).toBe('function');
    });

    it('should have register method', () => {
      expect(typeof service.register).toBe('function');
    });

    it('should have logout method', () => {
      expect(typeof service.logout).toBe('function');
    });

    it('should have refreshToken method', () => {
      expect(typeof service.refreshToken).toBe('function');
    });

    it('should have restoreSession method', () => {
      expect(typeof service.restoreSession).toBe('function');
    });

    it('should have hasStoredSession method', () => {
      expect(typeof service.hasStoredSession).toBe('function');
    });

    it('should have clearError method', () => {
      expect(typeof service.clearError).toBe('function');
    });

    it('should have setAuthFromResult method', () => {
      expect(typeof service.setAuthFromResult).toBe('function');
    });

    it('should have setTauriSession method', () => {
      expect(typeof service.setTauriSession).toBe('function');
    });
  });

  // ==========================================================================
  // Observable/Signal Return Type Tests
  // ==========================================================================

  describe('return type verification', () => {
    it('login should return Promise<AuthResult>', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      const result = await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      expect(result).toBeDefined();
      expect(typeof result === 'object').toBe(true);
      expect('success' in result).toBe(true);
    });

    it('register should return Promise<AuthResult>', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      const result = await service.register('password', {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'password123',
        displayName: 'New User',
      });

      expect(result).toBeDefined();
      expect(typeof result === 'object').toBe(true);
      expect('success' in result).toBe(true);
    });

    it('logout should return Promise<void>', async () => {
      const logoutPromise = service.logout();
      expect(logoutPromise instanceof Promise).toBe(true);
      await logoutPromise;
    });

    it('refreshToken should return Promise<AuthResult>', async () => {
      const result = await service.refreshToken();
      expect(result).toBeDefined();
      expect(typeof result === 'object').toBe(true);
      expect('success' in result).toBe(true);
    });

    it('restoreSession should return boolean', () => {
      const result = service.restoreSession();
      expect(typeof result === 'boolean').toBe(true);
    });

    it('hasStoredSession should return boolean', () => {
      const result = service.hasStoredSession();
      expect(typeof result === 'boolean').toBe(true);
    });

    it('clearError should return undefined', () => {
      const result = service.clearError();
      expect(result).toBeUndefined();
    });

    it('setAuthFromResult should return undefined', () => {
      const result = service.setAuthFromResult({
        success: false,
        error: 'test',
      });
      expect(result).toBeUndefined();
    });

    it('setTauriSession should return undefined', () => {
      const result = service.setTauriSession({
        humanId: 'test',
        agentPubKey: 'test',
        doorwayUrl: 'http://test',
        identifier: 'test',
      });
      expect(result).toBeUndefined();
    });
  });

  // ==========================================================================
  // Input Parameter Tests
  // ==========================================================================

  describe('input parameter handling', () => {
    it('registerProvider should accept AuthProvider type', () => {
      const provider = createMockProvider('password');
      expect(() => service.registerProvider(provider)).not.toThrow();
    });

    it('getProvider should accept AuthProviderType', () => {
      expect(() => service.getProvider('password')).not.toThrow();
    });

    it('hasProvider should accept AuthProviderType', () => {
      expect(() => service.hasProvider('password')).not.toThrow();
    });

    it('login should accept provider type and credentials', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      const credentials: AuthCredentials = {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      };

      expect(() => service.login('password', credentials)).not.toThrow();
      await service.login('password', credentials);
    });

    it('register should accept provider type and registration credentials', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      const credentials: RegisterCredentials = {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'password123',
        displayName: 'New User',
      };

      expect(() => service.register('password', credentials)).not.toThrow();
      await service.register('password', credentials);
    });

    it('setAuthFromResult should accept AuthResult with optional provider type', () => {
      const result: AuthResult = {
        success: true,
        token: 'test-token',
        humanId: 'test-human',
        agentPubKey: 'test-agent',
        expiresAt: Date.now() + 3600000,
        identifier: 'test@example.com',
      };

      expect(() => service.setAuthFromResult(result)).not.toThrow();
      expect(() => service.setAuthFromResult(result, 'password')).not.toThrow();
    });

    it('setTauriSession should accept session object with required fields', () => {
      const session = {
        humanId: 'tauri-human',
        agentPubKey: 'tauri-agent',
        doorwayUrl: 'http://localhost:8888',
        identifier: 'tauri@local',
      };

      expect(() => service.setTauriSession(session)).not.toThrow();
    });

    it('setTauriSession should accept session object with optional displayName', () => {
      const session = {
        humanId: 'tauri-human',
        agentPubKey: 'tauri-agent',
        doorwayUrl: 'http://localhost:8888',
        identifier: 'tauri@local',
        displayName: 'Tauri User',
      };

      expect(() => service.setTauriSession(session)).not.toThrow();
    });
  });

  // ==========================================================================
  // State Mutation Tests
  // ==========================================================================

  describe('state mutation and synchronization', () => {
    it('should update all auth signals when state changes', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      // All signals should reflect the new state
      expect(service.isAuthenticated()).toBe(true);
      expect(service.token()).toBeTruthy();
      expect(service.humanId()).toBeTruthy();
      expect(service.agentPubKey()).toBeTruthy();
      expect(service.identifier()).toBe('test@example.com');
      expect(service.provider()).toBe('password');
      expect(service.isLoading()).toBe(false);
      expect(service.error()).toBeNull();
    });

    it('should maintain consistency between individual signals and auth signal', async () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      await service.login('password', {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      const fullAuth = service.auth();
      expect(fullAuth.isAuthenticated).toBe(service.isAuthenticated());
      expect(fullAuth.token).toBe(service.token());
      expect(fullAuth.humanId).toBe(service.humanId());
      expect(fullAuth.agentPubKey).toBe(service.agentPubKey());
      expect(fullAuth.identifier).toBe(service.identifier());
      expect(fullAuth.provider).toBe(service.provider());
      expect(fullAuth.isLoading).toBe(service.isLoading());
      expect(fullAuth.error).toBe(service.error());
    });
  });

  // ==========================================================================
  // Edge Case Tests
  // ==========================================================================

  describe('edge cases and boundary conditions', () => {
    it('should handle provider not registered for all auth operations', async () => {
      const loginResult = await service.login('nonexistent' as AuthProviderType, {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      });

      expect(loginResult.success).toBe(false);
      if (!loginResult.success) {
        expect(loginResult.code).toBe('NOT_ENABLED');
      }
    });

    it('should handle empty localStorage gracefully', () => {
      // localStorage should already be empty from setup
      const hasSession = service.hasStoredSession();
      expect(hasSession).toBe(false);
    });

    it('should preserve provider instance after registration', () => {
      const provider = createMockProvider('password');
      service.registerProvider(provider);

      const retrieved = service.getProvider('password');
      expect(retrieved).toBe(provider);
    });

    it('should not throw when clearing error with no prior error', () => {
      expect(() => service.clearError()).not.toThrow();
      expect(service.error()).toBeNull();
    });

    it('should accept null values in setAuthFromResult for failed results', () => {
      const result: AuthResult = {
        success: false,
        error: 'Auth failed',
      };

      expect(() => service.setAuthFromResult(result)).not.toThrow();
      expect(service.isAuthenticated()).toBe(false);
    });
  });

  // ==========================================================================
  // TODO: Async Flow Tests and Complex Mocking
  // ==========================================================================
  // TODO: Add async flow tests for token refresh scheduling and timeout management
  // TODO: Add comprehensive mocks for refresh timer behavior
  // TODO: Add business logic tests for expiration logic and refresh scheduling logic
});
