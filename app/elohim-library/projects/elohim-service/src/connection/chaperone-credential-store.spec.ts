/**
 * The device signing credential is persisted and REUSED — because every fresh
 * keypair costs a permanent Holochain CapGrant that the conductor then re-reads
 * on every zome call (11 619 / 15 768 / 18 516 rows on matthew and adam,
 * 2026-09-19; ~47 000 SQL queries and 4–9 s per call).
 */

import { beforeEach, describe, expect, it } from 'vitest';

import {
  CHAPERONE_CREDENTIALS_KEY,
  CHAPERONE_CREDENTIAL_SCOPE_KEY,
  ChaperoneCredentialStore,
  agentFromDoorwayToken,
  looksLikeCapGrantRejection,
  type ChaperoneCredentialScope,
  type ChaperoneCredentials,
} from './chaperone-credential-store';

// ---------------------------------------------------------------------------
// A localStorage stand-in. The library targets Node (CLI) and browser alike,
// so the ambient `localStorage` is genuinely absent under vitest's node
// environment — which is also the "storage unavailable" case under test.
// ---------------------------------------------------------------------------

interface FakeStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

function installStorage(overrides: Partial<FakeStorage> = {}): Map<string, string> {
  const map = new Map<string, string>();
  const storage: FakeStorage = {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
    removeItem: (key: string) => void map.delete(key),
    ...overrides,
  };
  (globalThis as Record<string, unknown>).localStorage = storage;
  return map;
}

function removeStorage(): void {
  delete (globalThis as Record<string, unknown>).localStorage;
}

const SCOPE: ChaperoneCredentialScope = {
  origin: 'https://doorway-alpha.elohim.host',
  agent: 'uhCAkAgentOne',
};

function credentials(seed = 1): ChaperoneCredentials {
  return {
    capSecret: new Uint8Array(64).fill(seed),
    signingKey: new Uint8Array(39).fill(seed + 1),
    keyPair: {
      keyType: 'ed25519',
      publicKey: new Uint8Array(32).fill(seed + 2),
      privateKey: new Uint8Array(64).fill(seed + 3),
    },
  };
}

function base64url(value: object): string {
  return Buffer.from(JSON.stringify(value))
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

describe('ChaperoneCredentialStore', () => {
  beforeEach(() => {
    removeStorage();
  });

  /**
   * THE INVARIANT. What the browser saves is exactly what it presents on the
   * next connect — byte for byte. Anything less and the doorway sees an unknown
   * key and authors another grant.
   */
  it('round-trips a credential byte for byte', () => {
    installStorage();
    const store = new ChaperoneCredentialStore();
    const original = credentials();

    expect(store.save(SCOPE, original)).toBe(true);
    const restored = store.load(SCOPE);

    expect(restored).not.toBeNull();
    expect(restored!.capSecret).toEqual(original.capSecret);
    expect(restored!.signingKey).toEqual(original.signingKey);
    expect(restored!.keyPair.publicKey).toEqual(original.keyPair.publicKey);
    expect(restored!.keyPair.privateKey).toEqual(original.keyPair.privateKey);
    expect(restored!.keyPair.keyType).toBe('ed25519');
  });

  /**
   * REUSE ACROSS A RELOAD. A new page load is a NEW store instance over the
   * SAME storage — the one shape that matters, because a page load is what used
   * to mint three grants.
   */
  it('a fresh store instance over the same storage reuses the credential', () => {
    installStorage();
    const original = credentials(7);
    new ChaperoneCredentialStore().save(SCOPE, original);

    // Simulated reload: brand new instance, same browser.
    const afterReload = new ChaperoneCredentialStore().load(SCOPE);

    expect(afterReload).not.toBeNull();
    expect(afterReload!.signingKey).toEqual(original.signingKey);
  });

  /** A different doorway must not be handed a grant minted by another. */
  it('refuses a credential stored for a different doorway origin', () => {
    installStorage();
    new ChaperoneCredentialStore().save(SCOPE, credentials());

    const other = new ChaperoneCredentialStore().load({
      origin: 'https://doorway-b.elohim.host',
      agent: SCOPE.agent,
    });
    expect(other).toBeNull();
  });

  /** Two hosted humans sharing a browser profile never share a credential. */
  it('refuses a credential stored for a different agent', () => {
    installStorage();
    new ChaperoneCredentialStore().save(SCOPE, credentials());

    const other = new ChaperoneCredentialStore().load({
      origin: SCOPE.origin,
      agent: 'uhCAkAgentTwo',
    });
    expect(other).toBeNull();
  });

  /**
   * The credential record's key is shared with the app's own store (that is
   * what makes sign-out clear it). If another writer replaces it, the scope's
   * fingerprint no longer matches and the store refuses rather than presenting
   * a key its scope does not describe.
   */
  it('refuses a credential another writer replaced under the shared key', () => {
    const map = installStorage();
    new ChaperoneCredentialStore().save(SCOPE, credentials(1));

    // The native admin-WS path writes the same key with its own credential.
    const foreign = credentials(9);
    map.set(
      CHAPERONE_CREDENTIALS_KEY,
      JSON.stringify({
        capSecret: Buffer.from(foreign.capSecret).toString('base64'),
        keyPair: {
          publicKey: Buffer.from(foreign.keyPair.publicKey).toString('base64'),
          privateKey: Buffer.from(foreign.keyPair.privateKey).toString('base64'),
        },
        signingKey: Buffer.from(foreign.signingKey).toString('base64'),
      })
    );

    expect(new ChaperoneCredentialStore().load(SCOPE)).toBeNull();
  });

  /**
   * SIGN-OUT. Clearing removes the credential AND its scope, so the next
   * sign-in mints a fresh key and receives one fresh grant.
   */
  it('clear removes both records', () => {
    const map = installStorage();
    const store = new ChaperoneCredentialStore();
    store.save(SCOPE, credentials());
    expect(map.has(CHAPERONE_CREDENTIALS_KEY)).toBe(true);
    expect(map.has(CHAPERONE_CREDENTIAL_SCOPE_KEY)).toBe(true);

    store.clear();

    expect(map.has(CHAPERONE_CREDENTIALS_KEY)).toBe(false);
    expect(map.has(CHAPERONE_CREDENTIAL_SCOPE_KEY)).toBe(false);
    expect(store.load(SCOPE)).toBeNull();
  });

  /**
   * SIGN-OUT VIA THE APP. The consuming app's session store removes only the
   * credential record's key (it does not know about the scope record). The
   * store must read that as "no credential" and leave the stale scope inert —
   * otherwise sign-out would not actually forget the device key.
   */
  it('treats an app-cleared credential as absent, leaving the scope inert', () => {
    const map = installStorage();
    const store = new ChaperoneCredentialStore();
    store.save(SCOPE, credentials());

    // Exactly what `browser-session-token.store.ts::clear()` does.
    map.delete(CHAPERONE_CREDENTIALS_KEY);

    expect(store.load(SCOPE)).toBeNull();
    expect(map.has(CHAPERONE_CREDENTIAL_SCOPE_KEY)).toBe(true);

    // …and the next sign-in overwrites both with the fresh key.
    const fresh = credentials(33);
    store.save(SCOPE, fresh);
    expect(store.load(SCOPE)!.signingKey).toEqual(fresh.signingKey);
  });

  /**
   * THE HEAL, from the store's side: after `clear()` the next connect finds
   * nothing and generates a fresh keypair, which the doorway does not recognise
   * and therefore grants. One round trip, no loop.
   */
  it('after a heal the device presents a different key', () => {
    installStorage();
    const store = new ChaperoneCredentialStore();
    const first = credentials(1);
    store.save(SCOPE, first);

    store.clear();
    expect(store.load(SCOPE)).toBeNull();

    const healed = credentials(50);
    store.save(SCOPE, healed);
    expect(store.load(SCOPE)!.signingKey).toEqual(healed.signingKey);
    expect(store.load(SCOPE)!.signingKey).not.toEqual(first.signingKey);
  });

  /**
   * The constructor seam: a host (or a test) can supply its own store instead
   * of the browser global. Nothing about the scoping or fingerprint rules
   * changes.
   */
  it('uses an injected store in preference to the global', () => {
    removeStorage();
    const map = new Map<string, string>();
    const injected = {
      getItem: (key: string) => map.get(key) ?? null,
      setItem: (key: string, value: string) => void map.set(key, value),
      removeItem: (key: string) => void map.delete(key),
    };
    const store = new ChaperoneCredentialStore(injected);
    const original = credentials(21);

    expect(store.save(SCOPE, original)).toBe(true);
    expect(map.has(CHAPERONE_CREDENTIALS_KEY)).toBe(true);
    expect(store.load(SCOPE)!.signingKey).toEqual(original.signingKey);

    // The global was never touched.
    expect((globalThis as Record<string, unknown>).localStorage).toBeUndefined();
  });

  // ── storage unavailable: degrade, never fail ─────────────────────────────

  /**
   * A private window, a blocked-storage policy, or SSR must degrade to the
   * previous in-memory behaviour — a fresh key per session — and NEVER throw
   * into the connect path.
   */
  it('degrades silently when storage is absent', () => {
    removeStorage();
    const store = new ChaperoneCredentialStore();

    expect(store.load(SCOPE)).toBeNull();
    expect(store.save(SCOPE, credentials())).toBe(false);
    expect(() => store.clear()).not.toThrow();
  });

  it('degrades silently when storage throws on write (quota / private mode)', () => {
    installStorage({
      setItem: () => {
        throw new Error('QuotaExceededError');
      },
    });
    const store = new ChaperoneCredentialStore();

    expect(store.save(SCOPE, credentials())).toBe(false);
    expect(store.load(SCOPE)).toBeNull();
  });

  it('degrades silently when storage throws on read', () => {
    installStorage({
      getItem: () => {
        throw new Error('SecurityError');
      },
    });
    expect(new ChaperoneCredentialStore().load(SCOPE)).toBeNull();
  });

  it('ignores a corrupt or truncated record rather than throwing', () => {
    const map = installStorage();
    const store = new ChaperoneCredentialStore();
    store.save(SCOPE, credentials());

    map.set(CHAPERONE_CREDENTIALS_KEY, '{not json');
    expect(store.load(SCOPE)).toBeNull();

    map.set(CHAPERONE_CREDENTIALS_KEY, JSON.stringify({ capSecret: 'only-this' }));
    expect(store.load(SCOPE)).toBeNull();
  });
});

describe('agentFromDoorwayToken', () => {
  /** The scope's agent half comes from the session token already in hand. */
  it('reads agent_pub_key from a doorway JWT payload', () => {
    const token = `header.${base64url({ agent_pub_key: 'uhCAkAgentOne', sub: 'x' })}.signature`;
    expect(agentFromDoorwayToken(token)).toBe('uhCAkAgentOne');
  });

  /**
   * Unparseable input yields `null`, which the strategy reads as "do not
   * persist" — an explicit refusal to guess a slot, never a thrown connect.
   */
  it('returns null rather than throwing on anything unusable', () => {
    expect(agentFromDoorwayToken(undefined)).toBeNull();
    expect(agentFromDoorwayToken(null)).toBeNull();
    expect(agentFromDoorwayToken('')).toBeNull();
    expect(agentFromDoorwayToken('not-a-jwt')).toBeNull();
    expect(agentFromDoorwayToken('header.@@@not-base64@@@.sig')).toBeNull();
    expect(agentFromDoorwayToken(`header.${base64url({ sub: 'x' })}.sig`)).toBeNull();
    expect(agentFromDoorwayToken(`header.${base64url({ agent_pub_key: 7 })}.sig`)).toBeNull();
  });
});

describe('looksLikeCapGrantRejection', () => {
  /** What the bounded heal listens for. */
  it('recognises the conductor saying our grant is gone', () => {
    for (const message of [
      'Zome call failed: Unauthorized',
      'InvalidCommit: CapabilityCheckFailed',
      'no cap grant found for secret',
      'invalid cap secret',
    ]) {
      expect(looksLikeCapGrantRejection(new Error(message))).toBe(true);
    }
  });

  /**
   * …and what it must not. Healing on an ordinary zome error would author a
   * capability grant for every validation failure — the growth this change
   * removes, wearing a different hat.
   */
  it('ignores ordinary zome errors', () => {
    for (const message of [
      "Zome function find_publishers doesn't exist",
      'validation failed: content too large',
      'source chain head has moved',
      'Zome call timed out after 10000ms',
    ]) {
      expect(looksLikeCapGrantRejection(new Error(message))).toBe(false);
    }
  });
});
