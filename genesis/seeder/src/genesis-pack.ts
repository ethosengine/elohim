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
 *   └── manifest.json  # Maps content_id → { cid, size_bytes, hash }
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
import * as crypto from 'crypto';
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
  // ... other fields we don't need to pack
}

interface BlobManifestEntry {
  cid: string;
  hash: string;
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

/**
 * Compute SHA256 hash of data
 */
function computeHash(data: Buffer): string {
  return crypto.createHash('sha256').update(data).digest('hex');
}

/**
 * Compute CIDv1 from SHA256 hash (IPFS-compatible)
 * Format: bafkrei... (base32-encoded CIDv1 with raw codec)
 */
function hashToCid(sha256Hex: string): string {
  // Build multihash: varint(sha2-256 code) + varint(digest length) + digest
  const digestBytes = Buffer.from(sha256Hex, 'hex');
  const SHA256_CODE = 0x12;  // sha2-256 in multihash
  const DIGEST_LENGTH = 32;

  // Multihash encoding
  const multihash = Buffer.alloc(2 + DIGEST_LENGTH);
  multihash[0] = SHA256_CODE;
  multihash[1] = DIGEST_LENGTH;
  digestBytes.copy(multihash, 2);

  // CIDv1 with raw codec (0x55)
  const RAW_CODEC = 0x55;
  const CID_VERSION = 1;

  // CIDv1: version + codec + multihash
  const cidBytes = Buffer.concat([
    Buffer.from([CID_VERSION, RAW_CODEC]),
    multihash
  ]);

  // Base32 encode with "b" prefix (rfc4648)
  return 'b' + base32Encode(cidBytes).toLowerCase();
}

/**
 * RFC 4648 base32 encoding (no padding)
 */
function base32Encode(data: Buffer): string {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = 0;
  let value = 0;
  let output = '';

  for (let i = 0; i < data.length; i++) {
    value = (value << 8) | data[i];
    bits += 8;

    while (bits >= 5) {
      output += alphabet[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }

  if (bits > 0) {
    output += alphabet[(value << (5 - bits)) & 31];
  }

  return output;
}

/**
 * Extract content body from JSON file
 */
function extractContentBody(json: ContentJson): { body: Buffer; format: string } | null {
  // Skip entries without content body
  if (!json.content) {
    return null;
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
    contentStr = JSON.stringify(json.content, Object.keys(json.content).sort());
  } else {
    return null;
  }

  // Determine format
  const format = json.contentFormat || 'markdown';

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
      const hash = computeHash(body);
      const cid = hashToCid(hash);

      // Store blob
      if (!dryRun) {
        const blobPath = path.join(BLOBS_DIR, hash);
        if (!fs.existsSync(blobPath)) {
          fs.writeFileSync(blobPath, body);
        }
      }

      // Add to manifest
      manifest.entries[json.id] = {
        cid,
        hash,
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
