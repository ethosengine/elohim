/**
 * Today's signer: a non-extractable ECDSA P-256 key made with WebCrypto and
 * kept, as the CryptoKey pair itself, in IndexedDB — never exported, never
 * in localStorage. Only the public half is ever read out.
 */

import type { PublicJwk, SessionKeyStore, SessionSigner } from './signer.js';

const DB = 'elohim-session-key';
const STORE = 'keys';
const KEY = 'session';

class WebCryptoSigner implements SessionSigner {
  readonly alg = 'ES256' as const;
  private jwk?: PublicJwk;

  constructor(readonly pair: CryptoKeyPair) {}

  async publicJwk(): Promise<PublicJwk> {
    if (!this.jwk) {
      const full = await globalThis.crypto.subtle.exportKey('jwk', this.pair.publicKey);
      // Only the members that name a public key; nothing else is sent.
      this.jwk = { kty: 'EC', crv: 'P-256', x: full.x ?? '', y: full.y ?? '' };
    }
    return this.jwk;
  }

  async sign(bytes: Uint8Array): Promise<Uint8Array> {
    const signature = await globalThis.crypto.subtle.sign(
      { name: 'ECDSA', hash: 'SHA-256' },
      this.pair.privateKey,
      bytes as BufferSource
    );
    return new Uint8Array(signature);
  }
}

const request = async <T>(req: IDBRequest<T>): Promise<T> =>
  new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error('IndexedDB request failed'));
  });

async function open(): Promise<IDBDatabase> {
  const req = globalThis.indexedDB.open(DB, 1);
  req.onupgradeneeded = () => req.result.createObjectStore(STORE);
  return request(req);
}

async function withStore<T>(
  mode: IDBTransactionMode,
  run: (store: IDBObjectStore) => IDBRequest<T>
): Promise<T> {
  const db = await open();
  try {
    return await request(run(db.transaction(STORE, mode).objectStore(STORE)));
  } finally {
    db.close();
  }
}

/** WebCrypto keys kept in this origin's IndexedDB. */
export function webCryptoSessionKeys(): SessionKeyStore {
  return {
    available: () =>
      globalThis.isSecureContext === true &&
      typeof globalThis.crypto?.subtle?.generateKey === 'function' &&
      typeof globalThis.indexedDB?.open === 'function',
    async make() {
      const pair = await globalThis.crypto.subtle.generateKey(
        { name: 'ECDSA', namedCurve: 'P-256' },
        false,
        ['sign', 'verify']
      );
      return new WebCryptoSigner(pair);
    },
    async keep(signer) {
      if (!(signer instanceof WebCryptoSigner)) throw new Error('not a WebCrypto signer');
      await withStore('readwrite', store => store.put(signer.pair, KEY));
    },
    async load() {
      try {
        const pair = (await withStore('readonly', store => store.get(KEY))) as
          | CryptoKeyPair
          | undefined;
        return pair?.privateKey && pair.publicKey ? new WebCryptoSigner(pair) : null;
      } catch {
        return null;
      }
    },
    async clear() {
      try {
        await withStore('readwrite', store => store.delete(KEY));
      } catch {
        // Nothing kept, or storage blocked: nothing to forget.
      }
    },
  };
}
