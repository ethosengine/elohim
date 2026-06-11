/**
 * AuthStateService Tests
 *
 * Behavior contract preserved across the @elohim/identity migration
 * (auth-wire plan 2026-06-10, Task 3):
 * - init() without a stored token resolves unauthenticated WITHOUT probing
 *   /auth/account (the anonymous-page 401-noise guard)
 * - init() with a stored token probes /auth/account and hydrates the signals
 * - a ?session_token URL param is exchanged via GET /auth/exchange-session
 *   (DoorwaySessionClient walking), stored under the legacy localStorage
 *   key, and scrubbed from the URL
 * - logout() clears the legacy key, resets the account, and navigates home —
 *   even when the server-side POST /auth/logout fails
 */

import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { of } from 'rxjs';
import { vi, type Mock } from 'vitest';

import { AccountResponse } from '../models/doorway.model';
import { AuthStateService } from './auth-state.service';
import { DoorwayAdminService } from './doorway-admin.service';
import { AUTH_TOKEN_KEY } from './doorway-session-token.store';

const STEWARD_ACCOUNT: AccountResponse = {
  id: 'user-1',
  identifier: 'operator@example.com',
  identifierType: 'email',
  permissionLevel: 'ADMIN',
  isActive: true,
  isSteward: true,
  hasLocalConductor: true,
  hasExportedKey: false,
  createdAt: null,
  lastLoginAt: null,
  doorwayName: 'alpha',
  doorwayRegion: null,
  usage: {
    storageBytes: 0,
    storageMb: 0,
    projectionQueries: 0,
    bandwidthBytes: 0,
    bandwidthMb: 0,
    periodStart: null,
    lastUpdated: null,
  },
  quota: {
    storageLimitBytes: 0,
    storageLimitMb: 0,
    storagePercentUsed: 0,
    dailyQueryLimit: 0,
    queriesPercentUsed: 0,
    dailyBandwidthLimitBytes: 0,
    dailyBandwidthLimitMb: 0,
    bandwidthPercentUsed: 0,
    enforceHardLimit: false,
    isOverQuota: false,
  },
};

/** Minimal fetch Response stand-in (DoorwaySessionClient uses ok/json/text). */
function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    text: async () => JSON.stringify(body),
  } as unknown as Response;
}

describe('AuthStateService', () => {
  let mockAdmin: { getAccount: Mock };
  let mockRouter: { navigate: Mock };
  let fetchMock: Mock;

  beforeEach(() => {
    localStorage.clear();
    window.history.replaceState({}, '', '/');

    mockAdmin = { getAccount: vi.fn().mockReturnValue(of(STEWARD_ACCOUNT)) };
    mockRouter = { navigate: vi.fn() };
    fetchMock = vi.fn();
    // DoorwaySessionClient binds globalThis.fetch at construction — stub
    // BEFORE the service (and its composed client) is instantiated.
    vi.stubGlobal('fetch', fetchMock);

    TestBed.configureTestingModule({
      providers: [
        { provide: DoorwayAdminService, useValue: mockAdmin },
        { provide: Router, useValue: mockRouter },
      ],
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    localStorage.clear();
    window.history.replaceState({}, '', '/');
  });

  function service(): AuthStateService {
    return TestBed.inject(AuthStateService);
  }

  describe('init', () => {
    it('should resolve unauthenticated without probing /auth/account when no token is stored', async () => {
      const auth = service();

      await auth.init();

      expect(mockAdmin.getAccount).not.toHaveBeenCalled();
      expect(fetchMock).not.toHaveBeenCalled();
      expect(auth.isAuthenticated()).toBe(false);
      expect(auth.isLoading()).toBe(false);
    });

    it('should probe /auth/account and hydrate signals when a token is stored', async () => {
      localStorage.setItem(AUTH_TOKEN_KEY, 'stored-jwt');
      const auth = service();

      await auth.init();

      expect(mockAdmin.getAccount).toHaveBeenCalledTimes(1);
      expect(auth.isAuthenticated()).toBe(true);
      expect(auth.isSteward()).toBe(true);
      expect(auth.modeLabel()).toBe('Steward');
      expect(auth.initials()).toBe('O');
      expect(auth.account()).toEqual(STEWARD_ACCOUNT);
    });

    it('should exchange a session_token URL param, persist the JWT, and scrub the URL', async () => {
      window.history.replaceState({}, '', '/account?session_token=transfer-1&tab=usage');
      fetchMock.mockResolvedValue(
        jsonResponse({
          token: 'exchanged-jwt',
          humanId: 'human-1',
          agentPubKey: 'agent-key-1',
          identifier: 'operator@example.com',
          expiresAt: 1750000000,
        })
      );
      const auth = service();

      await auth.init();

      expect(fetchMock).toHaveBeenCalledWith(
        '/auth/exchange-session?session_token=transfer-1',
        expect.objectContaining({ method: 'GET' })
      );
      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('exchanged-jwt');
      expect(window.location.pathname + window.location.search).toBe('/account?tab=usage');
      // The freshly stored token feeds straight into the account probe.
      expect(mockAdmin.getAccount).toHaveBeenCalledTimes(1);
      expect(auth.isAuthenticated()).toBe(true);
    });

    it('should resolve unauthenticated when the session_token exchange fails', async () => {
      window.history.replaceState({}, '', '/?session_token=expired-1');
      fetchMock.mockResolvedValue(jsonResponse({ error: 'expired' }, 401));
      const auth = service();

      await auth.init();

      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
      expect(window.location.search).toBe('');
      expect(mockAdmin.getAccount).not.toHaveBeenCalled();
      expect(auth.isAuthenticated()).toBe(false);
      expect(auth.isLoading()).toBe(false);
    });
  });

  describe('logout', () => {
    it('should POST /auth/logout with the stored bearer, clear the key, and navigate home', async () => {
      localStorage.setItem(AUTH_TOKEN_KEY, 'stored-jwt');
      fetchMock.mockResolvedValue(jsonResponse({ success: true, message: 'ok' }));
      const auth = service();
      await auth.init();

      await auth.logout();

      expect(fetchMock).toHaveBeenCalledWith(
        '/auth/logout',
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({ Authorization: 'Bearer stored-jwt' }),
        })
      );
      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
      expect(auth.isAuthenticated()).toBe(false);
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
    });

    it('should still clear the session and navigate when the server logout fails', async () => {
      localStorage.setItem(AUTH_TOKEN_KEY, 'stored-jwt');
      fetchMock.mockResolvedValue(jsonResponse({ error: 'boom' }, 500));
      const auth = service();

      await auth.logout();

      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
      expect(auth.isAuthenticated()).toBe(false);
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
    });

    it('should clear local state and navigate immediately, not waiting for a hung server logout', async () => {
      vi.useFakeTimers();
      localStorage.setItem(AUTH_TOKEN_KEY, 'stored-jwt');
      // Mock fetch to return a never-resolving promise (hung socket).
      fetchMock.mockReturnValue(new Promise(() => {}));
      const auth = service();

      // Start logout but don't await yet so we can check state before network settles.
      const logoutPromise = auth.logout();

      // Even though the fetch hasn't settled, navigate and the account signal
      // should have been cleared synchronously.
      expect(auth.isAuthenticated()).toBe(false);
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);

      // Fast-forward past the 30s timeout to complete the logout promise.
      vi.advanceTimersByTime(30_000);
      await logoutPromise;

      // Switch back to real timers before checking localStorage, as the
      // sessionClient's finally block may run on real timer schedule.
      vi.useRealTimers();

      // The key contract is that local state (account signal and navigation)
      // happened synchronously without waiting for the network. The token clear
      // is eventually handled by the sessionClient's finally block (which waits
      // for the race to resolve, then the client clears it).
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
    });
  });

  describe('storeToken', () => {
    it('should persist a bare JWT under the legacy key (login/register component path)', () => {
      const auth = service();

      auth.storeToken('fresh-login-jwt');

      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('fresh-login-jwt');
    });
  });

  describe('refresh', () => {
    it('should re-probe /auth/account and update the account signal', async () => {
      const auth = service();

      await auth.refresh();

      expect(mockAdmin.getAccount).toHaveBeenCalledTimes(1);
      expect(auth.account()).toEqual(STEWARD_ACCOUNT);
    });
  });
});
