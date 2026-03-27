/**
 * Content extension types — blob distribution and stewardship.
 *
 * These are app-layer types (not from protocol schema or generated code).
 * They extend ContentNode with capabilities for large media and
 * graduated content stewardship.
 */

/**
 * ContentBlob - Reference to large binary media for P2P distribution.
 *
 * Blobs are NOT stored in DHT (too large). Instead, ContentBlob stores:
 * - Cryptographic hash for integrity verification
 * - Size for cache planning
 * - Fallback URLs for resilience
 * - Bitrate variants for adaptive streaming
 *
 * Blobs are distributed via:
 * - HTTP Range requests (resume)
 * - HLS/DASH streaming (adaptive)
 * - Custodian network replication (P2P)
 */
export interface ContentBlob {
  /** Cryptographic hash of blob (SHA256 hex string) */
  hash: string;

  /** Size in bytes - used for cache allocation and streaming decisions */
  sizeBytes: number;

  /** MIME type (e.g., "video/mp4", "audio/mpeg", "application/pdf") */
  mimeType: string;

  /** Primary + fallback URLs for resilience (try in order) */
  fallbackUrls: string[];

  /** Bitrate in Mbps (useful for codec/quality tracking) */
  bitrateMbps?: number;

  /** Duration in seconds (for audio/video) */
  durationSeconds?: number;

  /** Codec information (H.264, H.265, VP9, AV1, AAC, OPUS, etc.) */
  codec?: string;

  /** Resolutions/bitrate variants for adaptive streaming */
  variants?: ContentBlobVariant[];

  /** Subtitle/caption tracks */
  captions?: ContentBlobCaption[];

  /** When this blob was created */
  createdAt?: string;

  /** When this blob was last verified/updated */
  verifiedAt?: string;
}

/**
 * Variant of a blob for adaptive streaming (e.g., 480p, 720p, 1080p, 4K).
 */
export interface ContentBlobVariant {
  /** Resolution (e.g., "1080p", "720p", "480p") or bitrate (e.g., "5000k") */
  label: string;

  /** Bitrate in Mbps */
  bitrateMbps: number;

  /** Width in pixels (for video) */
  width?: number;

  /** Height in pixels (for video) */
  height?: number;

  /** Fallback URLs for this variant (same structure as parent) */
  fallbackUrls: string[];

  /** Hash of this variant for verification */
  hash: string;

  /** Size in bytes */
  sizeBytes: number;
}

/**
 * Subtitle or caption track for media.
 */
export interface ContentBlobCaption {
  /** Language code (ISO 639-1: "en", "es", "fr", etc.) */
  language: string;

  /** Human-readable label ("English", "Spanish", "French with SDH") */
  label: string;

  /** Format (webvtt, srt, vtt, etc.) */
  format: 'webvtt' | 'srt' | 'vtt' | 'ass' | 'ssa';

  /** URL to caption file */
  url: string;

  /** Whether captions include hearing impaired info */
  isHardOfHearing?: boolean;
}

/**
 * ContentSteward - A human's stewardship relationship with content.
 *
 * Stewardship is graduated, not binary. The author typically has highest
 * affinity, but curators, translators, and endorsers all have stewardship
 * relationships with different weights.
 *
 * Affinity determines:
 * - Which conductor is "home" for this content (highest affinity steward)
 * - How REA value flows proportionally back to stewards
 * - Replication priority (higher affinity = earlier sync)
 */
export interface ContentSteward {
  /** Reference to a genesis human ID */
  humanId: string;

  /** Strength of stewardship relationship (0-1) */
  affinity: number;

  /** What kind of stewardship */
  role: StewardshipRole;
}

/**
 * StewardshipRole - How this human relates to the content.
 */
export type StewardshipRole = 'author' | 'curator' | 'translator' | 'endorser' | 'steward';
