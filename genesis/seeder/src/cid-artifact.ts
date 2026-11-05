// cid-artifact.ts
//
// The deterministic floor for CID-addressed content artifacts.
//
// A content node in genesis/data/lamad/content/*.json that carries a `blobCid`
// AND a `sourcePath` ending in `.md` is a *CID-addressed artifact*: at seed time
// the source `.md` is uploaded to `/blob/<cid>` and the node is addressed by its
// `blobCid`. Two of the node's fields are therefore NOT free-authored — they are
// *derived from the source file* and must stay in lockstep with it:
//
//   content   === the source .md with its YAML frontmatter stripped
//   blobCid   === byte-profile CIDv1 (raw 0x55 + blake3-256, `bafkr4i…`) of the source .md bytes
//
// If the source is edited without re-deriving these, the CID reference dangles
// SILENTLY — the row seeds but /blob/<old-cid> 404s in the running protocol.
// This module is the single canonical definition of that derivation, shared by
// the guard test (attestation) and the sync tool (fix), so both agree on the floor.

import { readFileSync, readdirSync } from 'node:fs';
import { resolve, join, basename } from 'node:path';

import { byteCid } from './byte-cid.js';

/** Strip a leading YAML frontmatter block (`---\n … \n---\n`). Matches the
 *  seeder's markdown body extraction (parseMarkdownToNode). */
export function stripFrontmatter(md: string): string {
  return md.replace(/^---\n[\s\S]*?\n---\n/, '');
}

/** Byte-profile CIDv1 (raw 0x55 + blake3-256) of the source bytes. */
export function deriveBlobCid(bytes: Buffer): string {
  return byteCid(bytes);
}

export interface DerivedFields {
  content: string;
  blobCid: string;
}

export function deriveFields(sourceBytes: Buffer): DerivedFields {
  return {
    content: stripFrontmatter(sourceBytes.toString('utf-8')),
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
  drift: { mdMissing: boolean; content: boolean; blobCid: boolean };
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
    if (node.blobCid === undefined || node.blobCid === null) continue;

    const mdPath = resolve(docsRoot, sourcePath);
    const actual: DerivedFields = {
      content: typeof node.content === 'string' ? node.content : '',
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
        blobCid: mdExists ? actual.blobCid !== want!.blobCid : false,
      },
    });
  }
  return out;
}
