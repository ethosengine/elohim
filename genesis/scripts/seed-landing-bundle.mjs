#!/usr/bin/env node
/**
 * Seed the elohim-host-landing ContentNode bundle.
 *
 * - Reads app/elohim-app/dist/elohim-app/browser/ as the source tree
 * - Produces a zip in-memory
 * - Mints the byte-profile CID (raw + blake3-256, bafkr4i…) of the zip
 * - PUTs the bytes to {STORAGE_URL}/blob/{cid} (X-Content-Address: {cid})
 * - Patches genesis/data/lamad/content/elohim-host-landing.json with the CID
 * - Idempotent: if the CID already matches, the seed file is untouched
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 node genesis/scripts/seed-landing-bundle.mjs
 *   DRY_RUN=1 [BUNDLE_DIR=<dir>] node genesis/scripts/seed-landing-bundle.mjs
 *     (zips and mints the CID only: no upload, no seed-file write)
 */

import { readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import JSZip from 'jszip';

// Node type-stripping loads the seeder's one byte-CID mint (never re-implemented here).
import { byteCid } from '../seeder/src/byte-cid.ts';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const REPO_ROOT = resolve(__dirname, '..', '..');
const BUNDLE_DIR = process.env.BUNDLE_DIR
  ? resolve(process.env.BUNDLE_DIR)
  : resolve(REPO_ROOT, 'app/elohim-app/dist/elohim-app/browser');
const DRY_RUN = process.env.DRY_RUN === '1';
const SEED_PATH = resolve(REPO_ROOT, 'genesis/data/lamad/content/elohim-host-landing.json');
const STORAGE_URL = process.env.STORAGE_URL ?? 'http://localhost:8090';

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walk(full)));
    } else if (entry.isFile()) {
      files.push(full);
    }
  }
  return files;
}

async function buildZip() {
  const zip = new JSZip();
  const files = await walk(BUNDLE_DIR);
  for (const f of files) {
    const rel = relative(BUNDLE_DIR, f);
    const bytes = await readFile(f);
    zip.file(rel, bytes);
  }
  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE' });
}

async function uploadBlob(bytes, cid) {
  const res = await fetch(`${STORAGE_URL}/blob/${cid}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/zip', 'X-Content-Address': cid },
    body: bytes,
  });
  if (!res.ok) {
    throw new Error(`blob upload failed: ${res.status} ${await res.text()}`);
  }
  const body = await res.json();
  const remote = body.blobCid ?? body.cid;
  if (!remote) throw new Error(`blob response missing blobCid field: ${JSON.stringify(body)}`);
  return remote;
}

async function main() {
  const stats = await stat(BUNDLE_DIR).catch(() => null);
  if (!stats || !stats.isDirectory()) {
    throw new Error(
      `Bundle directory not found: ${BUNDLE_DIR}\n` +
        `Run \`pnpm --filter elohim-app run build\` first.`,
    );
  }

  console.log(`[seed-landing-bundle] reading ${BUNDLE_DIR}…`);
  const zipBytes = await buildZip();
  console.log(`[seed-landing-bundle] zipped: ${zipBytes.byteLength} bytes`);

  const localCid = byteCid(zipBytes);
  console.log(`[seed-landing-bundle] local CID: ${localCid}`);
  if (DRY_RUN) {
    console.log('[seed-landing-bundle] DRY_RUN=1: no upload, seed file untouched');
    return;
  }

  console.log(`[seed-landing-bundle] uploading to ${STORAGE_URL}/blob/${localCid}…`);
  const remoteCid = await uploadBlob(zipBytes, localCid);
  console.log(`[seed-landing-bundle] remote CID: ${remoteCid}`);

  if (remoteCid !== localCid) {
    throw new Error(
      `[seed-landing-bundle] remote CID ${remoteCid} differs from local ${localCid} — storage verified different bytes`,
    );
  }

  const seedText = await readFile(SEED_PATH, 'utf8');
  const seed = JSON.parse(seedText);
  if (seed.blobCid === remoteCid) {
    console.log('[seed-landing-bundle] seed already up-to-date; no changes');
    return;
  }
  seed.blobCid = remoteCid;
  seed.updatedAt = new Date().toISOString();
  await writeFile(SEED_PATH, JSON.stringify(seed, null, 2) + '\n');
  console.log(`[seed-landing-bundle] patched ${SEED_PATH}`);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
