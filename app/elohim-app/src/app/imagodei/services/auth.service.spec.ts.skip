import { TestBed } from '@angular/core/testing';
import { AuthService } from './auth.service';
import { DoorwayRegistryService } from './doorway-registry.service';
import {
  AuthProvider,
  AuthCredentials,
  AuthResult,
  AUTH_TOKEN_KEY,
  AUTH_PROVIDER_KEY,
  AUTH_EXPIRY_KEY,
  AUTH_IDENTIFIER_KEY,
  RegisterCredentials,
} from '../models/auth.model';

describe('AuthService', () => {
  let service: AuthService;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;

  // Mock provider for testing
  const createMockProvider = (overrides: Partial<AuthProvider> = {}): AuthProvider => ({
    type: 'password',
    login: jasmine.createSpy('login').and.returnValue(Promise.resolve({
      success: true,
      token: 'test-token-123',
      humanId: 'human-abc',
      agentPubKey: 'agent-xyz',
      expiresAt: new Date(Date.now() + 3600000).toISOString(), // 1 hour
      identifier: 'test@example.com',
    } as AuthResult)),
    logout: jasmine.createSpy('logout').and.returnValue(Promise.resolve()),
    ...overrides,
  });

  beforeEach(() => {
    // Clear localStorage before each test
    localStorage.clear();

    mockDoorwayRegistry = jasmine.createSpyObj('DoorwayRegistryService', [], {
      selected: null,
      selectedUrl: null,
      hasSelection: false,
    });

    TestBed.configureTestingModule({
      providers: [
        AuthService,
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
      ],
    });

    service = TestBed.inject(AuthService);
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('should be created', () => {
    expect(service).toBeInstanceOf(AuthService);
  });

  describe('Initial State', () => {
    it('should start unauthenticated', () => {
      expect(service.isAuthenticated()).toBe(false);
      expect(service.token()).toBeNull();
      expect(service.humanId()).toBeNull();
      expect(service.agentPubKey()).toBeNull();
    });

    it('should not be loading initially', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should have no error initially', () => {
      expect(service.error()).toBeNull();
    });
  });

  describe('Provider Management', () => {
    it('should register a provider', () => {
      const mockProvider = createMockProvider();
      service.registerProvider(mockProvider);

      expect(service.hasProvider('password')).toBe(true);
      expect(service.getProvider('password')).toBe(mockProvider);
    });

    it('should return undefined for unregistered provider', () => {
      expect(service.getProvider('password')).toBeUndefined();
      expect(service.hasProvider('password')).toBe(false);
    });

    it('should allow multiple providers', () => {
      const passwordProvider = createMockProvider({ type: 'password' });
      const passkeyProvider = createMockProvider({ type: 'passkey' });

      service.registerProvider(passwordProvider);
      service.registerProvider(passkeyProvider);

      expect(service.hasProvider('password')).toBe(true);
      expect(service.hasProvider('passkey')).toBe(true);
    });
  });

  describe('Login', () => {
    it('should fail login if provider not registered', async () => {
      const credentials: AuthCredentials = {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      };

      const result = await service.login('password', credentials);

      expect(result.success).toBe(false);
      expect(result.code).toBe('NOT_ENABLED');
      expect(service.isAuthenticated()).toBe(false);
    });

    it('should login successfully with valid credentials', async () => {
      const mockProvider = createMockProvider();
      service.registerProvider(mockProvider);

      const credentials: AuthCredentials = {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      };

      const result = await service.login('password', credentials);

      expect(result.success).toBe(true);
      expect(service.isAuthenticated()).toBe(true);
      expect(service.token()).toBe('test-token-123');
      expect(service.humanId()).toBe('human-abc');
      expect(service.agentPubKey()).toBe('agent-xyz');
      expect(service.identifier()).toBe('test@example.com');
    });

    it('should persist auth to localStorage on successful login', async () => {
      const mockProvider = createMockProvider();
      service.registerProvider(mockProvider);

      const credentials: AuthCredentials = {
        type: 'password',
        identifier: 'test@example.com',
        password: 'password123',
      };

      await service.login('password', credentials);

      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('test-token-123');
      expect(localStorage.getItem(AUTH_PROVIDER_KEY)).toBe('password');
      expect(localStorage.getItem(AUTH_IDENTIFIER_KEY)).toBe('test@example.com');
      expect(localStorage.getItem(AUTH_EXPIRY_KEY)).not.toBeNull();
    });

    it('should set loading state during login', async () => {
      let wasLoading = false;
      const mockProvider = createMockProvider({
        login: jasmine.createSpy('login').and.callFake(async () => {
          wasLoading = service.isLoading();
          return {
            success: true,
            token: 'test-token',
            humanId: 'human-abc',
            agentPubKey: 'agent-xyz',
            expiresAt: new Date(Date.now() + 3600000).toISOString(),
            identifier: 'test@example.com',
          };
        }),
      });

      service.registerProvider(mockProvider);
      await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'pass' });

      expect(wasLoading).toBe(true);
      expect(service.isLoading()).toBe(false);
    });

    it('should handle login failure', async () => {
      const mockProvider = createMockProvider({
        login: jasmine.createSpy('login').and.returnValue(Promise.resolve({
          success: false,
          error: 'Invalid credentials',
          code: 'INVALID_CREDENTIALS',
        })),
      });

      service.registerProvider(mockProvider);
      const result = await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'wrong' });

      expect(result.success).toBe(false);
      expect(service.isAuthenticated()).toBe(false);
      expect(service.error()).toBe('Invalid credentials');
    });

    it('should handle network errors during login', async () => {
      const mockProvider = createMockProvider({
        login: jasmine.createSpy('login').and.returnValue(Promise.reject(new Error('Network error'))),
      });

      service.registerProvider(mockProvider);
      const result = await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'pass' });

      expect(result.success).toBe(false);
      expect(result.code).toBe('NETWORK_ERROR');
      expect(service.error()).toBe('Network error');
    });
  });

  describe('Registration', () => {
    it('should fail registration if provider not registered', async () => {
      const credentials: RegisterCredentials = {
        identifier: 'test@example.com',
        identifierType: 'email',
        password: 'password123',
        humanId: 'human-abc',
        agentPubKey: 'agent-xyz',
      };

      const result = await service.register('password', credentials);

      expect(result.success).toBe(false);
      expect(result.code).toBe('NOT_ENABLED');
    });

    it('should fail if provider does not support registration', async () => {
      const mockProvider = createMockProvider();
      // Remove register method
      delete (mockProvider as any).register;

      service.registerProvider(mockProvider);

      const credentials: RegisterCredentials = {
        identifier: 'test@example.com',
        identifierType: 'email',
        password: 'password123',
        humanId: 'human-abc',
        agentPubKey: 'agent-xyz',
      };

      const result = await service.register('password', credentials);

      expect(result.success).toBe(false);
      expect(result.code).toBe('NOT_ENABLED');
    });

    it('should register successfully with valid credentials', async () => {
      const mockProvider = createMockProvider({
        register: jasmine.createSpy('register').and.returnValue(Promise.resolve({
          success: true,
          token: 'new-token-456',
          humanId: 'human-new',
          agentPubKey: 'agent-new',
          expiresAt: new Date(Date.now() + 3600000).toISOString(),
          identifier: 'new@example.com',
        })),
      });

      service.registerProvider(mockProvider);

      const credentials: RegisterCredentials = {
        identifier: 'new@example.com',
        identifierType: 'email',
        password: 'password123',
        humanId: 'human-new',
        agentPubKey: 'agent-new',
      };

      const result = await service.register('password', credentials);

      expect(result.success).toBe(true);
      expect(service.isAuthenticated()).toBe(true);
      expect(service.token()).toBe('new-token-456');
    });
  });

  describe('Logout', () => {
    it('should clear authentication state on logout', async () => {
      const mockProvider = createMockProvider();
      service.registerProvider(mockProvider);

      // Login first
      await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'pass' });
      expect(service.isAuthenticated()).toBe(true);

      // Logout
      await service.logout();

      expect(service.isAuthenticated()).toBe(false);
      expect(service.token()).toBeNull();
      expect(service.humanId()).toBeNull();
    });

    it('should clear localStorage on logout', async () => {
      const mockProvider = createMockProvider();
      service.registerProvider(mockProvider);

      await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'pass' });
      await service.logout();

      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
      expect(localStorage.getItem(AUTH_PROVIDER_KEY)).toBeNull();
      expect(localStorage.getItem(AUTH_EXPIRY_KEY)).toBeNull();
      expect(localStorage.getItem(AUTH_IDENTIFIER_KEY)).toBeNull();
    });

    it('should call provider logout', async () => {
      const mockProvider = createMockProvider();
      service.registerProvider(mockProvider);

      await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'pass' });
      await service.logout();

      expect(mockProvider.logout).toHaveBeenCalled();
    });
  });

  describe('Session Restoration', () => {
    it('should restore session from localStorage', () => {
      const futureExpiry = new Date(Date.now() + 3600000).toISOString();
      localStorage.setItem(AUTH_TOKEN_KEY, 'stored-token');
      localStorage.setItem(AUTH_PROVIDER_KEY, 'password');
      localStorage.setItem(AUTH_EXPIRY_KEY, futureExpiry);
      localStorage.setItem(AUTH_IDENTIFIER_KEY, 'stored@example.com');

      // Create new service to trigger restoreSession in constructor
      const newService = TestBed.inject(AuthService);

      expect(newService.isAuthenticated()).toBe(true);
      expect(newService.token()).toBe('stored-token');
      expect(newService.provider()).toBe('password');
      expect(newService.identifier()).toBe('stored@example.com');
    });

    it('should not restore expired session', () => {
      const pastExpiry = new Date(Date.now() - 3600000).toISOString(); // 1 hour ago
      localStorage.setItem(AUTH_TOKEN_KEY, 'expired-token');
      localStorage.setItem(AUTH_PROVIDER_KEY, 'password');
      localStorage.setItem(AUTH_EXPIRY_KEY, pastExpiry);

      service.restoreSession();

      expect(service.isAuthenticated()).toBe(false);
      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
    });

    it('should return false if no stored session', () => {
      const result = service.restoreSession();
      expect(result).toBe(false);
    });

    it('should detect stored session', () => {
      localStorage.setItem(AUTH_TOKEN_KEY, 'some-token');

      expect(service.hasStoredSession()).toBe(true);

      localStorage.removeItem(AUTH_TOKEN_KEY);
      expect(service.hasStoredSession()).toBe(false);
    });
  });

  describe('Token Refresh', () => {
    it('should fail refresh if not authenticated', async () => {
      const result = await service.refreshToken();

      expect(result.success).toBe(false);
      expect(result.code).toBe('TOKEN_EXPIRED');
    });

    it('should fail if provider does not support refresh', async () => {
      const mockProvider = createMockProvider();
      // Remove refreshToken method
      delete (mockProvider as any).refreshToken;

      service.registerProvider(mockProvider);
      await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'pass' });

      const result = await service.refreshToken();

      expect(result.success).toBe(false);
      expect(result.code).toBe('NOT_ENABLED');
    });

    it('should refresh token successfully', async () => {
      const mockProvider = createMockProvider({
        refreshToken: jasmine.createSpy('refreshToken').and.returnValue(Promise.resolve({
          success: true,
          token: 'refreshed-token-789',
          humanId: 'human-abc',
          agentPubKey: 'agent-xyz',
          expiresAt: new Date(Date.now() + 7200000).toISOString(), // 2 hours
          identifier: 'test@example.com',
        })),
      });

      service.registerProvider(mockProvider);
      await service.login('password', { type: 'password', identifier: 'test@example.com', password: 'pass' });

      const result = await service.refreshToken();

      expect(result.success).toBe(true);
      expect(service.token()).toBe('refreshed-token-789');
    });
  });

  describe('Error Handling', () => {
    it('should clear error', () => {
      // Manually set an error state
      service['updateState']({ error: 'Test error' });
      expect(service.error()).toBe('Test error');

      service.clearError();
      expect(service.error()).toBeNull();
    });
  });

  describe('Doorway Signals', () => {
    it('should expose doorway signals from registry', () => {
      expect(service.selectedDoorway).toBeDefined();
      expect(service.doorwayUrl).toBeDefined();
      expect(service.hasDoorway).toBeDefined();
    });
  });
});
