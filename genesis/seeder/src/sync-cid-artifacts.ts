// sync-cid-artifacts.ts
//
// Re-earns the reach of every CID-addressed content artifact by re-deriving its
// three source-tracked fields (content, blobHash, blobCid) from the current
// source .md — the fix side of the guard in cid-artifact-integrity.spec.ts.
//
// Authoring happens freely in the working tree (edit the source .md). This tool
// re-derives the CID address so the artifact can pass the deterministic-floor
// gate and land on the shared repo/runtime. It is surgical: only the drifted
// field literals are replaced, with an abort-guard if any literal is not found
// exactly once, so all curated metadata and JSON formatting stay byte-identical.
//
//   pnpm --filter genesis-seeder run sync:cid-artifacts          # fix in place
//   pnpm --filter genesis-seeder run sync:cid-artifacts --check  # report only (CI-style)

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { scanCidArtifacts } from './cid-artifact';

const CONTENT_DIR = resolve(process.cwd(), '../data/lamad/content');
const DOCS_ROOT = resolve(process.cwd(), '../docs/content/elohim-protocol');
const CHECK_ONLY = process.argv.includes('--check');

/** Replace `oldLit` with `newLit`, requiring it to appear exactly once. */
function replaceExactlyOnce(txt: string, oldLit: string, newLit: string, label: string): string {
  const first = txt.indexOf(oldLit);
  if (first === -1 || txt.indexOf(oldLit, first + 1) !== -1) {
    throw new Error(`${label}: source literal not found exactly once — aborting (no write)`);
  }
  return txt.slice(0, first) + newLit + txt.slice(first + oldLit.length);
}

const lit = (s: string) => JSON.stringify(s);

let drifted = 0;
let synced = 0;

for (const a of scanCidArtifacts(CONTENT_DIR, DOCS_ROOT)) {
  if (a.drift.mdMissing) {
    console.error(`✗ ${a.file}: source ${a.sourcePath} is missing`);
    process.exitCode = 1;
    continue;
  }
  const fields = (['content', 'blobHash', 'blobCid'] as const).filter((k) => a.drift[k]);
  if (fields.length === 0) {
    console.log(`  ${a.file}: in sync ✓`);
    continue;
  }
  drifted++;
  if (CHECK_ONLY) {
    console.error(`✗ ${a.file}: drifted [${fields.join(', ')}] — run without --check to fix`);
    process.exitCode = 1;
    continue;
  }
  let txt = readFileSync(a.jsonPath, 'utf-8');
  for (const k of fields) {
    txt = replaceExactlyOnce(txt, lit(a.actual[k]), lit(a.want![k]), `${a.file}.${k}`);
  }
  writeFileSync(a.jsonPath, txt);
  synced++;
  console.log(`✓ ${a.file}: synced [${fields.join(', ')}]`);
}

if (CHECK_ONLY) {
  console.log(drifted === 0 ? 'All CID-addressed artifacts in sync ✓' : `${drifted} artifact(s) drifted`);
} else {
  console.log(synced === 0 ? 'Nothing to sync — all CID-addressed artifacts current ✓' : `Synced ${synced} artifact(s)`);
}
