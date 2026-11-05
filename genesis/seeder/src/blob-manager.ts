/**
 * Blob Manager - Content extraction, hashing, and management
 *
 * Handles the separation of metadata (DNA) from content blobs (projection cache).
 *
 * Content Architecture:
 * - DNA stores metadata with blobCid references
 * - elohim-cache-core stores and serves actual blob content
 * - Clients use ContentResolver to find content across tiers
 *
 * Blob-extractable formats:
 * - html5-app: Zip files with entryPoint
 * - perseus-quiz-json: Large quiz JSON
 * - gherkin: Feature file content (usually small, may stay inline)
 *
 * Usage:
 *   const manager = new BlobManager({ doorwayUrl: 'https://doorway.example.com' });
 *   const result = await manager.processContent(content);
 *   // result.metadata -> seed to DNA
 *   // result.blob -> push to projection cache
 */

import * as fs from 'fs';
import * as path from 'path';
import { byteCid, isByteCid } from './byte-cid.js';

// =============================================================================
// Types
// =============================================================================

export interface BlobMetadata {
  /** Byte-profile CID (raw + blake3-256, `bafkr4i…`) of the blob content */
  cid: string;
  /** Size in bytes */
  sizeBytes: number;
  /** MIME type */
  mimeType: string;
  /** Entry point for compound content (e.g., "index.html" for html5-app) */
  entryPoint?: string;
  /** Fallback URL if resolution fails */
  fallbackUrl?: string;
}

export interface ProcessedContent {
  /** Metadata to store in DNA (blob_cid reference instead of inline content) */
  metadata: Record<string, unknown>;
  /** Blob to push to projection cache (null if content stays inline) */
  blob: Buffer | null;
  /** Blob metadata (null if content stays inline) */
  blobMetadata: BlobMetadata | null;
  /** Whether content was extracted to blob */
  extracted: boolean;
}

export interface BlobManagerConfig {
  /** Doorway URL for pushing blobs */
  doorwayUrl: string;
  /** API key for doorway authentication */
  apiKey?: string;
  /** Minimum size (bytes) to extract as blob (default: 10KB) */
  minBlobSize?: number;
  /** Directory for cached blobs */
  cacheDir?: string;
  /** Dry run - don't actually push blobs */
  dryRun?: boolean;
  /** Force upload blobs even if blob_cid already exists (for fresh deployments) */
  forceUpload?: boolean;
}

export interface ContentFile {
  id: string;
  contentType?: string;
  contentFormat?: string;
  title?: string;
  description?: string;
  content?: string | object;
  blobCid?: string;     // byte-profile CID (JSON file format)
  blobUrl?: string;
  entryPoint?: string;
  fallbackUrl?: string;
  [key: string]: unknown;
}

// Formats that should be extracted as blobs
const BLOB_FORMATS = ['html5-app', 'perseus-quiz-json'];

// MIME types by format
const FORMAT_MIME_TYPES: Record<string, string> = {
  'html5-app': 'application/zip',
  'perseus-quiz-json': 'application/json',
  'gherkin': 'text/plain',
  'markdown': 'text/markdown',
  'plaintext': 'text/plain',
};

// =============================================================================
// Blob Manager
// =============================================================================

export class BlobManager {
  private config: Required<BlobManagerConfig>;
  private blobCache: Map<string, BlobMetadata> = new Map();

  constructor(config: BlobManagerConfig) {
    this.config = {
      doorwayUrl: config.doorwayUrl,
      apiKey: config.apiKey || '',
      minBlobSize: config.minBlobSize || 10 * 1024, // 10KB default
      cacheDir: config.cacheDir || '.blob-cache',
      dryRun: config.dryRun || false,
      forceUpload: config.forceUpload || false,
    };

    // Ensure cache directory exists
    if (!fs.existsSync(this.config.cacheDir)) {
      fs.mkdirSync(this.config.cacheDir, { recursive: true });
    }
  }

  /**
   * Process content file and determine if blob extraction is needed.
   *
   * @param content - The content file object
   * @param contentDir - Directory containing content files (for resolving blob files)
   * @returns ProcessedContent with metadata and optional blob
   */
  async processContent(content: ContentFile, contentDir: string): Promise<ProcessedContent> {
    const format = content.contentFormat || 'markdown';

    // Check if this format should be extracted as blob
    if (!BLOB_FORMATS.includes(format)) {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false,
      };
    }

    // The row's declared blob address. Only a byte-profile CID is an address;
    // any other spelling is not normalised into one (no dual-read) — it is
    // treated as absent so the blob is re-derived from its bytes below.
    const existingCid = isByteCid(content.blobCid) ? content.blobCid : null;

    // Check if content already has a blob reference
    // If forceUpload is true, we still extract and prepare the blob for upload
    // (needed for fresh deployments where blob_cid exists but blob isn't in storage)
    if (existingCid && !this.config.forceUpload) {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false, // Already has reference
      };
    }

    // Extract blob based on format
    let blob: Buffer;
    let entryPoint: string | undefined;

    if (format === 'html5-app') {
      // HTML5 app: look for zip file or blob file
      const blobResult = await this.extractHtml5AppBlob(content, contentDir);
      if (!blobResult) {
        // No blob to extract, keep as inline
        return {
          metadata: content,
          blob: null,
          blobMetadata: null,
          extracted: false,
        };
      }
      blob = blobResult.blob;
      entryPoint = blobResult.entryPoint || 'index.html';
    } else if (format === 'perseus-quiz-json') {
      // Perseus quiz: serialize content as JSON blob
      const blobResult = this.extractPerseusQuizBlob(content);
      if (!blobResult) {
        return {
          metadata: content,
          blob: null,
          blobMetadata: null,
          extracted: false,
        };
      }
      blob = blobResult;
    } else {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false,
      };
    }

    // Check minimum size
    if (blob.length < this.config.minBlobSize) {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false,
      };
    }

    // Use existing blobCid if available (forceUpload mode), otherwise compute
    // This ensures consistency between JSON and actual blob address
    const cid = existingCid || this.computeCid(blob);
    const mimeType = FORMAT_MIME_TYPES[format] || 'application/octet-stream';

    const blobMetadata: BlobMetadata = {
      cid,
      sizeBytes: blob.length,
      mimeType,
      entryPoint,
      fallbackUrl: content.fallbackUrl as string | undefined,
    };

    // Create metadata with blob reference (remove inline content)
    const metadata: Record<string, unknown> = {
      ...content,
      blobCid: cid,
      blobUrl: `${this.config.doorwayUrl}/blob/${cid}`,
    };

    if (entryPoint) {
      metadata.entryPoint = entryPoint;
    }

    // Remove inline content from metadata
    delete metadata.content;

    // Cache blob locally
    this.cacheBlob(cid, blob);
    this.blobCache.set(cid, blobMetadata);

    return {
      metadata,
      blob,
      blobMetadata,
      extracted: true,
    };
  }

  /**
   * Extract blob from HTML5 app content.
   * Looks for (in order):
   * 1. metadata.localZipPath - relative to genesis directory
   * 2. A corresponding zip file in the content directory ({id}.zip)
   * 3. A blob_file reference in the content
   */
  private async extractHtml5AppBlob(
    content: ContentFile,
    contentDir: string
  ): Promise<{ blob: Buffer; entryPoint?: string; slug?: string } | null> {
    // Get slug from content.content object (for html5-app format)
    const contentObj = typeof content.content === 'object' ? content.content as Record<string, unknown> : null;
    const slug = contentObj?.slug as string | undefined;
    const entryPoint = contentObj?.entryPoint as string | undefined || (content.entryPoint as string) || 'index.html';

    // Try metadata.localZipPath first (relative to genesis directory)
    const metadata = content.metadata as Record<string, unknown> | undefined;
    if (metadata?.localZipPath) {
      // Genesis directory is 3 levels up from content dir (content -> lamad -> data -> genesis)
      const genesisDir = path.resolve(contentDir, '../../..');
      const zipPath = path.join(genesisDir, metadata.localZipPath as string);
      if (fs.existsSync(zipPath)) {
        console.log(`   📦 Found ZIP via metadata.localZipPath: ${metadata.localZipPath}`);
        return {
          blob: fs.readFileSync(zipPath),
          entryPoint,
          slug,
        };
      }
    }

    // Try to find a zip file with same ID in content directory
    const zipPath = path.join(contentDir, `${content.id}.zip`);
    if (fs.existsSync(zipPath)) {
      return {
        blob: fs.readFileSync(zipPath),
        entryPoint,
        slug,
      };
    }

    // Try blob_file reference
    if (content.blobFile) {
      const blobPath = path.join(contentDir, content.blobFile as string);
      if (fs.existsSync(blobPath)) {
        return {
          blob: fs.readFileSync(blobPath),
          entryPoint,
          slug,
        };
      }
    }

    // No local blob found
    return null;
  }

  /**
   * Extract perseus quiz content as blob.
   */
  private extractPerseusQuizBlob(content: ContentFile): Buffer | null {
    if (!content.content) {
      return null;
    }

    // Serialize content to JSON
    const contentStr = typeof content.content === 'string'
      ? content.content
      : JSON.stringify(content.content, null, 2);

    return Buffer.from(contentStr, 'utf-8');
  }

  /**
   * Compute the byte-profile CID (raw + blake3-256) of blob content.
   */
  computeCid(data: Buffer): string {
    return byteCid(data);
  }

  /**
   * Cache blob to local filesystem.
   */
  private cacheBlob(hash: string, data: Buffer): void {
    const cachePath = path.join(this.config.cacheDir, hash);
    fs.writeFileSync(cachePath, data);
  }

  /**
   * Get cached blob by hash.
   */
  getCachedBlob(hash: string): Buffer | null {
    const cachePath = path.join(this.config.cacheDir, hash);
    if (fs.existsSync(cachePath)) {
      return fs.readFileSync(cachePath);
    }
    return null;
  }

  /**
   * Get blob metadata by hash.
   */
  getBlobMetadata(hash: string): BlobMetadata | undefined {
    return this.blobCache.get(hash);
  }

  /**
   * Get all cached blob CIDs.
   */
  getCachedBlobCids(): string[] {
    return Array.from(this.blobCache.keys());
  }

  /**
   * Get statistics about cached blobs.
   */
  getStats(): {
    totalBlobs: number;
    totalBytes: number;
    byFormat: Record<string, number>;
  } {
    let totalBytes = 0;
    const byFormat: Record<string, number> = {};

    for (const [, metadata] of Array.from(this.blobCache.entries())) {
      totalBytes += metadata.sizeBytes;
      const format = metadata.mimeType;
      byFormat[format] = (byFormat[format] || 0) + 1;
    }

    return {
      totalBlobs: this.blobCache.size,
      totalBytes,
      byFormat,
    };
  }
}

// =============================================================================
// Validation Functions
// =============================================================================

/**
 * Validate blob address format (byte-profile CID).
 */
export function isValidBlobCid(cid: string): boolean {
  return isByteCid(cid);
}

/**
 * Validate content has required blob fields for its format.
 */
export function validateBlobReferences(content: ContentFile): {
  valid: boolean;
  errors: string[];
  warnings: string[];
} {
  const errors: string[] = [];
  const warnings: string[] = [];
  const format = content.contentFormat;

  if (!format || !BLOB_FORMATS.includes(format)) {
    return { valid: true, errors, warnings };
  }

  // For blob formats, should have either inline content OR blob reference
  const hasInlineContent = content.content !== undefined && content.content !== null;
  const hasBlobRef = content.blobCid !== undefined && content.blobCid !== null;

  if (!hasInlineContent && !hasBlobRef) {
    errors.push(`${format} content must have either 'content' or 'blobCid'`);
  }

  if (hasBlobRef) {
    if (!isValidBlobCid(content.blobCid!)) {
      errors.push(`Invalid blobCid format: ${content.blobCid}`);
    }

    // Warn if no fallback URL for external content
    if (!content.fallbackUrl) {
      warnings.push(`No fallbackUrl for blob content (recommended for resilience)`);
    }
  }

  // html5-app should have entryPoint
  if (format === 'html5-app' && hasBlobRef && !content.entryPoint) {
    warnings.push(`html5-app should have 'entryPoint' (defaulting to 'index.html')`);
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

export default BlobManager;
