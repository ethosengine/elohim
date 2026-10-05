/**
 * The proof sent with every request that makes the node sign — a compact
 * JWS in the `DPoP` header, made with this browser's session key.
 *
 *   header  { typ: "dpop+jwt", alg: "ES256", jwk: <public JWK> }
 *   claims  { jti, htm, htu, iat, bsh }
 *
 * `bsh` is base64url(SHA-256(the exact body bytes sent)), so the body is
 * serialised once and that same string is both hashed and sent. Segments
 * are base64url without padding; the signature is ES256's raw r||s.
 */

import type { SessionSigner } from './signer.js';

const ENCODER = new TextEncoder();

/** base64url without padding. */
export function base64url(bytes: Uint8Array): string {
  let binary = '';
  for (const b of bytes) binary += String.fromCodePoint(b);
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
}

const b64json = (value: unknown): string => base64url(ENCODER.encode(JSON.stringify(value)));

/** base64url(SHA-256(bytes)). */
export async function sha256Base64url(bytes: Uint8Array): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest('SHA-256', bytes as BufferSource);
  return base64url(new Uint8Array(digest));
}

/** A random proof id, unique per request. */
export function newJti(): string {
  return base64url(globalThis.crypto.getRandomValues(new Uint8Array(16)));
}

export interface ProofFor {
  /** HTTP method, upper case. */
  htm: string;
  /** The request URL, absolute, as sent. */
  htu: string;
  /** The exact body bytes sent (none for a body-less request). */
  body: Uint8Array;
  /** For tests only: fixed proof id and time. */
  jti?: string;
  iat?: number;
}

/** The compact JWS for one request. */
export async function makeProof(signer: SessionSigner, request: ProofFor): Promise<string> {
  const header = { typ: 'dpop+jwt', alg: signer.alg, jwk: await signer.publicJwk() };
  const claims = {
    jti: request.jti ?? newJti(),
    htm: request.htm,
    htu: request.htu,
    iat: request.iat ?? Math.floor(Date.now() / 1000),
    bsh: await sha256Base64url(request.body),
  };
  const signingInput = `${b64json(header)}.${b64json(claims)}`;
  const signature = await signer.sign(ENCODER.encode(signingInput));
  return `${signingInput}.${base64url(signature)}`;
}
