/**
 * Conductor Content Normalizer
 *
 * Transforms conductor zome responses (snake_case, metadata_json: string,
 * `content` field) into the ContentView shape (camelCase, parsed metadata,
 * `contentBody` field) used by the typed content pipeline.
 *
 * This is the ONE normalization point for conductor content responses.
 * All three connection strategies produce the same ContentView shape:
 *
 *   Conductor (snake_case) -> normalizeConductorContent() -> ContentView
 *   Storage HTTP (camelCase) -> already ContentView
 *   IndexedDB (pre-transformed) -> already ContentNode
 *
 * The existing transformZomeResponse() in zome-wire-types.ts handles
 * agent-centric data (mastery, practice pools, points). This normalizer
 * handles CONTENT specifically, mapping to the schema-generated ContentView.
 *
 * Doorway note: doorway's Rust resolver does NOT normalize conductor
 * responses — it passes them through as-is (see api.rs line 244:
 * "Return whatever the DNA returned — no interpretation"). When doorway
 * falls back to conductor, the Angular side must normalize.
 *
 * @packageDocumentation
 */

import type {
  ContentView,
  ContentType,
  ContentFormat,
  Reach,
  ValidationStatus,
} from '../generated/content-view';

// =============================================================================
// Conductor Wire Type
// =============================================================================

/**
 * Content record as returned by conductor zome calls (snake_case wire format).
 *
 * This matches the Rust `Content` struct serialized by the content_store zome.
 * Key differences from ContentView:
 * - snake_case field names (Rust default serde)
 * - `content` field (not `contentBody`)
 * - `metadata_json: string` (not parsed `metadata: unknown`)
 * - No `hAppId` (added by storage layer)
 * - No `validationStatus` (added by storage layer)
 * - No `dhtAnchorHash` (added by storage layer)
 */
export interface ConductorContentResponse {
  id: string;
  content_type: string;
  title: string;
  description: string | null;
  content: string | null;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  metadata_json: string | null;
  blob_cid: string | null;
  blob_hash: string | null;
  content_size_bytes: number | null;
  created_at: string;
  updated_at: string;
}

// =============================================================================
// Normalizer
// =============================================================================

/**
 * Normalize a conductor content response into the ContentView shape.
 *
 * Maps snake_case fields to camelCase, parses metadata_json, and renames
 * `content` to `contentBody`. Fields not present on the conductor wire
 * type are set to sensible defaults:
 * - `hAppId`: 'lamad' (the only content DNA today)
 * - `validationStatus`: 'valid' (conductor content is authoritative)
 *
 * @param raw - Conductor zome response in snake_case wire format
 * @returns ContentView matching the schema-generated contract
 */
export function normalizeConductorContent(raw: ConductorContentResponse): ContentView {
  return {
    id: raw.id,
    hAppId: 'lamad',
    title: raw.title,
    description: raw.description ?? undefined,
    contentType: raw.content_type as ContentType,
    contentFormat: raw.content_format as ContentFormat,
    contentBody: raw.content ?? undefined,
    blobHash: raw.blob_hash ?? undefined,
    blobCid: raw.blob_cid ?? undefined,
    contentSizeBytes: raw.content_size_bytes ?? undefined,
    metadata: parseMetadataJson(raw.metadata_json),
    reach: raw.reach as Reach,
    validationStatus: 'valid' as ValidationStatus,
    createdBy: undefined,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

/**
 * Normalize an array of conductor content responses.
 */
export function normalizeConductorContentBatch(items: ConductorContentResponse[]): ContentView[] {
  return items.map(normalizeConductorContent);
}

// =============================================================================
// Type Guard
// =============================================================================

/**
 * Detect whether a content response is in conductor wire format (snake_case)
 * or already in ContentView format (camelCase).
 *
 * Uses the presence of `content_type` (snake_case) as the discriminator.
 * ContentView has `contentType` (camelCase) instead.
 *
 * Useful when doorway falls back to conductor — the same HTTP endpoint can
 * return either format depending on which tier served the response.
 */
export function isConductorResponse(response: unknown): response is ConductorContentResponse {
  if (response === null || response === undefined || typeof response !== 'object') {
    return false;
  }
  const obj = response as Record<string, unknown>;
  return typeof obj['content_type'] === 'string' && typeof obj['id'] === 'string';
}

/**
 * Normalize a content response that may be in either conductor or ContentView format.
 *
 * If the response is already a ContentView (camelCase), returns it as-is.
 * If it's a conductor response (snake_case), normalizes it.
 *
 * This is the safe entry point when the source tier is unknown.
 */
export function ensureContentView(response: unknown): ContentView {
  if (isConductorResponse(response)) {
    return normalizeConductorContent(response);
  }
  return response as ContentView;
}

// =============================================================================
// Helpers
// =============================================================================

function parseMetadataJson(metadataJson: string | null): unknown {
  if (!metadataJson) {
    return undefined;
  }
  try {
    return JSON.parse(metadataJson);
  } catch {
    return undefined;
  }
}
