#!/usr/bin/env node
// scripts/ci/lib/byte-cid.mjs — the ONE byte-address mint for CI shell scripts.
//
// Prints the byte-profile CID of each file argument, one per line:
//   CIDv1 (0x01) . raw codec (0x55) . blake3-256 multihash (0x1e, len 0x20) . digest,
//   multibase base32-lower ('b' prefix)  ->  bafkr4i…  (59 chars).
// Same profile as genesis/seeder/src/byte-cid.ts and the kernel's BYTE_PROFILE_0.
//
// Only blake3 needs a library (@noble/hashes, CJS-loadable). multiformats is
// ESM-only and cannot be `require`d from a bare script, and the 4-byte header is
// fixed by the profile, so the base32 step is spelled out here (verified against
// the shared vectors by byte-cid.test.sh). OCI `sha256:` digests never come
// through this file.
//
// Usage: node byte-cid.mjs <file> [<file>...]      exit 2 on a missing dependency.
import { createRequire } from 'node:module';
import { readFileSync, realpathSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '..', '..', '..');
// Anchors whose node_modules carry @noble/hashes (workspace packages that declare it).
const anchors = [
  resolve(repo, 'genesis/seeder/package.json'),
  resolve(repo, 'elohim/sdk/epr-ts/package.json'),
  resolve(repo, 'package.json'),
];

function loadBlake3() {
  for (const a of anchors) {
    try {
      return createRequire(a)('@noble/hashes/blake3').blake3;
    } catch {
      // try the next anchor
    }
  }
  console.error('byte-cid.mjs: @noble/hashes is not installed (run `pnpm install`); tried: ' + anchors.join(', '));
  process.exit(2);
}

const ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';
function base32(bytes) {
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

export function byteCid(blake3, bytes) {
  const header = Uint8Array.from([0x01, 0x55, 0x1e, 0x20]);
  const digest = blake3(bytes);
  const cid = new Uint8Array(header.length + digest.length);
  cid.set(header, 0);
  cid.set(digest, header.length);
  return 'b' + base32(cid);
}

if (process.argv[1] && realpathSync(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const files = process.argv.slice(2);
  if (files.length === 0) {
    console.error('usage: byte-cid.mjs <file> [<file>...]');
    process.exit(1);
  }
  const blake3 = loadBlake3();
  for (const f of files) console.log(byteCid(blake3, readFileSync(f)));
}
