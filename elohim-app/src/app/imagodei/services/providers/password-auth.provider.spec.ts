/**
 * Password Authentication Provider Tests
 */

import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { PasswordAuthProvider } from './password-auth.provider';
import { DoorwayRegistryService } from '../doorway-registry.service';
import { environment } from '../../../../environments/environment';
import type {
  PasswordCredentials,
  RegisterCredentials,
  AuthResponse,
} from '../../models/auth.model';

describe('PasswordAuthProvider', () => {
  let provider: PasswordAuthProvider;
  let httpMock: HttpTestingController;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;

  const mockAuthResponse: AuthResponse = {
    token: 'jwt-token-123',
    humanId: 'human-123',
    agentPubKey: 'agent-pub-key-123',
    expiresAt: Date.now() + 3600000,
    identifier: 'test@example.com',
  };

  beforeEach(() => {
    mockDoorwayRegistry = jasmine.createSpyObj('DoorwayRegistryService', [], {
      selectedUrl: jasmine.createSpy().and.returnValue(null),
    });

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        PasswordAuthProvider,
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
      ],
    });

    provider = TestBed.inject(PasswordAuthProvider);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // ==========================================================================
  // Provider Type Tests
  // ==========================================================================

  describe('type', () => {
    it('should have type "password"', () => {
      expect(provider.type).toBe('password');
    });
  });

  // ==========================================================================
  // Login Tests
  // ==========================================================================

  describe('login', () => {
    const validCredentials: PasswordCredentials = {
      type: 'password',
      identifier: 'test@example.com',
      password: 'password123',
    };

    it('should return error for invalid credentials type', async () => {
      const invalidCredentials = { type: 'oauth' as const, identifier: 'test' };

      const result = await provider.login(invalidCredentials as never);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.code).toBe('VALIDATION_ERROR');
      }
    });

    it('should make POST request to /auth/login', async () => {
      const loginPromise = provider.login(validCredentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual({
        identifier: 'test@example.com',
        password: 'password123',
      });

      req.flush(mockAuthResponse);

      const result = await loginPromise;
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.token).toBe('jwt-token-123');
        expect(result.humanId).toBe('human-123');
        expect(result.agentPubKey).toBe('agent-pub-key-123');
      }
    });

    it('should use selected doorway URL when available', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(
        'https://my-doorway.example.com'
      );

      const loginPromise = provider.login(validCredentials);

      const req = httpMock.expectOne('https://my-doorway.example.com/auth/login');
      req.flush(mockAuthResponse);

      await loginPromise;
    });

    it('should handle 401 error', async () => {
      const loginPromise = provider.login(validCredentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush({ error: 'Invalid credentials' }, { status: 401, statusText: 'Unauthorized' });

      const result = await loginPromise;
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.code).toBe('INVALID_CREDENTIALS');
      }
    });

    it('should handle 500 server error', async () => {
      const loginPromise = provider.login(validCredentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush({ error: 'Internal server error' }, { status: 500, statusText: 'Server Error' });

      const result = await loginPromise;
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.code).toBe('NETWORK_ERROR');
      }
    });

    it('should handle network error', async () => {
      const loginPromise = provider.login(validCredentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.error(new ProgressEvent('error'));

      const result = await loginPromise;
      expect(result.success).toBe(false);
    });
  });

  // ==========================================================================
  // Register Tests
  // ==========================================================================

  describe('register', () => {
    const registerCredentials: RegisterCredentials = {
      identifier: 'new@example.com',
      identifierType: 'email',
      password: 'password123',
      displayName: 'New User',
      affinities: ['learning'],
      profileReach: 'community',
    };

    it('should make POST request to /auth/register', async () => {
      const registerPromise = provider.register(registerCredentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/register'));
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual(
        jasmine.objectContaining({
          identifier: 'new@example.com',
          identifierType: 'email',
          password: 'password123',
          displayName: 'New User',
        })
      );

      req.flush(mockAuthResponse);

      const result = await registerPromise;
      expect(result.success).toBe(true);
    });

    it('should handle 409 conflict (user exists)', async () => {
      const registerPromise = provider.register(registerCredentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/register'));
      req.flush({ error: 'User already exists' }, { status: 409, statusText: 'Conflict' });

      const result = await registerPromise;
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.code).toBe('USER_EXISTS');
      }
    });

    it('should include optional fields', async () => {
      const fullCredentials: RegisterCredentials = {
        ...registerCredentials,
        bio: 'A test bio',
        location: 'Test City',
        humanId: 'existing-human-123',
        agentPubKey: 'existing-agent-key',
      };

      const registerPromise = provider.register(fullCredentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/register'));
      expect(req.request.body.bio).toBe('A test bio');
      expect(req.request.body.location).toBe('Test City');
      expect(req.request.body.humanId).toBe('existing-human-123');

      req.flush(mockAuthResponse);
      await registerPromise;
    });
  });

  // ==========================================================================
  // Logout Tests
  // ==========================================================================

  describe('logout', () => {
    it('should resolve without making HTTP request', async () => {
      await provider.logout();
      httpMock.expectNone(req => req.url.includes('/auth'));
    });
  });

  // ==========================================================================
  // Refresh Token Tests
  // ==========================================================================

  describe('refreshToken', () => {
    it('should make POST request to /auth/refresh with Bearer token', async () => {
      const refreshPromise = provider.refreshToken('old-token');

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/refresh'));
      expect(req.request.method).toBe('POST');
      expect(req.request.headers.get('Authorization')).toBe('Bearer old-token');

      req.flush(mockAuthResponse);

      const result = await refreshPromise;
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.token).toBe('jwt-token-123');
      }
    });

    it('should handle refresh failure', async () => {
      const refreshPromise = provider.refreshToken('expired-token');

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/refresh'));
      req.flush({ error: 'Token expired' }, { status: 401, statusText: 'Unauthorized' });

      const result = await refreshPromise;
      expect(result.success).toBe(false);
    });
  });

  // ==========================================================================
  // Get Current User Tests
  // ==========================================================================

  describe('getCurrentUser', () => {
    it('should make GET request to /auth/me with Bearer token', async () => {
      const userPromise = provider.getCurrentUser('valid-token');

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/me'));
      expect(req.request.method).toBe('GET');
      expect(req.request.headers.get('Authorization')).toBe('Bearer valid-token');

      req.flush({
        humanId: 'human-123',
        agentPubKey: 'agent-123',
        identifier: 'test@example.com',
      });

      const result = await userPromise;
      expect(result).toEqual({
        humanId: 'human-123',
        agentPubKey: 'agent-123',
        identifier: 'test@example.com',
      });
    });

    it('should return null on error', async () => {
      const userPromise = provider.getCurrentUser('invalid-token');

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/me'));
      req.flush({ error: 'Invalid token' }, { status: 401, statusText: 'Unauthorized' });

      const result = await userPromise;
      expect(result).toBeNull();
    });
  });

  // ==========================================================================
  // URL Resolution Tests
  // ==========================================================================

  describe('URL resolution', () => {
    it('should prioritize selected doorway URL', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(
        'https://custom-doorway.com'
      );

      const loginPromise = provider.login({
        type: 'password',
        identifier: 'test@example.com',
        password: 'pass',
      });

      const req = httpMock.expectOne('https://custom-doorway.com/auth/login');
      req.flush(mockAuthResponse);

      await loginPromise;
    });

    it('should use environment authUrl as fallback', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(null);

      // The actual URL depends on environment config
      const loginPromise = provider.login({
        type: 'password',
        identifier: 'test@example.com',
        password: 'pass',
      });

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush(mockAuthResponse);

      await loginPromise;
    });
  });

  // ==========================================================================
  // Error Handling Tests
  // ==========================================================================

  describe('error handling', () => {
    const credentials: PasswordCredentials = {
      type: 'password',
      identifier: 'test@example.com',
      password: 'pass',
    };

    it('should return proper error for 400 Bad Request', async () => {
      const loginPromise = provider.login(credentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush({ error: 'Invalid request' }, { status: 400, statusText: 'Bad Request' });

      const result = await loginPromise;
      expect(result.success).toBe(false);
    });

    it('should return proper error for 403 Forbidden', async () => {
      const loginPromise = provider.login(credentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush({ error: 'Access denied' }, { status: 403, statusText: 'Forbidden' });

      const result = await loginPromise;
      expect(result.success).toBe(false);
    });

    it('should return proper error for 404 Not Found', async () => {
      const loginPromise = provider.login(credentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush({ error: 'Not found' }, { status: 404, statusText: 'Not Found' });

      const result = await loginPromise;
      expect(result.success).toBe(false);
    });

    it('should return NOT_ENABLED for 501', async () => {
      const loginPromise = provider.login(credentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush({ error: 'Not implemented' }, { status: 501, statusText: 'Not Implemented' });

      const result = await loginPromise;
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.code).toBe('NOT_ENABLED');
      }
    });

    it('should use server error message when provided', async () => {
      const loginPromise = provider.login(credentials);

      const req = httpMock.expectOne(req => req.url.endsWith('/auth/login'));
      req.flush(
        { error: 'Custom server error', code: 'CUSTOM_CODE' },
        { status: 400, statusText: 'Bad Request' }
      );

      const result = await loginPromise;
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBe('Custom server error');
      }
    });
  });
});
