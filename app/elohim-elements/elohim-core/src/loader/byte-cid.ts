/**
 * The byte profile's one address spelling, for the lamad bundle: a CIDv1 raw (0x55) +
 * BLAKE3-256 (0x1e, 32 bytes) byte CID, `bafkr4i…`, 59 characters.
 *
 * lamad is an independently served bundle (app/CLAUDE.md, bundle seams), so it carries its own
 * small admission helper rather than adding a workspace-crossing import. The rule is the one
 * `@elohim/epr` `addressForm(address) === 'bytes'` enforces: a FULL decode, never a prefix check.
 * The text must be a lower-case base32 multibase (`b…`) whose bytes are exactly
 * `01 55 1e 20 ‖ digest` (version, codec, multihash code, digest length), and re-encoding those
 * bytes must reproduce the input (non-zero padding bits, upper case and other multibases fail).
 * Anything else — the legacy `sha256-<hex>` / `sha256:<hex>` / `blake3-<hex>` markers, bare hex,
 * CIDv0, a citation (`bafkrei…`) or record (`bafyrei…`) CID — is refused.
 *
 * Minting stays in Rust: `byteCidFromDigest` only re-spells a digest a verifier just computed so
 * it can be compared with the expected address.
 */
import { blake3 } from '@noble/hashes/blake3';

const ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';
const HEADER = [0x01, 0x55, 0x1e, 0x20];
const BYTE_CID_LENGTH = 59;

function base32Encode(bytes: Uint8Array): string {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
      value &= (1 << bits) - 1;
    }
  }
  if (bits > 0) out += ALPHABET[(value << (5 - bits)) & 31];
  return out;
}

function base32Decode(text: string): Uint8Array | null {
  let bits = 0;
  let value = 0;
  const out: number[] = [];
  for (const ch of text) {
    const index = ALPHABET.indexOf(ch);
    if (index < 0) return null;
    value = (value << 5) | index;
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
      value &= (1 << bits) - 1;
    }
  }
  return Uint8Array.from(out);
}

/** The 32-byte digest a canonical byte CID commits to, or null when `address` is not one. */
export function byteCidDigest(address: string): Uint8Array | null {
  if (address.length !== BYTE_CID_LENGTH || address[0] !== 'b') return null;
  const bytes = base32Decode(address.slice(1));
  if (bytes?.length !== HEADER.length + 32) return null;
  if (!HEADER.every((b, i) => bytes[i] === b)) return null;
  // Canonical text: re-encoding must reproduce the input exactly.
  if (`b${base32Encode(bytes)}` !== address) return null;
  return bytes.slice(HEADER.length);
}

/** True iff `address` is a canonical byte CID (`bafkr4i…`). */
export function isByteCid(address: string | null | undefined): address is string {
  return !!address && byteCidDigest(address) !== null;
}

/** The byte CID text for a 32-byte BLAKE3-256 digest. */
export function byteCidFromDigest(digest: Uint8Array): string {
  return `b${base32Encode(Uint8Array.from([...HEADER, ...digest]))}`;
}

/** BLAKE3-256 of `bytes`, spelled as a byte CID. */
export function byteCidOfBytes(bytes: Uint8Array): string {
  return byteCidFromDigest(blake3(bytes, { dkLen: 32 }));
}
