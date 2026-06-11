/**
 * BrowserSessionTokenStore Tests
 *
 * Covers both platform paths:
 * - browser: StoredSession mapped onto the legacy per-key localStorage layout
 * - server (SSR): in-memory fallback, asserting zero localStorage access
 */

import { PLATFORM_ID } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';

import type { StoredSession } from '@elohim/identity';

import { SIGNING_CREDENTIALS_KEY } from '../../elohim/models/holochain-connection.model';
import {
  AUTH_TOKEN_KEY,
  AUTH_PROVIDER_KEY,
  AUTH_EXPIRY_KEY,
  AUTH_IDENTIFIER_KEY,
  AUTH_HUMAN_ID_KEY,
  AUTH_AGENT_PUB_KEY_KEY,
  AUTH_INSTALLED_APP_ID_KEY,
} from '../models/auth.model';
import { DOORWAY_CACHE_KEY } from '../models/doorway.model';

import { BrowserSessionTokenStore } from './browser-session-token.store';

describe('BrowserSessionTokenStore', () => {
  let localStorageMock: Record<string, string>;

  const sampleSession: StoredSession = {
    token: 'jwt-token-abc',
    humanId: 'human-1',
    agentPubKey: 'agent-key-1',
    identifier: 'user@example.com',
    expiresAt: 1750000000, // unix seconds
    installedAppId: 'elohim-app-7',
  };

  beforeEach(() => {
    localStorageMock = {};
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(
      (key: string) => localStorageMock[key] ?? null
    );
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation((key: string, value: string) => {
      localStorageMock[key] = value;
    });
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation((key: string) => {
      delete localStorageMock[key];
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ==========================================================================
  // Browser platform
  // ==========================================================================

  describe('browser platform', () => {
    let store: BrowserSessionTokenStore;

    beforeEach(() => {
      TestBed.configureTestingModule({});
      store = TestBed.inject(BrowserSessionTokenStore);
    });

    it('should persist the session under the legacy localStorage keys', () => {
      store.set(sampleSession);

      expect(localStorageMock[AUTH_TOKEN_KEY]).toBe('jwt-token-abc');
      expect(localStorageMock[AUTH_EXPIRY_KEY]).toBe('1750000000');
      expect(localStorageMock[AUTH_IDENTIFIER_KEY]).toBe('user@example.com');
      expect(localStorageMock[AUTH_HUMAN_ID_KEY]).toBe('human-1');
      expect(localStorageMock[AUTH_AGENT_PUB_KEY_KEY]).toBe('agent-key-1');
      expect(localStorageMock[AUTH_INSTALLED_APP_ID_KEY]).toBe('elohim-app-7');
    });

    it('should round-trip a stored session', () => {
      store.set(sampleSession);

      expect(store.get()).toEqual(sampleSession);
    });

    it('should return null when no token is stored', () => {
      localStorageMock[AUTH_EXPIRY_KEY] = '1750000000';
      localStorageMock[AUTH_IDENTIFIER_KEY] = 'user@example.com';

      expect(store.get()).toBeNull();
    });

    it('should read legacy millisecond expiry values (migrate-on-read)', () => {
      const msExpiry = 1750000000000; // ms epoch — the pre-migration stored shape
      localStorageMock[AUTH_TOKEN_KEY] = 'jwt-token-abc';
      localStorageMock[AUTH_EXPIRY_KEY] = String(msExpiry);

      expect(store.get()?.expiresAt).toBe(1750000000);
    });

    it('should read legacy ISO expiry values (migrate-on-read)', () => {
      localStorageMock[AUTH_TOKEN_KEY] = 'jwt-token-abc';
      localStorageMock[AUTH_EXPIRY_KEY] = '2026-01-13T10:15:47Z';

      expect(store.get()?.expiresAt).toBe(Math.floor(Date.parse('2026-01-13T10:15:47Z') / 1000));
    });

    it('should treat missing expiry as already expired', () => {
      localStorageMock[AUTH_TOKEN_KEY] = 'jwt-token-abc';

      expect(store.get()?.expiresAt).toBe(0);
    });

    it('should mark absent humanId/agentPubKey/identifier with empty strings', () => {
      localStorageMock[AUTH_TOKEN_KEY] = 'jwt-token-abc';
      localStorageMock[AUTH_EXPIRY_KEY] = '1750000000';

      const stored = store.get();

      expect(stored?.humanId).toBe('');
      expect(stored?.agentPubKey).toBe('');
      expect(stored?.identifier).toBe('');
      expect(stored?.installedAppId).toBeUndefined();
    });

    it('should leave existing optional entries untouched when the new session omits them', () => {
      localStorageMock[AUTH_HUMAN_ID_KEY] = 'stale-human';
      localStorageMock[AUTH_AGENT_PUB_KEY_KEY] = 'stale-agent';

      store.set({ ...sampleSession, humanId: '', agentPubKey: '', installedAppId: undefined });

      expect(localStorageMock[AUTH_HUMAN_ID_KEY]).toBe('stale-human');
      expect(localStorageMock[AUTH_AGENT_PUB_KEY_KEY]).toBe('stale-agent');
      expect(AUTH_INSTALLED_APP_ID_KEY in localStorageMock).toBe(false);
    });

    it('should clear the full legacy key set', () => {
      store.set(sampleSession);
      store.setProviderType('password');
      localStorageMock[SIGNING_CREDENTIALS_KEY] = '{"creds":true}';
      localStorageMock[DOORWAY_CACHE_KEY] = '{"doorway":true}';

      store.clear();

      expect(localStorageMock).toEqual({});
    });

    it('should store and read the provider type', () => {
      expect(store.getProviderType()).toBeNull();

      store.setProviderType('password');

      expect(localStorageMock[AUTH_PROVIDER_KEY]).toBe('password');
      expect(store.getProviderType()).toBe('password');
    });
  });

  // ==========================================================================
  // Server platform (SSR)
  // ==========================================================================

  describe('server platform (SSR)', () => {
    let store: BrowserSessionTokenStore;

    beforeEach(() => {
      TestBed.configureTestingModule({
        providers: [{ provide: PLATFORM_ID, useValue: 'server' }],
      });
      store = TestBed.inject(BrowserSessionTokenStore);
    });

    it('should never touch localStorage on the server path', () => {
      store.set(sampleSession);
      store.setProviderType('password');
      store.get();
      store.getProviderType();
      store.clear();

      expect(Storage.prototype.getItem).not.toHaveBeenCalled();
      expect(Storage.prototype.setItem).not.toHaveBeenCalled();
      expect(Storage.prototype.removeItem).not.toHaveBeenCalled();
    });

    it('should fall back to in-memory session behavior', () => {
      expect(store.get()).toBeNull();
      expect(store.getProviderType()).toBeNull();

      store.set(sampleSession);
      store.setProviderType('password');

      expect(store.get()).toEqual(sampleSession);
      expect(store.getProviderType()).toBe('password');

      store.clear();

      expect(store.get()).toBeNull();
      expect(store.getProviderType()).toBeNull();
    });
  });
});
