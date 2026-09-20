/**
 * Chaperone Credential Store
 *
 * Persists the browser's zome-call signing keypair and capability secret so a
 * device is granted ONCE rather than once per page load.
 *
 * ## Why this exists
 *
 * `POST /hc/connect` makes the doorway call `grant_zome_call_capability` for
 * each role cell. That is a Holochain chain write which nothing revokes, and
 * the conductor re-reads EVERY capability grant on EVERY zome call. On
 * 2026-09-19 the two conductors the public doorways front carried
 * 11 619 / 15 768 / 18 516 grants — ~47 000 SQL queries and 4–9 s per zome
 * call. The browser minting a fresh keypair and cap secret on every
 * `connectViaChaperone` is the unbounded source of that growth.
 *
 * The decision: **a browser/device is a relationship — one witnessed trust
 * act, reused.** Persist the key material and present the same key on every
 * later connect; the doorway recognises it and skips the grant.
 *
 * Revoke-on-session-end was considered and rejected: a deleted grant still
 * costs its per-row queries until the conductor's lookup is indexed, and
 * grant+delete per session doubles the writes on every hosted human's chain.
 *
 * ## Why localStorage
 *
 * There is no IndexedDB abstraction in this library — `elohim-service` compiles
 * without the DOM lib. The one precedent for client-side persistence here is
 * `localStorage` (the doorway JWT, `updateStoredToken`), so that is what this
 * uses. Adding an IndexedDB dependency for three small byte arrays would be a
 * heavier change than the data warrants.
 *
 * The store is reached through the module-local {@link KeyValueStore}
 * interface, never through the library's ambient `BrowserStorage` type — see
 * that interface for why naming an ambient type here breaks a consumer that
 * compiles this library from source.
 *
 * ## Why persisting a private key adds no meaningful power to an attacker
 *
 * The doorway JWT is ALREADY in `localStorage`, and that token alone lets a
 * caller hit `/hc/connect` and be issued a brand-new grant for a keypair of its
 * own choosing. Script that can read `localStorage` can therefore already act
 * as the human, with or without this key. What changes is only that the SAME
 * capability is now reachable without a round trip. The private key never
 * leaves the browser — the doorway receives only the public signing key and the
 * cap secret, exactly as before.
 *
 * ## Storage keys, and why the credential rides the app's existing one
 *
 * `holochain-signing-credentials` is the key the consuming app ALREADY writes
 * these exact credentials to after every connect, and ALREADY removes on
 * explicit sign-out (`browser-session-token.store.ts`). Reusing it is what
 * makes "clear the stored credential on sign-out" true today with no change to
 * the app: sign-out deletes the credential, and the next sign-in is granted a
 * fresh key. The SCOPE record lives beside it under its own key and carries
 * only public data (an origin, an agent id, and the public signing key as a
 * fingerprint), so a stale scope without its credential is inert.
 *
 * Every storage access is guarded. A private window, a blocked-storage policy,
 * or a Node/SSR context degrades to the previous in-memory behaviour — a fresh
 * keypair per session — and NEVER fails the connect.
 *
 * @packageDocumentation
 */

/**
 * The credential record's key. Deliberately the SAME key the consuming app
 * writes after each connect and clears on sign-out — see the module note.
 */
export const CHAPERONE_CREDENTIALS_KEY = 'holochain-signing-credentials';

/**
 * The scope record's key: which (doorway origin, agent) the stored credential
 * belongs to, plus a fingerprint of the credential it describes. Public data
 * only.
 */
export const CHAPERONE_CREDENTIAL_SCOPE_KEY = 'elohim-chaperone-credential-scope';

/**
 * A libsodium-shaped Ed25519 keypair.
 *
 * Declared locally rather than imported from `libsodium-wrappers`: that package
 * is a transitive dependency of `@holochain/client`, not a declared dependency
 * of this library, and the shape is three fields.
 */
export interface ChaperoneKeyPair {
  keyType: 'ed25519';
  publicKey: Uint8Array;
  privateKey: Uint8Array;
}

/** The credential a chaperone connect signs zome calls with. */
export interface ChaperoneCredentials {
  capSecret: Uint8Array;
  keyPair: ChaperoneKeyPair;
  /** The 39-byte `AgentPubKey` form of the public key — what the doorway grants. */
  signingKey: Uint8Array;
}

/**
 * Which (doorway origin, agent) a stored credential belongs to.
 *
 * Both halves matter. The origin keeps one doorway's grant from being presented
 * to another. The agent keeps two hosted humans sharing a browser profile from
 * sharing a credential — the second human gets their own key and their own
 * witnessed grant.
 */
export interface ChaperoneCredentialScope {
  origin: string;
  agent: string;
}

interface StoredCredentialRecord {
  capSecret: string;
  keyPair: { publicKey: string; privateKey: string };
  signingKey: string;
}

interface StoredScopeRecord {
  v: 1;
  origin: string;
  agent: string;
  /** Base64 public signing key — ties the scope to the credential it describes. */
  fingerprint: string;
}

// ---------------------------------------------------------------------------
// base64 (node + browser), mirroring the strategy's own helpers
// ---------------------------------------------------------------------------

function toBase64(bytes: Uint8Array): string {
  if (typeof btoa === 'function') {
    let binary = '';
    for (const byte of bytes) {
      binary += String.fromCodePoint(byte);
    }
    return btoa(binary);
  }
  return Buffer.from(bytes).toString('base64');
}

/**
 * UTF-8 decode, with a byte-wise fallback for runtimes with no `TextDecoder`.
 *
 * The only claim read here is `agent_pub_key`, which is ASCII, so the fallback
 * is always sufficient for this module's purpose.
 */
function decodeUtf8(bytes: Uint8Array): string {
  if (typeof TextDecoder === 'function') {
    return new TextDecoder().decode(bytes);
  }
  let out = '';
  for (const byte of bytes) {
    out += String.fromCodePoint(byte);
  }
  return out;
}

function fromBase64(value: string): Uint8Array {
  if (typeof atob === 'function') {
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.codePointAt(i)!;
    }
    return bytes;
  }
  return new Uint8Array(Buffer.from(value, 'base64'));
}

// ---------------------------------------------------------------------------
// Identity of the human this credential belongs to
// ---------------------------------------------------------------------------

/**
 * Read the agent public key out of a doorway JWT, WITHOUT verifying it.
 *
 * Verification is the doorway's job and happens on every `/hc/connect`. This
 * read is used only to decide which localStorage slot a credential belongs to,
 * so a forged claim can at worst make a browser store its own key under a
 * different slot on its own device. Returns `null` on anything unparseable, and
 * the caller then keeps the credential in memory only — the previous behaviour.
 */
// eslint-disable-next-line sonarjs/function-return-type -- intentional `T | null` API; rule misfires on nullable unions in this toolchain
export function agentFromDoorwayToken(token: string | undefined | null): string | null {
  if (!token) {
    return null;
  }
  const segments = token.split('.');
  if (segments.length < 2) {
    return null;
  }
  try {
    const padded = segments[1].replace(/-/g, '+').replace(/_/g, '/');
    const bytes = fromBase64(padded + '='.repeat((4 - (padded.length % 4)) % 4));
    const json = decodeUtf8(bytes);
    const claims = JSON.parse(json) as { agent_pub_key?: unknown };
    return typeof claims.agent_pub_key === 'string' && claims.agent_pub_key.length > 0
      ? claims.agent_pub_key
      : null;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// The conductor's "your grant is gone" answer
// ---------------------------------------------------------------------------

/**
 * Error-message fragments that say the conductor no longer honours the grant
 * behind our signing key.
 *
 * A deliberate copy of the doorway's `CAP_GRANT_REJECTION_MARKERS`
 * (`doorway-service/src/services/signing_credentials.rs`) — the same question
 * about the same conductor strings, asked on the other side of the wire.
 */
const CAP_GRANT_REJECTION_MARKERS = [
  'unauthorized',
  'capability',
  'cap grant',
  'capgrant',
  'cap secret',
  'capsecret',
];

/**
 * Does this zome-call failure mean the conductor lost our grant?
 *
 * The caller's response must be bounded: discard the stored credential and
 * reconnect ONCE with a fresh keypair (see
 * `DoorwayConnectionStrategy.healSigningCredentials`). Never a loop.
 */
export function looksLikeCapGrantRejection(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error ?? '');
  const lower = message.toLowerCase();
  return CAP_GRANT_REJECTION_MARKERS.some(marker => lower.includes(marker));
}

// ---------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------

/**
 * The three operations this module needs from a key/value store.
 *
 * Declared STRUCTURALLY, in this module, on purpose. The library's ambient
 * `BrowserStorage` (`src/types/browser-globals.d.ts`) is not visible to a
 * consumer that compiles this library from SOURCE through a path alias — the
 * app's `tsconfig.app.json` does not include the library's `types/`, so naming
 * that ambient type here broke `ng build` in `app/elohim-app` with
 * `TS2304: Cannot find name 'BrowserStorage'` while the library's own tsc and
 * vitest stayed green. A local interface type-checks identically under this
 * library's DOM-less `lib: ["ES2022"]` and under the app's DOM lib, and the
 * browser's real `Storage` satisfies it structurally.
 */
export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

/**
 * Reads and writes the device's signing credential, guarding every storage
 * access so a blocked-storage context silently degrades to in-memory.
 */
export class ChaperoneCredentialStore {
  /**
   * @param injected - an explicit store, for tests and for any host that wants
   * to supply its own. Omitted in production, where the browser's
   * `localStorage` is discovered structurally off `globalThis`.
   */
  constructor(private readonly injected?: KeyValueStore) {}

  /** `null` whenever storage is unavailable — Node, SSR, private window, blocked. */
  // eslint-disable-next-line sonarjs/function-return-type -- intentional `T | null` API; rule misfires on nullable unions in this toolchain
  private storage(): KeyValueStore | null {
    if (this.injected) {
      return this.injected;
    }
    try {
      // Reached through `globalThis` rather than the bare `localStorage`
      // identifier so this compiles with or without the DOM lib, and so a
      // getter that THROWS (a blocked-storage policy) is caught here rather
      // than at the first read.
      // Cast through `unknown`: the consuming app compiles with
      // `noPropertyAccessFromIndexSignature`, so an index-signature read
      // (`Record<string, unknown>`) is a TS4111 error there, while a bare
      // `localStorage` identifier needs an ambient this library cannot export.
      // A named optional property on a widened `globalThis` satisfies both.
      const host = globalThis as unknown as { localStorage?: KeyValueStore };
      return host.localStorage ?? null;
    } catch {
      return null;
    }
  }

  /**
   * The credential this device already holds for `scope`, or `null`.
   *
   * `null` is always safe: the caller generates a fresh keypair and the doorway
   * grants it once.
   */
  // eslint-disable-next-line sonarjs/function-return-type -- intentional `T | null` API; rule misfires on nullable unions in this toolchain
  load(scope: ChaperoneCredentialScope): ChaperoneCredentials | null {
    const storage = this.storage();
    if (!storage) {
      return null;
    }

    let rawScope: string | null;
    let rawCreds: string | null;
    try {
      rawScope = storage.getItem(CHAPERONE_CREDENTIAL_SCOPE_KEY);
      rawCreds = storage.getItem(CHAPERONE_CREDENTIALS_KEY);
    } catch {
      return null;
    }
    if (!rawScope || !rawCreds) {
      return null;
    }

    try {
      const storedScope = JSON.parse(rawScope) as StoredScopeRecord;
      if (
        storedScope?.v !== 1 ||
        storedScope.origin !== scope.origin ||
        storedScope.agent !== scope.agent
      ) {
        return null;
      }

      const record = JSON.parse(rawCreds) as StoredCredentialRecord;
      if (
        typeof record?.capSecret !== 'string' ||
        typeof record?.signingKey !== 'string' ||
        typeof record?.keyPair?.publicKey !== 'string' ||
        typeof record?.keyPair?.privateKey !== 'string'
      ) {
        return null;
      }
      // The credential under the shared key may have been replaced by another
      // writer (the native admin-WS path writes the same key). Only accept the
      // one this scope describes.
      if (record.signingKey !== storedScope.fingerprint) {
        return null;
      }

      return {
        capSecret: fromBase64(record.capSecret),
        signingKey: fromBase64(record.signingKey),
        keyPair: {
          keyType: 'ed25519',
          publicKey: fromBase64(record.keyPair.publicKey),
          privateKey: fromBase64(record.keyPair.privateKey),
        },
      };
    } catch {
      return null;
    }
  }

  /**
   * Remember this device's credential for `scope`.
   *
   * Returns `false` when storage is unavailable — the caller carries on with an
   * in-memory credential, which is exactly the pre-2026-09-20 behaviour.
   */
  save(scope: ChaperoneCredentialScope, credentials: ChaperoneCredentials): boolean {
    const storage = this.storage();
    if (!storage) {
      return false;
    }
    const signingKey = toBase64(credentials.signingKey);
    const record: StoredCredentialRecord = {
      capSecret: toBase64(credentials.capSecret),
      keyPair: {
        publicKey: toBase64(credentials.keyPair.publicKey),
        privateKey: toBase64(credentials.keyPair.privateKey),
      },
      signingKey,
    };
    const scopeRecord: StoredScopeRecord = {
      v: 1,
      origin: scope.origin,
      agent: scope.agent,
      fingerprint: signingKey,
    };
    try {
      storage.setItem(CHAPERONE_CREDENTIALS_KEY, JSON.stringify(record));
      storage.setItem(CHAPERONE_CREDENTIAL_SCOPE_KEY, JSON.stringify(scopeRecord));
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Forget the device credential — on explicit sign-out, and on the bounded
   * heal after the conductor rejects the grant behind it.
   */
  clear(): void {
    const storage = this.storage();
    if (!storage) {
      return;
    }
    try {
      storage.removeItem(CHAPERONE_CREDENTIALS_KEY);
      storage.removeItem(CHAPERONE_CREDENTIAL_SCOPE_KEY);
    } catch {
      // Storage refused the write; there is nothing useful to do and nothing
      // to fail — the in-memory credential is dropped by the caller regardless.
    }
  }
}
