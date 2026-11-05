#!/usr/bin/env npx tsx
/**
 * Genesis Blob Pack Builder
 *
 * Pre-computes content-addressed blobs from seed data JSONs.
 * This enables fast seeding by separating blob storage from DHT manifest creation.
 *
 * Output Structure:
 *   genesis/blobs/
 *   ├── {cid}/         # Raw content body files
 *   └── manifest.json  # Maps content_id → { cid, size_bytes, content_format }
 *
 * Usage:
 *   npm run genesis:pack           # Build blob pack from data/lamad/
 *   npm run genesis:pack -- --dry  # Preview what would be packed
 *
 * During seeding, the seeder:
 *   1. Checks if blobs exist locally (by CID)
 *   2. Copies from genesis/blobs/ if needed
 *   3. Creates DHT manifests pointing to CIDs (no body upload)
 */

import * as fs from 'fs';
import * as path from 'path';
import { byteCid } from './byte-cid.js';
import { fileURLToPath } from 'url';

// ES module compatibility
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Configuration
const DATA_DIR = path.resolve(__dirname, '../../data/lamad');
const BLOBS_DIR = path.resolve(__dirname, '../../blobs');
const MANIFEST_PATH = path.join(BLOBS_DIR, 'manifest.json');

interface ContentJson {
  id: string;
  title?: string;
  content?: string | object;  // String for markdown, object for quizzes/assessments
  contentFormat?: string;
  metadata?: {
    localZipPath?: string;  // For html5-app: path to ZIP file relative to genesis/
    [key: string]: unknown;
  };
  // ... other fields we don't need to pack
}

interface BlobManifestEntry {
  /** Byte-profile CID (bafkr4i…) — also the blob's filename under genesis/blobs/. */
  cid: string;
  size_bytes: number;
  content_format: string;
}

interface GenesisManifest {
  version: 1;
  created_at: string;
  total_blobs: number;
  total_bytes: number;
  entries: Record<string, BlobManifestEntry>;  // content_id → blob info
}

// Root directory for resolving relative paths (genesis/)
const GENESIS_DIR = path.resolve(__dirname, '../..');

/**
 * Extract content body from JSON file
 * For html5-app content, reads the actual ZIP file from metadata.localZipPath
 */
function extractContentBody(json: ContentJson): { body: Buffer; format: string } | null {
  // Skip entries without content body
  if (!json.content) {
    return null;
  }

  // Determine format
  const format = json.contentFormat || 'markdown';

  // Special handling for html5-app: read the ZIP file, not the JSON content
  if (format === 'html5-app' && json.metadata?.localZipPath) {
    const zipPath = path.resolve(GENESIS_DIR, json.metadata.localZipPath);
    if (fs.existsSync(zipPath)) {
      const zipData = fs.readFileSync(zipPath);
      console.log(`  📦 ${json.id}: Reading ZIP from ${json.metadata.localZipPath} (${(zipData.length / 1024 / 1024).toFixed(2)} MB)`);
      return { body: zipData, format };
    } else {
      console.warn(`  ⚠️ ${json.id}: ZIP not found at ${zipPath}`);
      return null;
    }
  }

  // Handle both string and object content
  let contentStr: string;
  if (typeof json.content === 'string') {
    contentStr = json.content.trim();
    if (contentStr === '') {
      return null;
    }
  } else if (typeof json.content === 'object') {
    // Object content (quizzes, assessments) - serialize deterministically
    // For arrays, just stringify without a replacer to preserve all keys
    if (Array.isArray(json.content)) {
      contentStr = JSON.stringify(json.content);
    } else {
      contentStr = JSON.stringify(json.content, Object.keys(json.content).sort());
    }
  } else {
    return null;
  }

  // Content body as UTF-8 buffer
  return {
    body: Buffer.from(contentStr, 'utf-8'),
    format
  };
}

/**
 * Process all content JSON files and build blob pack
 */
async function buildGenesisPack(dryRun: boolean = false): Promise<void> {
  console.log('========================================');
  console.log('   GENESIS BLOB PACK BUILDER');
  console.log('========================================\n');

  const contentDir = path.join(DATA_DIR, 'content');
  const assessmentDir = path.join(DATA_DIR, 'assessments');

  if (!fs.existsSync(contentDir)) {
    console.error(`Content directory not found: ${contentDir}`);
    process.exit(1);
  }

  // Collect all JSON files
  const jsonFiles: string[] = [];

  const collectJsonFiles = (dir: string) => {
    if (!fs.existsSync(dir)) return;
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        collectJsonFiles(fullPath);
      } else if (entry.name.endsWith('.json')) {
        jsonFiles.push(fullPath);
      }
    }
  };

  collectJsonFiles(contentDir);
  collectJsonFiles(assessmentDir);

  console.log(`Found ${jsonFiles.length} JSON files\n`);

  // Process each file
  const manifest: GenesisManifest = {
    version: 1,
    created_at: new Date().toISOString(),
    total_blobs: 0,
    total_bytes: 0,
    entries: {}
  };

  let skipped = 0;
  let processed = 0;

  if (!dryRun) {
    fs.mkdirSync(BLOBS_DIR, { recursive: true });
  }

  for (const jsonPath of jsonFiles) {
    try {
      const jsonContent = fs.readFileSync(jsonPath, 'utf-8');
      const json: ContentJson = JSON.parse(jsonContent);

      if (!json.id) {
        skipped++;
        continue;
      }

      const extracted = extractContentBody(json);
      if (!extracted) {
        skipped++;
        continue;
      }

      const { body, format } = extracted;
      const cid = byteCid(body);

      // Store blob
      if (!dryRun) {
        const blobPath = path.join(BLOBS_DIR, cid);
        if (!fs.existsSync(blobPath)) {
          fs.writeFileSync(blobPath, body);
        }
      }

      // Add to manifest
      manifest.entries[json.id] = {
        cid,
        size_bytes: body.length,
        content_format: format
      };

      manifest.total_blobs++;
      manifest.total_bytes += body.length;
      processed++;

    } catch (err) {
      console.warn(`Warning: Failed to process ${jsonPath}: ${err}`);
    }
  }

  // Write manifest
  if (!dryRun) {
    fs.writeFileSync(MANIFEST_PATH, JSON.stringify(manifest, null, 2));
  }

  // Summary
  console.log('========================================');
  console.log('   SUMMARY');
  console.log('========================================\n');
  console.log(`  Processed: ${processed} content items`);
  console.log(`  Skipped: ${skipped} items (no content body)`);
  console.log(`  Total blobs: ${manifest.total_blobs}`);
  console.log(`  Total bytes: ${formatBytes(manifest.total_bytes)}`);
  console.log(`  Output: ${BLOBS_DIR}`);

  if (dryRun) {
    console.log('\n  [DRY RUN - no files written]');
  } else {
    console.log(`\n  Manifest: ${MANIFEST_PATH}`);
    console.log('\n  Next: npm run genesis:seed (uses manifest for fast seeding)');
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// CLI
const dryRun = process.argv.includes('--dry');
buildGenesisPack(dryRun).catch(console.error);
