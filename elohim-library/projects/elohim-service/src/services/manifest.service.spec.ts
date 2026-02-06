/**
 * Manifest Service Tests
 *
 * Tests for manifest tracking, hashing, and incremental import logic
 */

import * as fs from 'fs';
import * as path from 'path';
import {
  loadManifest,
  saveManifest,
  calculateFileHash,
  calculateNodeHash,
  hasSourceChanged,
  updateSourceHash,
  updateNodeHash,
  getNewOrChangedSources,
  getRemovedSources,
  removeSource,
  getImportStats,
  validateManifest
} from './manifest.service';
import { ContentManifest, createEmptyManifest } from '../models/manifest.model';

jest.mock('fs');

const mockFs = fs as jest.Mocked<typeof fs>;

describe('manifest.service', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('loadManifest', () => {
    it('should load existing manifest from disk', () => {
      // Arrange
      const manifestData: ContentManifest = {
        manifestVersion: '1.0.0',
        schemaVersion: '1.0.0',
        lastUpdated: '2024-01-02T00:00:00Z',
        importToolVersion: '0.1.0',
        sourceHashes: {
          'test.md': {
            hash: 'abc123',
            lastModified: '2024-01-01T00:00:00Z',
            generatedNodeIds: ['node-1']
          }
        },
        nodeHashes: {},
        migrations: [],
        totalSourceFiles: 1,
        totalNodes: 1,
        totalRelationships: 0,
        domainStats: {},
        contentTypeStats: {}
      };

      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue(JSON.stringify(manifestData));

      // Act
      const result = loadManifest('/test/output');

      // Assert
      expect(result.schemaVersion).toBe('1.0.0');
      expect(result.totalSourceFiles).toBe(1);
      expect(mockFs.existsSync).toHaveBeenCalledWith('/test/output/content-manifest.json');
    });

    it('should return empty manifest if file does not exist', () => {
      // Arrange
      mockFs.existsSync.mockReturnValue(false);

      // Act
      const result = loadManifest('/test/output');

      // Assert
      expect(result.schemaVersion).toBeDefined();
      expect(result.totalSourceFiles).toBe(0);
      expect(Object.keys(result.sourceHashes)).toHaveLength(0);
    });

    it('should return empty manifest if JSON parsing fails', () => {
      // Arrange
      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue('invalid json');

      const consoleWarnSpy = jest.spyOn(console, 'warn').mockImplementation();

      // Act
      const result = loadManifest('/test/output');

      // Assert
      expect(result.totalSourceFiles).toBe(0);
      expect(consoleWarnSpy).toHaveBeenCalledWith(expect.stringContaining('Failed to load manifest'));

      consoleWarnSpy.mockRestore();
    });
  });

  describe('saveManifest', () => {
    it('should save manifest to disk with updated timestamp', () => {
      // Arrange
      const manifest = createEmptyManifest();
      mockFs.existsSync.mockReturnValue(true);
      mockFs.writeFileSync.mockImplementation();

      // Act
      saveManifest(manifest, '/test/output');

      // Assert
      expect(mockFs.writeFileSync).toHaveBeenCalledWith(
        '/test/output/content-manifest.json',
        expect.any(String),
        'utf-8'
      );

      const writtenData = (mockFs.writeFileSync as jest.Mock).mock.calls[0][1];
      const parsed = JSON.parse(writtenData);
      expect(parsed.lastUpdated).toBeDefined();
    });

    it('should create output directory if it does not exist', () => {
      // Arrange
      const manifest = createEmptyManifest();
      mockFs.existsSync.mockReturnValue(false);
      mockFs.mkdirSync.mockImplementation();
      mockFs.writeFileSync.mockImplementation();

      // Act
      saveManifest(manifest, '/test/output');

      // Assert
      expect(mockFs.mkdirSync).toHaveBeenCalledWith('/test/output', { recursive: true });
    });
  });

  describe('calculateFileHash', () => {
    it('should calculate SHA-256 hash of file content', () => {
      // Arrange
      mockFs.readFileSync.mockReturnValue('test content');

      // Act
      const hash = calculateFileHash('/test/file.md');

      // Assert
      expect(hash).toBeDefined();
      expect(hash).toHaveLength(64); // SHA-256 produces 64 hex chars
      expect(mockFs.readFileSync).toHaveBeenCalledWith('/test/file.md', 'utf-8');
    });

    it('should produce different hashes for different content', () => {
      // Arrange
      mockFs.readFileSync
        .mockReturnValueOnce('content A')
        .mockReturnValueOnce('content B');

      // Act
      const hashA = calculateFileHash('/test/a.md');
      const hashB = calculateFileHash('/test/b.md');

      // Assert
      expect(hashA).not.toBe(hashB);
    });

    it('should produce same hash for identical content', () => {
      // Arrange
      const content = 'identical content';
      mockFs.readFileSync.mockReturnValue(content);

      // Act
      const hash1 = calculateFileHash('/test/file1.md');
      const hash2 = calculateFileHash('/test/file2.md');

      // Assert
      expect(hash1).toBe(hash2);
    });
  });

  describe('calculateNodeHash', () => {
    it('should calculate hash based on id, content, and metadata', () => {
      // Arrange
      const node = {
        id: 'test-node',
        content: 'node content',
        metadata: { key: 'value' }
      };

      // Act
      const hash = calculateNodeHash(node);

      // Assert
      expect(hash).toBeDefined();
      expect(hash).toHaveLength(64);
    });

    it('should produce different hashes for different nodes', () => {
      // Arrange
      const node1 = { id: 'node-1', content: 'content', metadata: {} };
      const node2 = { id: 'node-2', content: 'content', metadata: {} };

      // Act
      const hash1 = calculateNodeHash(node1);
      const hash2 = calculateNodeHash(node2);

      // Assert
      expect(hash1).not.toBe(hash2);
    });

    it('should handle nodes without content or metadata', () => {
      // Arrange
      const node = { id: 'minimal-node' };

      // Act
      const hash = calculateNodeHash(node);

      // Assert
      expect(hash).toBeDefined();
      expect(hash).toHaveLength(64);
    });
  });

  describe('hasSourceChanged', () => {
    it('should return true for new files not in manifest', () => {
      // Arrange
      const manifest = createEmptyManifest();

      // Act
      const result = hasSourceChanged(manifest, '/test/new.md', 'hash123');

      // Assert
      expect(result).toBe(true);
    });

    it('should return true if hash differs from manifest', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['/test/file.md'] = {
        hash: 'oldHash',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: []
      };

      // Act
      const result = hasSourceChanged(manifest, '/test/file.md', 'newHash');

      // Assert
      expect(result).toBe(true);
    });

    it('should return false if hash matches manifest', () => {
      // Arrange
      const manifest = createEmptyManifest();
      const hash = 'unchangedHash';
      manifest.sourceHashes['/test/file.md'] = {
        hash,
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: []
      };

      // Act
      const result = hasSourceChanged(manifest, '/test/file.md', hash);

      // Assert
      expect(result).toBe(false);
    });
  });

  describe('updateSourceHash', () => {
    it('should add new source hash entry to manifest', () => {
      // Arrange
      const manifest = createEmptyManifest();

      // Act
      updateSourceHash(manifest, '/test/file.md', 'hash123', ['node-1', 'node-2']);

      // Assert
      expect(manifest.sourceHashes['/test/file.md']).toBeDefined();
      expect(manifest.sourceHashes['/test/file.md'].hash).toBe('hash123');
      expect(manifest.sourceHashes['/test/file.md'].generatedNodeIds).toEqual(['node-1', 'node-2']);
      expect(manifest.totalSourceFiles).toBe(1);
    });

    it('should update existing source hash entry', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['/test/file.md'] = {
        hash: 'oldHash',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: ['old-node']
      };
      manifest.totalSourceFiles = 1;

      // Act
      updateSourceHash(manifest, '/test/file.md', 'newHash', ['new-node']);

      // Assert
      expect(manifest.sourceHashes['/test/file.md'].hash).toBe('newHash');
      expect(manifest.sourceHashes['/test/file.md'].generatedNodeIds).toEqual(['new-node']);
      expect(manifest.totalSourceFiles).toBe(1); // Count stays the same
    });
  });

  describe('updateNodeHash', () => {
    it('should add node hash entry to manifest', () => {
      // Arrange
      const manifest = createEmptyManifest();

      // Act
      updateNodeHash(manifest, 'node-1', 'nodeHash123', '/test/source.md', 'epic');

      // Assert
      expect(manifest.nodeHashes['node-1']).toBeDefined();
      expect(manifest.nodeHashes['node-1'].hash).toBe('nodeHash123');
      expect(manifest.nodeHashes['node-1'].sourcePath).toBe('/test/source.md');
      expect(manifest.nodeHashes['node-1'].contentType).toBe('epic');
      expect(manifest.totalNodes).toBe(1);
    });

    it('should update total node count', () => {
      // Arrange
      const manifest = createEmptyManifest();

      // Act
      updateNodeHash(manifest, 'node-1', 'hash1', '/test/file1.md', 'epic');
      updateNodeHash(manifest, 'node-2', 'hash2', '/test/file2.md', 'scenario');

      // Assert
      expect(manifest.totalNodes).toBe(2);
    });
  });

  describe('getNewOrChangedSources', () => {
    it('should identify new source files', () => {
      // Arrange
      const manifest = createEmptyManifest();
      const sourceHashes = new Map([
        ['/test/new.md', 'hash123']
      ]);

      // Act
      const result = getNewOrChangedSources(manifest, sourceHashes);

      // Assert
      expect(result).toContain('/test/new.md');
      expect(result).toHaveLength(1);
    });

    it('should identify changed source files', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['/test/changed.md'] = {
        hash: 'oldHash',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: []
      };

      const sourceHashes = new Map([
        ['/test/changed.md', 'newHash']
      ]);

      // Act
      const result = getNewOrChangedSources(manifest, sourceHashes);

      // Assert
      expect(result).toContain('/test/changed.md');
      expect(result).toHaveLength(1);
    });

    it('should not include unchanged files', () => {
      // Arrange
      const manifest = createEmptyManifest();
      const hash = 'unchangedHash';
      manifest.sourceHashes['/test/unchanged.md'] = {
        hash,
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: []
      };

      const sourceHashes = new Map([
        ['/test/unchanged.md', hash]
      ]);

      // Act
      const result = getNewOrChangedSources(manifest, sourceHashes);

      // Assert
      expect(result).toHaveLength(0);
    });
  });

  describe('getRemovedSources', () => {
    it('should identify removed source files', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['/test/removed.md'] = {
        hash: 'hash123',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: ['node-1']
      };
      manifest.sourceHashes['/test/existing.md'] = {
        hash: 'hash456',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: ['node-2']
      };

      const currentSources = new Set(['/test/existing.md']);

      // Act
      const result = getRemovedSources(manifest, currentSources);

      // Assert
      expect(result).toContain('/test/removed.md');
      expect(result).not.toContain('/test/existing.md');
      expect(result).toHaveLength(1);
    });

    it('should return empty array if no sources removed', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['/test/file.md'] = {
        hash: 'hash123',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: []
      };

      const currentSources = new Set(['/test/file.md']);

      // Act
      const result = getRemovedSources(manifest, currentSources);

      // Assert
      expect(result).toHaveLength(0);
    });
  });

  describe('removeSource', () => {
    it('should remove source and return generated node IDs', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['/test/remove.md'] = {
        hash: 'hash123',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: ['node-1', 'node-2']
      };
      manifest.totalSourceFiles = 1;

      // Act
      const removedNodeIds = removeSource(manifest, '/test/remove.md');

      // Assert
      expect(removedNodeIds).toEqual(['node-1', 'node-2']);
      expect(manifest.sourceHashes['/test/remove.md']).toBeUndefined();
      expect(manifest.totalSourceFiles).toBe(0);
    });

    it('should remove associated node hashes', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['/test/remove.md'] = {
        hash: 'hash123',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: ['node-1', 'node-2']
      };
      manifest.nodeHashes['node-1'] = {
        hash: 'nhash1',
        sourcePath: '/test/remove.md',
        contentType: 'epic',
        generatedAt: '2024-01-01T00:00:00Z'
      };
      manifest.nodeHashes['node-2'] = {
        hash: 'nhash2',
        sourcePath: '/test/remove.md',
        contentType: 'scenario',
        generatedAt: '2024-01-01T00:00:00Z'
      };
      manifest.totalNodes = 2;

      // Act
      removeSource(manifest, '/test/remove.md');

      // Assert
      expect(manifest.nodeHashes['node-1']).toBeUndefined();
      expect(manifest.nodeHashes['node-2']).toBeUndefined();
      expect(manifest.totalNodes).toBe(0);
    });

    it('should return empty array if source not found', () => {
      // Arrange
      const manifest = createEmptyManifest();

      // Act
      const removedNodeIds = removeSource(manifest, '/test/nonexistent.md');

      // Assert
      expect(removedNodeIds).toEqual([]);
    });
  });

  describe('getImportStats', () => {
    it('should return correct statistics from manifest', () => {
      // Arrange
      const manifest: ContentManifest = {
        manifestVersion: '1.0.0',
        schemaVersion: '1.0.0',
        lastUpdated: '2024-01-02T00:00:00Z',
        importToolVersion: '0.1.0',
        sourceHashes: {
          'file1.md': {
            hash: 'hash1',
            lastModified: '2024-01-01T00:00:00Z',
            generatedNodeIds: ['node-1']
          }
        },
        nodeHashes: {
          'node-1': {
            hash: 'nhash1',
            sourcePath: 'file1.md',
            contentType: 'epic',
            generatedAt: '2024-01-01T00:00:00Z'
          }
        },
        migrations: [
          {
            id: 'migration-1',
            fromVersion: '0.9.0',
            toVersion: '1.0.0',
            appliedAt: '2024-01-01T00:00:00Z',
            nodesMigrated: 0,
            rules: []
          }
        ],
        totalSourceFiles: 1,
        totalNodes: 1,
        totalRelationships: 5,
        domainStats: {},
        contentTypeStats: {}
      };

      // Act
      const stats = getImportStats(manifest);

      // Assert
      expect(stats.schemaVersion).toBe('1.0.0');
      expect(stats.lastImport).toBe('2024-01-02T00:00:00Z');
      expect(stats.totalSources).toBe(1);
      expect(stats.totalNodes).toBe(1);
      expect(stats.migrationCount).toBe(1);
    });
  });

  describe('validateManifest', () => {
    it('should validate a correct manifest', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['test.md'] = {
        hash: 'hash123',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: ['node-1']
      };
      manifest.nodeHashes['node-1'] = {
        hash: 'nhash1',
        sourcePath: 'test.md',
        contentType: 'epic',
        generatedAt: '2024-01-01T00:00:00Z'
      };
      manifest.totalSourceFiles = 1;
      manifest.totalNodes = 1;

      // Act
      const validation = validateManifest(manifest);

      // Assert
      expect(validation.valid).toBe(true);
      expect(validation.errors).toHaveLength(0);
    });

    it('should detect mismatched totalSourceFiles count', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['test.md'] = {
        hash: 'hash123',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: []
      };
      manifest.totalSourceFiles = 5; // Incorrect count

      // Act
      const validation = validateManifest(manifest);

      // Assert
      expect(validation.valid).toBe(false);
      expect(validation.errors).toContain(
        expect.stringContaining('totalSourceFiles')
      );
    });

    it('should detect mismatched totalNodes count', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.nodeHashes['node-1'] = {
        hash: 'nhash1',
        sourcePath: 'test.md',
        contentType: 'epic',
        generatedAt: '2024-01-01T00:00:00Z'
      };
      manifest.totalNodes = 10; // Incorrect count

      // Act
      const validation = validateManifest(manifest);

      // Assert
      expect(validation.valid).toBe(false);
      expect(validation.errors).toContain(
        expect.stringContaining('totalNodes')
      );
    });

    it('should detect orphaned nodes not referenced by sources', () => {
      // Arrange
      const manifest = createEmptyManifest();
      manifest.sourceHashes['test.md'] = {
        hash: 'hash123',
        lastModified: '2024-01-01T00:00:00Z',
        generatedNodeIds: ['node-1']
      };
      manifest.nodeHashes['node-1'] = {
        hash: 'nhash1',
        sourcePath: 'test.md',
        contentType: 'epic',
        generatedAt: '2024-01-01T00:00:00Z'
      };
      manifest.nodeHashes['orphan-node'] = {
        hash: 'nhash2',
        sourcePath: 'missing.md',
        contentType: 'scenario',
        generatedAt: '2024-01-01T00:00:00Z'
      };
      manifest.totalSourceFiles = 1;
      manifest.totalNodes = 2;

      // Act
      const validation = validateManifest(manifest);

      // Assert
      expect(validation.valid).toBe(false);
      expect(validation.errors.some(e => e.includes('orphan-node'))).toBe(true);
    });
  });
});
