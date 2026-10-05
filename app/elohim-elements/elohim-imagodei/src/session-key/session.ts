/**
 * This browser's session key across sign-in and sign-out, over any
 * {@link SessionKeyStore}. A host asks only two questions of it: what to
 * prove a signing request with, and whether the node bound a session to a
 * key this browser has since lost (site data cleared) — which reads as
 * signed out.
 */

import { webCryptoSessionKeys } from './webcrypto.js';

import type { SessionKeyStore, SessionSigner } from './signer.js';

/** A marker, not a key: "the node bound this browser's session to its key". */
const BOUND_MARK = 'elohim.session-key.bound';

const marks = {
  get(): boolean {
    try {
      return globalThis.localStorage?.getItem(BOUND_MARK) === '1';
    } catch {
      return false;
    }
  },
  set(on: boolean): void {
    try {
      if (on) globalThis.localStorage?.setItem(BOUND_MARK, '1');
      else globalThis.localStorage?.removeItem(BOUND_MARK);
    } catch {
      // Storage blocked: the page still works, it just cannot notice a lost key.
    }
  },
};

export interface SessionKeys {
  /** Whether this page can make and keep a key (a secure context with storage). */
  available(): boolean;
  /** A new key for a sign-in; kept only if the node binds the session to it. */
  forSignIn(): Promise<SessionSigner | null>;
  /** The sign-in answered: keep `signer` when the node bound the session to it. */
  signedIn(signer: SessionSigner | null, bound: boolean): Promise<void>;
  /** What to prove a signing request with: the kept key, or none. */
  current(): Promise<SessionSigner | null>;
  /** The node bound a session to a key this browser no longer has. */
  lost(): Promise<boolean>;
  /** Signed out: forget the key and the marker. */
  forget(): Promise<void>;
}

export function sessionKeysOver(store: SessionKeyStore): SessionKeys {
  let cached: Promise<SessionSigner | null> | null = null;
  const current = async (): Promise<SessionSigner | null> => {
    if (!store.available()) return Promise.resolve(null);
    cached ??= store.load();
    return cached;
  };
  return {
    available: () => store.available(),
    forSignIn: async () => (store.available() ? store.make() : null),
    async signedIn(signer, bound) {
      if (signer && bound) {
        await store.keep(signer);
        cached = Promise.resolve(signer);
        marks.set(true);
        return;
      }
      await store.clear();
      cached = Promise.resolve(null);
      marks.set(false);
    },
    current,
    async lost() {
      return marks.get() && (await current()) === null;
    },
    async forget() {
      await store.clear();
      cached = Promise.resolve(null);
      marks.set(false);
    },
  };
}

let shared: SessionKeys | null = null;

/** This page's session keys (WebCrypto in IndexedDB), one per page. */
export function sessionKeys(): SessionKeys {
  shared ??= sessionKeysOver(webCryptoSessionKeys());
  return shared;
}
