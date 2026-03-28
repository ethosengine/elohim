import { describe, it, expect } from 'vitest';

import type { ContentView } from '../generated/content-view';

import {
  normalizeConductorContent,
  normalizeConductorContentBatch,
  isConductorResponse,
  ensureContentView,
  type ConductorContentResponse,
} from './conductor-normalizer';

// =============================================================================
// Test Fixtures
// =============================================================================

const createConductorResponse = (
  overrides?: Partial<ConductorContentResponse>
): ConductorContentResponse => ({
  id: 'concept-genesis-reading',
  content_type: 'concept',
  title: 'Genesis Reading',
  description: 'A foundational concept about the Genesis narrative',
  content: '# Genesis Reading\n\nIn the beginning...',
  content_format: 'markdown',
  tags: ['genesis', 'foundation'],
  source_path: 'elohim-protocol/genesis/reading',
  related_node_ids: ['concept-creation', 'concept-garden'],
  reach: 'commons',
  metadata_json: JSON.stringify({
    summary: 'Genesis reading overview',
    sourcePath: 'elohim-protocol/genesis/reading',
    estimatedMinutes: 15,
    thumbnailUrl: 'sha256-abc123',
    bloomsLevel: 'understand',
  }),
  blob_cid: null,
  blob_hash: null,
  content_size_bytes: null,
  created_at: '2026-03-15T10:00:00Z',
  updated_at: '2026-03-20T14:30:00Z',
  ...overrides,
});

const createContentView = (overrides?: Partial<ContentView>): ContentView => ({
  id: 'concept-genesis-reading',
  appId: 'lamad',
  title: 'Genesis Reading',
  description: 'A foundational concept about the Genesis narrative',
  contentType: 'concept',
  contentFormat: 'markdown',
  contentBody: '# Genesis Reading\n\nIn the beginning...',
  reach: 'commons',
  validationStatus: 'valid',
  createdAt: '2026-03-15T10:00:00Z',
  updatedAt: '2026-03-20T14:30:00Z',
  ...overrides,
});

// =============================================================================
// normalizeConductorContent
// =============================================================================

describe('normalizeConductorContent', () => {
  it('should map snake_case fields to camelCase', () => {
    const raw = createConductorResponse();
    const result = normalizeConductorContent(raw);

    expect(result.id).toBe('concept-genesis-reading');
    expect(result.contentType).toBe('concept');
    expect(result.contentFormat).toBe('markdown');
    expect(result.createdAt).toBe('2026-03-15T10:00:00Z');
    expect(result.updatedAt).toBe('2026-03-20T14:30:00Z');
  });

  it('should rename content to contentBody', () => {
    const raw = createConductorResponse({ content: '# Hello World' });
    const result = normalizeConductorContent(raw);

    expect(result.contentBody).toBe('# Hello World');
  });

  it('should parse metadata_json string into metadata object', () => {
    const metadata = { summary: 'Test', estimatedMinutes: 10, bloomsLevel: 'apply' };
    const raw = createConductorResponse({ metadata_json: JSON.stringify(metadata) });
    const result = normalizeConductorContent(raw);

    expect(result.metadata).toEqual(metadata);
  });

  it('should set metadata to undefined when metadata_json is null', () => {
    const raw = createConductorResponse({ metadata_json: null });
    const result = normalizeConductorContent(raw);

    expect(result.metadata).toBeUndefined();
  });

  it('should set metadata to undefined when metadata_json is empty string', () => {
    const raw = createConductorResponse({ metadata_json: '' });
    const result = normalizeConductorContent(raw);

    expect(result.metadata).toBeUndefined();
  });

  it('should set metadata to undefined when metadata_json is invalid JSON', () => {
    const raw = createConductorResponse({ metadata_json: 'not-json{{{' });
    const result = normalizeConductorContent(raw);

    expect(result.metadata).toBeUndefined();
  });

  it('should map content_size_bytes to contentSizeBytes', () => {
    const raw = createConductorResponse({ content_size_bytes: 4096 });
    const result = normalizeConductorContent(raw);

    expect(result.contentSizeBytes).toBe(4096);
  });

  it('should set contentSizeBytes to undefined when null', () => {
    const raw = createConductorResponse({ content_size_bytes: null });
    const result = normalizeConductorContent(raw);

    expect(result.contentSizeBytes).toBeUndefined();
  });

  it('should map blob_hash and blob_cid', () => {
    const raw = createConductorResponse({
      blob_hash: 'sha256-abc123',
      blob_cid: 'bafkrei-xyz789',
    });
    const result = normalizeConductorContent(raw);

    expect(result.blobHash).toBe('sha256-abc123');
    expect(result.blobCid).toBe('bafkrei-xyz789');
  });

  it('should set blob fields to undefined when null', () => {
    const raw = createConductorResponse({ blob_hash: null, blob_cid: null });
    const result = normalizeConductorContent(raw);

    expect(result.blobHash).toBeUndefined();
    expect(result.blobCid).toBeUndefined();
  });

  it('should set appId to lamad', () => {
    const raw = createConductorResponse();
    const result = normalizeConductorContent(raw);

    expect(result.appId).toBe('lamad');
  });

  it('should set validationStatus to valid', () => {
    const raw = createConductorResponse();
    const result = normalizeConductorContent(raw);

    expect(result.validationStatus).toBe('valid');
  });

  it('should set createdBy to undefined', () => {
    const raw = createConductorResponse();
    const result = normalizeConductorContent(raw);

    expect(result.createdBy).toBeUndefined();
  });

  it('should handle description null', () => {
    const raw = createConductorResponse({ description: null });
    const result = normalizeConductorContent(raw);

    expect(result.description).toBeUndefined();
  });

  it('should handle content null (blob-only nodes)', () => {
    const raw = createConductorResponse({
      content: null,
      blob_hash: 'sha256-abc',
      blob_cid: 'bafkrei-xyz',
    });
    const result = normalizeConductorContent(raw);

    expect(result.contentBody).toBeUndefined();
    expect(result.blobHash).toBe('sha256-abc');
  });

  it('should preserve reach value', () => {
    const raw = createConductorResponse({ reach: 'community' });
    const result = normalizeConductorContent(raw);

    expect(result.reach).toBe('community');
  });

  it('should return a valid ContentView (compile-time check)', () => {
    const raw = createConductorResponse();
    const result: ContentView = normalizeConductorContent(raw);

    // If this compiles, the return type matches ContentView
    expect(result).toBeDefined();
  });

  it('should handle path content type with epr-composite format', () => {
    const raw = createConductorResponse({
      content_type: 'path',
      content_format: 'epr-composite',
      content: JSON.stringify({ sections: [{ id: 's1', title: 'Chapter 1', items: [] }] }),
      metadata_json: JSON.stringify({
        pathType: 'course',
        difficulty: 'intermediate',
        thumbnailUrl: 'sha256-thumb',
      }),
    });
    const result = normalizeConductorContent(raw);

    expect(result.contentType).toBe('path');
    expect(result.contentFormat).toBe('epr-composite');
    expect(result.metadata).toEqual({
      pathType: 'course',
      difficulty: 'intermediate',
      thumbnailUrl: 'sha256-thumb',
    });
  });

  it('should handle assessment content type', () => {
    const raw = createConductorResponse({
      content_type: 'assessment',
      content_format: 'sophia-quiz-json',
      metadata_json: JSON.stringify({
        instrument: 'mastery-gate',
        scoringRules: { passingThreshold: 0.8 },
        mode: 'mastery',
      }),
    });
    const result = normalizeConductorContent(raw);

    expect(result.contentType).toBe('assessment');
    expect(result.contentFormat).toBe('sophia-quiz-json');
    expect((result.metadata as Record<string, unknown>)['instrument']).toBe('mastery-gate');
  });
});

// =============================================================================
// normalizeConductorContentBatch
// =============================================================================

describe('normalizeConductorContentBatch', () => {
  it('should normalize an array of responses', () => {
    const items = [
      createConductorResponse({ id: 'item-1', title: 'First' }),
      createConductorResponse({ id: 'item-2', title: 'Second' }),
      createConductorResponse({ id: 'item-3', title: 'Third' }),
    ];

    const results = normalizeConductorContentBatch(items);

    expect(results).toHaveLength(3);
    expect(results[0].id).toBe('item-1');
    expect(results[0].title).toBe('First');
    expect(results[1].id).toBe('item-2');
    expect(results[2].id).toBe('item-3');
    // All should have camelCase fields
    results.forEach(r => {
      expect(r.appId).toBe('lamad');
      expect(r.validationStatus).toBe('valid');
    });
  });

  it('should return empty array for empty input', () => {
    expect(normalizeConductorContentBatch([])).toEqual([]);
  });
});

// =============================================================================
// isConductorResponse
// =============================================================================

describe('isConductorResponse', () => {
  it('should detect conductor response (snake_case)', () => {
    const raw = createConductorResponse();
    expect(isConductorResponse(raw)).toBe(true);
  });

  it('should reject ContentView (camelCase)', () => {
    const view = createContentView();
    expect(isConductorResponse(view)).toBe(false);
  });

  it('should reject null', () => {
    expect(isConductorResponse(null)).toBe(false);
  });

  it('should reject undefined', () => {
    expect(isConductorResponse(undefined)).toBe(false);
  });

  it('should reject non-object', () => {
    expect(isConductorResponse('string')).toBe(false);
    expect(isConductorResponse(42)).toBe(false);
    expect(isConductorResponse(true)).toBe(false);
  });

  it('should reject object without content_type', () => {
    expect(isConductorResponse({ id: 'test', title: 'Missing type' })).toBe(false);
  });

  it('should reject object without id', () => {
    expect(isConductorResponse({ content_type: 'concept' })).toBe(false);
  });

  it('should reject object with non-string content_type', () => {
    expect(isConductorResponse({ id: 'test', content_type: 42 })).toBe(false);
  });
});

// =============================================================================
// ensureContentView
// =============================================================================

describe('ensureContentView', () => {
  it('should normalize conductor response', () => {
    const raw = createConductorResponse();
    const result = ensureContentView(raw);

    expect(result.contentType).toBe('concept');
    expect(result.appId).toBe('lamad');
  });

  it('should pass through ContentView as-is', () => {
    const view = createContentView();
    const result = ensureContentView(view);

    expect(result).toBe(view); // Same reference, not a copy
  });

  it('should handle mixed-format batch with map', () => {
    const conductorItem = createConductorResponse({ id: 'from-conductor' });
    const viewItem = createContentView({ id: 'from-storage' });

    const results = [conductorItem, viewItem].map(ensureContentView);

    expect(results[0].id).toBe('from-conductor');
    expect(results[0].appId).toBe('lamad');
    expect(results[1].id).toBe('from-storage');
  });
});
