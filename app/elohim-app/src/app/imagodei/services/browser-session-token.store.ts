/**
 * Browser SessionTokenStore adapter for `DoorwaySessionClient`.
 *
 * Maps the client's `StoredSession` onto the SAME individual localStorage
 * keys AuthService has always used (`AUTH_TOKEN_KEY` etc.), so existing
 * sessions survive the migration and the other readers of those keys
 * (login.component identifier prefill, holochain-client doorway token)
 * keep working unchanged.
 *
 * SSR guard: elohim-app is SSR-rendered (`@angular/ssr`). On the server
 * platform this adapter delegates to the client's in-memory default store
 * and NEVER touches localStorage.
 *
 * Storage-shape note (migrate-on-read): `AUTH_EXPIRY_KEY` historically
 * stored `String(expiresAt)` in whatever shape the auth provider returned
 * (ms epoch, unix seconds, or ISO string). Reads go through
 * `parseExpiryDate` (accepts all three); writes are canonicalized to a
 * unix-seconds string (the `StoredSession.expiresAt` contract). No code
 * outside this pillar reads `AUTH_EXPIRY_KEY`, so legacy values are
 * migrated transparently on the next read/write cycle.
 */

import { isPlatformBrowser } from '@angular/common';
import { Injectable, PLATFORM_ID, inject } from '@angular/core';

import {
  InMemorySessionTokenStore,
  type SessionTokenStore,
  type StoredSession,
} from '@elohim/identity';

import { SIGNING_CREDENTIALS_KEY } from '../../elohim/models/holochain-connection.model';
import {
  type AuthProviderType,
  AUTH_TOKEN_KEY,
  AUTH_PROVIDER_KEY,
  AUTH_EXPIRY_KEY,
  AUTH_IDENTIFIER_KEY,
  AUTH_HUMAN_ID_KEY,
  AUTH_AGENT_PUB_KEY_KEY,
  AUTH_INSTALLED_APP_ID_KEY,
  parseExpiryDate,
} from '../models/auth.model';
import { DOORWAY_CACHE_KEY } from '../models/doorway.model';

@Injectable({ providedIn: 'root' })
export class BrowserSessionTokenStore implements SessionTokenStore {
  private readonly isBrowser = isPlatformBrowser(inject(PLATFORM_ID));

  /** Server-path fallback — the client's in-memory default; no localStorage. */
  private readonly memory = new InMemorySessionTokenStore();
  private memoryProvider: AuthProviderType | null = null;

  get(): StoredSession | null {
    if (!this.isBrowser) {
      return this.memory.get();
    }

    const token = localStorage.getItem(AUTH_TOKEN_KEY);
    if (!token) {
      return null;
    }

    const expiresAt = parseExpiryDate(localStorage.getItem(AUTH_EXPIRY_KEY));

    return {
      token,
      // StoredSession requires strings; '' marks an absent value
      // (AuthService maps '' back to null when hydrating AuthState).
      humanId: localStorage.getItem(AUTH_HUMAN_ID_KEY) ?? '',
      agentPubKey: localStorage.getItem(AUTH_AGENT_PUB_KEY_KEY) ?? '',
      identifier: localStorage.getItem(AUTH_IDENTIFIER_KEY) ?? '',
      // Missing/unparseable expiry reads as 0 (already expired) — same
      // outcome as the pre-migration isTokenExpiringSoon(null) path:
      // the session is not restorable.
      expiresAt: expiresAt ? Math.floor(expiresAt.getTime() / 1000) : 0,
      installedAppId: localStorage.getItem(AUTH_INSTALLED_APP_ID_KEY) ?? undefined,
    };
  }

  set(session: StoredSession): void {
    if (!this.isBrowser) {
      this.memory.set(session);
      return;
    }

    localStorage.setItem(AUTH_TOKEN_KEY, session.token);
    localStorage.setItem(AUTH_EXPIRY_KEY, String(session.expiresAt));
    localStorage.setItem(AUTH_IDENTIFIER_KEY, session.identifier);
    // Matches the legacy persistAuth contract: absent values leave any
    // existing entries untouched (they are only replaced, never blanked).
    if (session.humanId) {
      localStorage.setItem(AUTH_HUMAN_ID_KEY, session.humanId);
    }
    if (session.agentPubKey) {
      localStorage.setItem(AUTH_AGENT_PUB_KEY_KEY, session.agentPubKey);
    }
    if (session.installedAppId) {
      localStorage.setItem(AUTH_INSTALLED_APP_ID_KEY, session.installedAppId);
    }
    // doorwayId/doorwayUrl are intentionally NOT persisted — no localStorage
    // key existed for them pre-migration and no consumer reads them here.
  }

  clear(): void {
    if (!this.isBrowser) {
      this.memory.clear();
      this.memoryProvider = null;
      return;
    }

    // Exactly the legacy clearPersistedAuth() key set.
    localStorage.removeItem(AUTH_TOKEN_KEY);
    localStorage.removeItem(AUTH_PROVIDER_KEY);
    localStorage.removeItem(AUTH_EXPIRY_KEY);
    localStorage.removeItem(AUTH_IDENTIFIER_KEY);
    localStorage.removeItem(AUTH_HUMAN_ID_KEY);
    localStorage.removeItem(AUTH_AGENT_PUB_KEY_KEY);
    localStorage.removeItem(AUTH_INSTALLED_APP_ID_KEY);
    localStorage.removeItem(SIGNING_CREDENTIALS_KEY);
    localStorage.removeItem(DOORWAY_CACHE_KEY);
  }

  /**
   * Provider type rides alongside the session under the legacy
   * `AUTH_PROVIDER_KEY`. Provider selection is a service-edge concern, so
   * it is not part of `StoredSession` — the service reads/writes it here.
   */
  getProviderType(): AuthProviderType | null {
    if (!this.isBrowser) {
      return this.memoryProvider;
    }
    return localStorage.getItem(AUTH_PROVIDER_KEY) as AuthProviderType | null;
  }

  setProviderType(provider: AuthProviderType): void {
    if (!this.isBrowser) {
      this.memoryProvider = provider;
      return;
    }
    localStorage.setItem(AUTH_PROVIDER_KEY, provider);
  }
}
