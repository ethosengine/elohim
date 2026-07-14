// cid-artifact.ts
//
// The deterministic floor for CID-addressed content artifacts.
//
// A content node in genesis/data/lamad/content/*.json that carries a `blobHash`
// AND a `sourcePath` ending in `.md` is a *CID-addressed artifact*: at seed time
// the source `.md` is uploaded to `/blob/<hash>` and the node is addressed by its
// `blobCid`. Three of the node's fields are therefore NOT free-authored — they are
// *derived from the source file* and must stay in lockstep with it:
//
//   content   === the source .md with its YAML frontmatter stripped
//   blobHash  === "sha256-" + sha256(source .md bytes)
//   blobCid   === CIDv1(raw, sha2-256)(source .md bytes)   // same digest as blobHash
//
// If the source is edited without re-deriving these, the CID reference dangles
// SILENTLY — the row seeds but /blob/<old-hash> 404s in the running protocol.
// This module is the single canonical definition of that derivation, shared by
// the guard test (attestation) and the sync tool (fix), so both agree on the floor.

import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve, join, basename } from 'node:path';

// RFC 4648 base32, lowercase, no padding — the multibase 'b' alphabet.
const B32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';
export function base32LowerNoPad(bytes: Uint8Array): string {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += B32_ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) {
    out += B32_ALPHABET[(value << (5 - bits)) & 31];
  }
  return out;
}

/** Strip a leading YAML frontmatter block (`---\n … \n---\n`). Matches the
 *  seeder's markdown body extraction (parseMarkdownToNode). */
export function stripFrontmatter(md: string): string {
  return md.replace(/^---\n[\s\S]*?\n---\n/, '');
}

export function deriveBlobHash(bytes: Buffer): string {
  return 'sha256-' + createHash('sha256').update(bytes).digest('hex');
}

/** CIDv1, raw codec (0x55), sha2-256 multihash (0x12, len 32), base32 multibase. */
export function deriveBlobCid(bytes: Buffer): string {
  const digest = createHash('sha256').update(bytes).digest();
  const cidBytes = Buffer.concat([Buffer.from([0x01, 0x55, 0x12, 0x20]), digest]);
  return 'b' + base32LowerNoPad(cidBytes);
}

export interface DerivedFields {
  content: string;
  blobHash: string;
  blobCid: string;
}

export function deriveFields(sourceBytes: Buffer): DerivedFields {
  return {
    content: stripFrontmatter(sourceBytes.toString('utf-8')),
    blobHash: deriveBlobHash(sourceBytes),
    blobCid: deriveBlobCid(sourceBytes),
  };
}

export interface CidArtifact {
  jsonPath: string;
  file: string;
  sourcePath: string;
  mdPath: string;
  mdExists: boolean;
  actual: DerivedFields;
  want?: DerivedFields;
  drift: { mdMissing: boolean; content: boolean; blobHash: boolean; blobCid: boolean };
}

/** Scan a content directory for CID-addressed artifacts and compute their drift
 *  against the current source `.md` files. */
export function scanCidArtifacts(contentDir: string, docsRoot: string): CidArtifact[] {
  const out: CidArtifact[] = [];
  for (const f of readdirSync(contentDir).filter((n) => n.endsWith('.json')).sort()) {
    const jsonPath = join(contentDir, f);
    let node: Record<string, unknown>;
    try {
      node = JSON.parse(readFileSync(jsonPath, 'utf-8'));
    } catch {
      continue;
    }
    const sourcePath = node.sourcePath;
    if (typeof sourcePath !== 'string' || !sourcePath.endsWith('.md')) continue;
    if (node.blobHash === undefined || node.blobHash === null) continue;

    const mdPath = resolve(docsRoot, sourcePath);
    const actual: DerivedFields = {
      content: typeof node.content === 'string' ? node.content : '',
      blobHash: typeof node.blobHash === 'string' ? node.blobHash : '',
      blobCid: typeof node.blobCid === 'string' ? node.blobCid : '',
    };

    let want: DerivedFields | undefined;
    let mdExists = true;
    try {
      want = deriveFields(readFileSync(mdPath));
    } catch {
      mdExists = false;
    }

    out.push({
      jsonPath,
      file: basename(jsonPath),
      sourcePath,
      mdPath,
      mdExists,
      actual,
      want,
      drift: {
        mdMissing: !mdExists,
        content: mdExists ? actual.content !== want!.content : false,
        blobHash: mdExists ? actual.blobHash !== want!.blobHash : false,
        blobCid: mdExists ? actual.blobCid !== want!.blobCid : false,
      },
    });
  }
  return out;
}
