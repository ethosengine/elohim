#!/usr/bin/env node
/**
 * Seed the elohim-host-landing ContentNode bundle.
 *
 * - Reads app/elohim-app/dist/elohim-app/browser/ as the source tree
 * - Produces a zip in-memory
 * - POSTs the bytes to {STORAGE_URL}/blob → receives the sha256 hash
 * - Patches genesis/data/lamad/content/elohim-host-landing.json with the hash
 * - Idempotent: if the hash already matches, the seed file is untouched
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 node genesis/scripts/seed-landing-bundle.mjs
 */

import { createHash } from 'node:crypto';
import { readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import JSZip from 'jszip';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const REPO_ROOT = resolve(__dirname, '..', '..');
const BUNDLE_DIR = resolve(REPO_ROOT, 'app/elohim-app/dist/elohim-app/browser');
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

async function uploadBlob(bytes) {
  const res = await fetch(`${STORAGE_URL}/blob`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/zip' },
    body: bytes,
  });
  if (!res.ok) {
    throw new Error(`blob upload failed: ${res.status} ${await res.text()}`);
  }
  const body = await res.json();
  // Existing /blob endpoint returns { hash: "sha256-..." } per blob_store conventions.
  const hash = body.hash ?? body.blobHash ?? body.sha256;
  if (!hash) throw new Error(`blob response missing hash field: ${JSON.stringify(body)}`);
  return hash.replace(/^sha256[-:]/, '');
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

  const localHash = createHash('sha256').update(zipBytes).digest('hex');
  console.log(`[seed-landing-bundle] local sha256: ${localHash}`);

  console.log(`[seed-landing-bundle] uploading to ${STORAGE_URL}/blob…`);
  const remoteHash = await uploadBlob(zipBytes);
  console.log(`[seed-landing-bundle] remote hash:  ${remoteHash}`);

  if (remoteHash !== localHash) {
    console.warn(
      `[seed-landing-bundle] WARNING: remote hash differs from local — content addressing may have been altered in transit`,
    );
  }

  const seedText = await readFile(SEED_PATH, 'utf8');
  const seed = JSON.parse(seedText);
  if (seed.blobHash === remoteHash) {
    console.log('[seed-landing-bundle] seed already up-to-date; no changes');
    return;
  }
  seed.blobHash = remoteHash;
  seed.updatedAt = new Date().toISOString();
  await writeFile(SEED_PATH, JSON.stringify(seed, null, 2) + '\n');
  console.log(`[seed-landing-bundle] patched ${SEED_PATH}`);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
