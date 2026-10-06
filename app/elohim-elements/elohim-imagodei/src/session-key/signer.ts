/**
 * The key a signed-in browser proves each signing request with.
 *
 * A signer is three things: make a key, show its public half, sign bytes.
 * WebCrypto (a non-extractable P-256 key kept in IndexedDB) is today's
 * implementation; a platform authenticator is meant to be another, so a host
 * never touches key material, only this interface.
 *
 * Framework-free and Lit-free.
 */

/** The JOSE algorithm every signer here speaks. */
export type SessionKeyAlg = 'ES256';

/** The public half of a P-256 key, as JOSE names its members. */
export interface PublicJwk {
  kty: 'EC';
  crv: 'P-256';
  x: string;
  y: string;
}

/** A key that can sign, whose private half never leaves it. */
export interface SessionSigner {
  readonly alg: SessionKeyAlg;
  /** The public half, to name the key in a sign-in and in every proof. */
  publicJwk(): Promise<PublicJwk>;
  /** Sign `bytes`: for ES256 the raw r||s form JOSE requires (64 bytes). */
  sign(bytes: Uint8Array): Promise<Uint8Array>;
}

/** Where a browser keeps its signer between page loads. */
export interface SessionKeyStore {
  /** Whether this page can make and keep a key at all (a secure context with storage). */
  available(): boolean;
  /** A new key, not yet kept: kept only once a sign-in has bound it. */
  make(): Promise<SessionSigner>;
  /** Keep `signer` as this browser's key, replacing any other. */
  keep(signer: SessionSigner): Promise<void>;
  /** This browser's kept key, or none. */
  load(): Promise<SessionSigner | null>;
  /** Forget this browser's key (signing out). */
  clear(): Promise<void>;
}
