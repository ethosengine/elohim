import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// @noble/ed25519 v2 requires setting the sha-512 hash provider for sync operations
ed.etc.sha512Sync = (...m: Uint8Array[]) => sha512(ed.etc.concatBytes(...m));

/** Verify an Ed25519 signature. */
export async function verifyEd25519(
  publicKey: Uint8Array,
  message: Uint8Array,
  signature: Uint8Array,
): Promise<boolean> {
  if (publicKey.length !== 32 || signature.length !== 64) return false;
  try {
    return await ed.verifyAsync(signature, message, publicKey);
  } catch {
    return false;
  }
}
