/**
 * DoorwaySessionTokenStore Tests
 *
 * The contract under test: StoredSession maps onto the SAME single legacy
 * localStorage key (`doorway_auth_token`, raw JWT, no JSON envelope) that
 * the functional auth interceptor reads — sessions created before the
 * @elohim/identity migration must survive it.
 */

import { TestBed } from '@angular/core/testing';

import type { StoredSession } from '@elohim/identity';

import { AUTH_TOKEN_KEY, DoorwaySessionTokenStore } from './doorway-session-token.store';

describe('DoorwaySessionTokenStore', () => {
  let store: DoorwaySessionTokenStore;

  const sampleSession: StoredSession = {
    token: 'jwt-token-abc',
    humanId: 'human-1',
    agentPubKey: 'agent-key-1',
    identifier: 'operator@example.com',
    expiresAt: 1750000000,
  };

  beforeEach(() => {
    localStorage.clear();
    TestBed.configureTestingModule({});
    store = TestBed.inject(DoorwaySessionTokenStore);
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('should use the same key the auth interceptor reads', () => {
    expect(AUTH_TOKEN_KEY).toBe('doorway_auth_token');
  });

  it('should persist ONLY the raw token under the legacy key (no JSON envelope)', () => {
    store.set(sampleSession);

    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('jwt-token-abc');
    expect(localStorage.length).toBe(1);
  });

  it('should reconstruct a minimal session from a pre-migration stored token', () => {
    // A session written by the pre-migration storeToken path: bare JWT.
    localStorage.setItem(AUTH_TOKEN_KEY, 'legacy-jwt');

    expect(store.get()).toEqual({
      token: 'legacy-jwt',
      humanId: '',
      agentPubKey: '',
      identifier: '',
      expiresAt: 0,
    });
  });

  it('should return null when no token is stored', () => {
    expect(store.get()).toBeNull();
  });

  it('should remove the legacy key on clear', () => {
    store.set(sampleSession);

    store.clear();

    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
    expect(store.get()).toBeNull();
  });

  it('should write a bare JWT via setToken (legacy storeToken path)', () => {
    store.setToken('fresh-login-jwt');

    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('fresh-login-jwt');
    expect(store.get()?.token).toBe('fresh-login-jwt');
  });
});
