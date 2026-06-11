/**
 * doorway-app SessionTokenStore adapter for `DoorwaySessionClient`.
 *
 * doorway-app has only ever persisted ONE auth artifact: the raw JWT under
 * the localStorage key `doorway_auth_token` (read directly by the functional
 * auth interceptor in core/http/auth.interceptor.ts on every HttpClient
 * request). This adapter maps the client's `StoredSession` onto that SAME
 * single key — raw token value, no JSON envelope — so existing sessions
 * survive the migration and the interceptor keeps working unchanged.
 *
 * No platform guard: doorway-app is browser-only (no @angular/ssr), so
 * localStorage is always available.
 *
 * Shape note: session metadata (humanId, agentPubKey, identifier, expiresAt)
 * is NOT persisted — it never was in this app. `get()` therefore
 * reconstructs a minimal `StoredSession` with ''-sentinels and
 * `expiresAt: 0`. Consequence: `DoorwaySessionClient.restoreSession()`
 * (the expiry-checked local restore) is NOT usable with this adapter — a
 * 0 expiry would always read as expired and clear the token.
 * AuthStateService instead validates the session by probing
 * GET /auth/account, exactly as it always has.
 */

import { Injectable } from '@angular/core';

import type { SessionTokenStore, StoredSession } from '@elohim/identity';

/** The legacy localStorage key — shared contract with auth.interceptor.ts. */
export const AUTH_TOKEN_KEY = 'doorway_auth_token';

@Injectable({ providedIn: 'root' })
export class DoorwaySessionTokenStore implements SessionTokenStore {
  get(): StoredSession | null {
    const token = localStorage.getItem(AUTH_TOKEN_KEY);
    if (!token) {
      return null;
    }
    return {
      token,
      // Never persisted by doorway-app — '' marks an absent value and
      // expiresAt 0 marks "unknown; validate via /auth/account probe".
      humanId: '',
      agentPubKey: '',
      identifier: '',
      expiresAt: 0,
    };
  }

  set(session: StoredSession): void {
    // Persist exactly what doorway-app has always persisted: the raw JWT.
    localStorage.setItem(AUTH_TOKEN_KEY, session.token);
  }

  clear(): void {
    localStorage.removeItem(AUTH_TOKEN_KEY);
  }

  /**
   * Legacy write path for components that receive a bare JWT from their own
   * login/register POSTs (threshold-login / threshold-register via
   * `AuthStateService.storeToken`). Same key, same raw-token value.
   */
  setToken(token: string): void {
    localStorage.setItem(AUTH_TOKEN_KEY, token);
  }
}
