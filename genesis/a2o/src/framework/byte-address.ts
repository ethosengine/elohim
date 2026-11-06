/**
 * The byte-profile address for a2o steps and scripts: CIDv1 raw (0x55) + BLAKE3-256 (0x1e),
 * base32-lower (`bafkr4i…`, 59 characters). Admission is by FULL decode — version, codec,
 * multihash code, digest length and canonical text — never by a spelling or prefix test.
 * Same mint as `scripts/ci/lib/byte-cid.mjs` and `elohim_address::ByteCid`; the kernel
 * vectors in `elohim/eprfs/fixtures/tree-vectors.json` pin both.
 *
 * No `multiformats` here on purpose: that package is ESM-only and cucumber loads the a2o
 * step files through `requireModule: ['tsx']` (CommonJS), where its `./cid` subpath is not
 * exported — the first import of it took the whole lane down at load. The profile fixes the
 * 4-byte header, so the base32 step is spelled out, exactly as the CI twin does.
 */
import { blake3 } from '@noble/hashes/blake3';

/** CIDv1 · raw codec 0x55 · blake3-256 multihash 0x1e · 32-byte digest. */
const HEADER = Uint8Array.from([0x01, 0x55, 0x1e, 0x20]);
const DIGEST_LEN = 32;
const CID_LEN = HEADER.length + DIGEST_LEN; // 36 bytes
/** 'b' multibase prefix + ceil(36 * 8 / 5) = 58 base32 characters. */
const TEXT_LEN = 59;

const ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';
const ALPHABET_INDEX = new Map<string, number>([...ALPHABET].map((c, i) => [c, i]));

function base32Encode(bytes: Uint8Array): string {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += ALPHABET[(value << (5 - bits)) & 31];
  return out;
}

/** Decode base32-lower (no padding); `null` on any character outside the alphabet. */
function base32Decode(text: string): Uint8Array | null {
  const out: number[] = [];
  let bits = 0;
  let value = 0;
  for (const c of text) {
    const v = ALPHABET_INDEX.get(c);
    if (v === undefined) return null;
    value = (value << 5) | v;
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }
  return Uint8Array.from(out);
}

function encodeCid(bytes: Uint8Array): string {
  return 'b' + base32Encode(bytes);
}

/** Mint the byte CID of `data`. */
export function mintByteCid(data: Uint8Array): string {
  const digest = blake3(data, { dkLen: DIGEST_LEN });
  const cid = new Uint8Array(CID_LEN);
  cid.set(HEADER, 0);
  cid.set(digest, HEADER.length);
  return encodeCid(cid);
}

/**
 * The canonical byte CID text of `address`, or `null` when it does not fully decode as a
 * CIDv1 raw + BLAKE3-256 address (a `sha256-` spelling, bare hex, a raw+sha2 CID, a record
 * CID, a non-canonical text — all `null`).
 */
export function parseByteCid(address: unknown): string | null {
  if (typeof address !== 'string' || address.length !== TEXT_LEN || !address.startsWith('b')) {
    return null;
  }
  const bytes = base32Decode(address.slice(1));
  if (bytes?.length !== CID_LEN) return null;
  for (let i = 0; i < HEADER.length; i++) {
    if (bytes[i] !== HEADER[i]) return null;
  }
  // Canonical text: re-encoding the decoded bytes must reproduce the input exactly
  // (non-zero trailing bits or any other non-canonical spelling fail here).
  return encodeCid(bytes) === address ? address : null;
}

export function isByteCid(address: unknown): address is string {
  return parseByteCid(address) !== null;
}

/** Throwing admission for step assertions. */
export function assertByteCid(address: unknown, what = 'address'): string {
  const cid = parseByteCid(address);
  if (cid === null) {
    throw new Error(
      `${what} "${String(address)}" is not a byte CID (raw + BLAKE3-256, bafkr4i…, 59 chars)`
    );
  }
  return cid;
}
