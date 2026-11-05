/**
 * Deterministic-floor guard for CID-addressed artifacts that import into the
 * protocol runtime (supersedes the manifesto-only guard, genesis #1182 Cluster D).
 *
 * Any content node carrying a `blobCid` + a `sourcePath` .md is uploaded to
 * `/blob/<cid>` and addressed by that `blobCid`. Reaching the shared repo/runtime is
 * a reach-earning act: the CID address must be *attested* against its source.
 * If the source .md is edited without re-syncing the node, the reference dangles
 * SILENTLY (the row seeds, but /blob/<old-cid> 404s in the running protocol).
 *
 * The contract (both, for every CID-addressed artifact):
 *   content  === frontmatter-stripped source .md
 *   blobCid  === byte-profile CID (raw + blake3-256, bafkr4i…) of the source .md bytes
 *
 * Re-earn the reach after editing a source .md:
 *   pnpm --filter holochain-seeder run sync:cid-artifacts
 */
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import { scanCidArtifacts } from '../cid-artifact.js';

// Tests run from genesis/seeder; artifacts + sources live one level up under genesis/.
const CONTENT_DIR = resolve(process.cwd(), '../data/lamad/content');
const DOCS_ROOT = resolve(process.cwd(), '../docs/content/elohim-protocol');

const artifacts = scanCidArtifacts(CONTENT_DIR, DOCS_ROOT);
const resync = 'pnpm --filter holochain-seeder run sync:cid-artifacts';

describe('CID-addressed artifact integrity', () => {
  it('the guard is wired (finds the known CID-addressed artifacts)', () => {
    // If this fails, the scan stopped seeing artifacts — the each-blocks below
    // would silently become zero tests, so assert the floor is actually watched.
    expect(artifacts.map((a) => a.file)).toContain('manifesto.json');
  });

  it.each(artifacts)('$file — source .md exists ($sourcePath)', (a) => {
    expect(a.mdExists, `${a.file}: source ${a.sourcePath} not found at ${a.mdPath}`).toBe(true);
  });

  it.each(artifacts)('$file — content matches frontmatter-stripped source', (a) => {
    if (!a.mdExists) return;
    expect(
      a.actual.content,
      `${a.file}.content drifted from ${a.sourcePath}. Re-sync: ${resync}`,
    ).toBe(a.want!.content);
  });

  it.each(artifacts)('$file — blobCid == byte-profile CID of the source bytes', (a) => {
    if (!a.mdExists) return;
    expect(
      a.actual.blobCid,
      `${a.file}.blobCid drifted from ${a.sourcePath} — CID address dangles. Re-sync: ${resync}`,
    ).toBe(a.want!.blobCid);
  });
});
