/**
 * elohim-imagodei/session-key — the key a signed-in browser proves each
 * signing request with: the signer interface, today's WebCrypto signer kept
 * in IndexedDB, the per-request proof, and the session's key lifecycle.
 * Framework-free; nothing here loads Lit.
 */

export * from './signer.js';
export * from './proof.js';
export * from './webcrypto.js';
export * from './session.js';

/** The node's refusals when a signing request's proof does not hold. */
export const SESSION_PROOF_CODE = {
  missing: 'session_proof_missing',
  invalid: 'session_proof_invalid',
  stale: 'session_proof_stale',
  replayed: 'session_proof_replayed',
} as const;

const PROOF_CODES = new Set<string>(Object.values(SESSION_PROOF_CODE));

/** This browser's sign-in can no longer be confirmed (any of the four). */
export function isSessionProofRefusal(code: string | undefined): boolean {
  return code !== undefined && PROOF_CODES.has(code);
}
