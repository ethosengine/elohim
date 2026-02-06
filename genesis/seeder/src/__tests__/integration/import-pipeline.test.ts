/**
 * Integration Test: Content Import Pipeline
 *
 * Tests end-to-end flow:
 * 1. Content file (markdown/feature) → Parse
 * 2. Parse → Transform to ContentNode
 * 3. ContentNode → Database write
 * 4. Database → Verify retrieval
 *
 * Uses real file fixtures, not mocks.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import * as fs from 'fs/promises';
import * as path from 'path';
import { DoorwayClient } from '../../doorway-client.js';
import { ProgressClient } from '../../progress-client.js';
import { StorageClient } from '../../storage-client.js';

// =============================================================================
// Test Fixtures
// =============================================================================

const FIXTURES_DIR = path.join(__dirname, '../fixtures');
const TEST_CONTENT_MD = path.join(FIXTURES_DIR, 'sample-content.md');
const TEST_SCENARIO_FEATURE = path.join(FIXTURES_DIR, 'sample-scenario.feature');

// =============================================================================
// Mock ContentNode Parser (simplified for testing)
// =============================================================================

interface ContentNode {
  id: string;
  title: string;
  contentType: string;
  contentFormat: string;
  contentBody: string;
  description?: string;
  tags: string[];
  reach: string;
  relatedNodeIds: string[];
}

/**
 * Simple markdown parser for test fixtures.
 * Extracts YAML-like metadata from markdown.
 */
function parseMarkdownToNode(filepath: string, content: string): ContentNode {
  const lines = content.split('\n');
  const metadata: Record<string, string> = {};
  let bodyStartIndex = 0;

  // Parse metadata
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith('**Type:**')) {
      metadata.type = line.replace('**Type:**', '').trim();
    } else if (line.startsWith('**Tags:**')) {
      metadata.tags = line.replace('**Tags:**', '').trim();
    } else if (line.startsWith('**Reach:**')) {
      metadata.reach = line.replace('**Reach:**', '').trim();
    } else if (line.startsWith('## Content')) {
      bodyStartIndex = i + 2;
      break;
    }
  }

  const title = lines[0].replace(/^#\s+/, '');
  const description = lines.find(l => l.startsWith('## Description'));
  const body = lines.slice(bodyStartIndex).join('\n');

  // Extract related content IDs
  const relatedSection = content.match(/## Related Content\n\n([\s\S]*?)(\n## |\n$|$)/);
  const relatedNodeIds: string[] = [];
  if (relatedSection) {
    const items = relatedSection[1].split('\n').filter(l => l.trim().startsWith('-'));
    items.forEach(item => {
      const id = item.replace(/^-\s*/, '').trim();
      if (id) relatedNodeIds.push(id);
    });
  }

  return {
    id: path.basename(filepath, path.extname(filepath)),
    title,
    contentType: metadata.type || 'concept',
    contentFormat: 'markdown',
    contentBody: body,
    description: description ? description.replace('## Description', '').trim() : undefined,
    tags: metadata.tags ? metadata.tags.split(',').map(t => t.trim()) : [],
    reach: metadata.reach || 'commons',
    relatedNodeIds,
  };
}

/**
 * Simple Gherkin parser for test fixtures.
 */
function parseGherkinToNode(filepath: string, content: string): ContentNode {
  const lines = content.split('\n');
  const metadata: Record<string, string> = {};

  // Parse metadata
  for (const line of lines) {
    if (line.startsWith('**Type:**')) {
      metadata.type = line.replace('**Type:**', '').trim();
    } else if (line.startsWith('**Tags:**')) {
      metadata.tags = line.replace('**Tags:**', '').trim();
    } else if (line.startsWith('**Reach:**')) {
      metadata.reach = line.replace('**Reach:**', '').trim();
    }
  }

  const title = lines[0].replace(/^#\s+/, '');
  const featureMatch = content.match(/Feature: (.*)/);
  const featureTitle = featureMatch ? featureMatch[1] : title;

  return {
    id: path.basename(filepath, path.extname(filepath)),
    title,
    contentType: metadata.type || 'scenario',
    contentFormat: 'gherkin',
    contentBody: content,
    description: featureTitle,
    tags: metadata.tags ? metadata.tags.split(',').map(t => t.trim()) : [],
    reach: metadata.reach || 'commons',
    relatedNodeIds: [],
  };
}

// =============================================================================
// Integration Tests
// =============================================================================

describe('Content Import Pipeline Integration', () => {
  let doorwayClient: DoorwayClient;
  let storageClient: StorageClient;
  let progressClient: ProgressClient;

  const DOORWAY_URL = process.env.DOORWAY_URL || 'http://localhost:3000';
  const STORAGE_URL = process.env.STORAGE_URL || 'http://localhost:8090';

  beforeAll(async () => {
    // Initialize clients
    doorwayClient = new DoorwayClient({
      baseUrl: DOORWAY_URL,
      storageUrl: STORAGE_URL,
      timeout: 10000,
      retries: 2,
    });

    storageClient = new StorageClient({
      baseUrl: STORAGE_URL,
      timeout: 10000,
      retries: 2,
    });

    progressClient = new ProgressClient({
      baseUrl: DOORWAY_URL,
      timeout: 10000,
    });
  });

  describe('1. File → Parse → Transform', () => {
    it('should parse markdown file to ContentNode', async () => {
      const content = await fs.readFile(TEST_CONTENT_MD, 'utf-8');
      const node = parseMarkdownToNode(TEST_CONTENT_MD, content);

      expect(node.id).toBe('sample-content');
      expect(node.title).toBe('Test Governance Epic');
      expect(node.contentType).toBe('epic');
      expect(node.contentFormat).toBe('markdown');
      expect(node.tags).toContain('governance');
      expect(node.tags).toContain('test');
      expect(node.reach).toBe('commons');
      expect(node.relatedNodeIds).toContain('governance-feature-1');
      expect(node.relatedNodeIds).toContain('governance-scenario-1');
      expect(node.contentBody).toContain('Governance is about');
    });

    it('should parse Gherkin file to ContentNode', async () => {
      const content = await fs.readFile(TEST_SCENARIO_FEATURE, 'utf-8');
      const node = parseGherkinToNode(TEST_SCENARIO_FEATURE, content);

      expect(node.id).toBe('sample-scenario');
      expect(node.title).toBe('Test Governance Scenario');
      expect(node.contentType).toBe('scenario');
      expect(node.contentFormat).toBe('gherkin');
      expect(node.tags).toContain('governance');
      expect(node.tags).toContain('decision-making');
      expect(node.contentBody).toContain('Feature: Community Decision Making');
      expect(node.contentBody).toContain('Scenario: Proposing a change');
    });

    it('should handle missing metadata gracefully', async () => {
      const minimalContent = '# Minimal Content\n\nThis is minimal.';
      const node = parseMarkdownToNode('minimal.md', minimalContent);

      expect(node.id).toBe('minimal');
      expect(node.title).toBe('Minimal Content');
      expect(node.contentType).toBe('concept'); // default
      expect(node.reach).toBe('commons'); // default
      expect(node.tags).toEqual([]);
      expect(node.relatedNodeIds).toEqual([]);
    });
  });

  describe('2. Transform → Database Write', () => {
    it('should write ContentNode to storage via Doorway', async () => {
      // Skip if not in integration test environment
      if (!process.env.RUN_INTEGRATION_TESTS) {
        console.log('  ⏭️  Skipping (set RUN_INTEGRATION_TESTS=1 to run)');
        return;
      }

      const content = await fs.readFile(TEST_CONTENT_MD, 'utf-8');
      const node = parseMarkdownToNode(TEST_CONTENT_MD, content);

      // Transform to storage format
      const item = {
        id: `test-${node.id}-${Date.now()}`,
        title: node.title,
        contentType: node.contentType,
        contentFormat: node.contentFormat,
        contentBody: node.contentBody,
        description: node.description,
        tags: node.tags,
        reach: node.reach,
        metadataJson: JSON.stringify({
          relatedNodeIds: node.relatedNodeIds,
          source: 'integration-test',
        }),
      };

      // Write to database
      const result = await doorwayClient.bulkCreateContent([item]);

      expect(result.inserted).toBeGreaterThanOrEqual(1);
      expect(result.errors).toEqual([]);
    }, 30000);

    it('should batch write multiple nodes efficiently', async () => {
      if (!process.env.RUN_INTEGRATION_TESTS) {
        console.log('  ⏭️  Skipping (set RUN_INTEGRATION_TESTS=1 to run)');
        return;
      }

      const mdContent = await fs.readFile(TEST_CONTENT_MD, 'utf-8');
      const featureContent = await fs.readFile(TEST_SCENARIO_FEATURE, 'utf-8');

      const node1 = parseMarkdownToNode(TEST_CONTENT_MD, mdContent);
      const node2 = parseGherkinToNode(TEST_SCENARIO_FEATURE, featureContent);

      const timestamp = Date.now();
      const items = [node1, node2].map((node, idx) => ({
        id: `test-batch-${timestamp}-${idx}`,
        title: node.title,
        contentType: node.contentType,
        contentFormat: node.contentFormat,
        contentBody: node.contentBody,
        description: node.description,
        tags: node.tags,
        reach: node.reach,
        metadataJson: JSON.stringify({ source: 'batch-test' }),
      }));

      const result = await doorwayClient.bulkCreateContent(items);

      expect(result.inserted).toBe(2);
      expect(result.errors).toEqual([]);
    }, 30000);
  });

  describe('3. Database → Verify Retrieval', () => {
    it('should retrieve written content by ID', async () => {
      if (!process.env.RUN_INTEGRATION_TESTS) {
        console.log('  ⏭️  Skipping (set RUN_INTEGRATION_TESTS=1 to run)');
        return;
      }

      const content = await fs.readFile(TEST_CONTENT_MD, 'utf-8');
      const node = parseMarkdownToNode(TEST_CONTENT_MD, content);

      const testId = `test-retrieve-${Date.now()}`;
      const item = {
        id: testId,
        title: node.title,
        contentType: node.contentType,
        contentFormat: node.contentFormat,
        contentBody: node.contentBody,
        tags: node.tags,
        reach: node.reach,
      };

      // Write
      await doorwayClient.bulkCreateContent([item]);

      // Retrieve
      const response = await fetch(`${DOORWAY_URL}/db/content/${testId}`);
      expect(response.ok).toBe(true);

      const retrieved = await response.json();
      expect(retrieved.id).toBe(testId);
      expect(retrieved.title).toBe(node.title);
      expect(retrieved.contentType).toBe(node.contentType);
    }, 30000);
  });

  describe('4. Error Propagation', () => {
    it('should propagate validation errors from storage layer', async () => {
      if (!process.env.RUN_INTEGRATION_TESTS) {
        console.log('  ⏭️  Skipping (set RUN_INTEGRATION_TESTS=1 to run)');
        return;
      }

      // Missing required fields
      const invalidItem = {
        id: `test-invalid-${Date.now()}`,
        // Missing title, contentType, etc.
        tags: [],
      };

      const result = await doorwayClient.bulkCreateContent([invalidItem as any]);

      // Should have errors
      expect(result.errors.length).toBeGreaterThan(0);
    }, 30000);

    it('should handle network failures gracefully', async () => {
      const badClient = new DoorwayClient({
        baseUrl: 'http://localhost:99999', // Invalid port
        timeout: 1000,
        retries: 1,
      });

      await expect(async () => {
        await badClient.bulkCreateContent([{
          id: 'test',
          title: 'Test',
          contentType: 'concept',
          contentFormat: 'markdown',
          contentBody: 'Test',
          tags: [],
        }]);
      }).rejects.toThrow();
    });
  });

  describe('5. Blob Storage Integration', () => {
    it('should store large content as blob and reference by hash', async () => {
      if (!process.env.RUN_INTEGRATION_TESTS) {
        console.log('  ⏭️  Skipping (set RUN_INTEGRATION_TESTS=1 to run)');
        return;
      }

      // Create large content (> 1MB to trigger blob storage)
      const largeContent = 'A'.repeat(1024 * 1024 * 2); // 2MB
      const blob = Buffer.from(largeContent, 'utf-8');

      // Push to storage
      const pushResult = await storageClient.pushBlob(blob, 'text/plain', 'commons');

      expect(pushResult.success).toBe(true);
      expect(pushResult.manifest).toBeDefined();
      expect(pushResult.manifest!.blob_hash).toBeDefined();

      // Create content referencing blob
      const item = {
        id: `test-blob-${Date.now()}`,
        title: 'Large Content Test',
        contentType: 'concept',
        contentFormat: 'markdown',
        blobHash: pushResult.manifest!.blob_hash,
        blobCid: pushResult.manifest!.blob_hash, // Same for now
        description: 'Large content stored as blob',
        tags: ['test', 'blob'],
        reach: 'commons',
      };

      const result = await doorwayClient.bulkCreateContent([item]);
      expect(result.inserted).toBe(1);
    }, 60000);
  });
});
