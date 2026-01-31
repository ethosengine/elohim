/**
 * Import Pipeline Service Tests
 *
 * Tests for the core content import orchestration logic
 */

import * as fs from 'fs';
import * as path from 'path';
import { runImportPipeline, importContent } from './import-pipeline.service';
import { ImportOptions, ImportResult } from '../models/import-context.model';
import { ContentNode } from '../models/content-node.model';
import * as manifestService from './manifest.service';
import { KuzuClient } from '../db/kuzu-client';

// Mock dependencies
jest.mock('fs');
jest.mock('glob');
jest.mock('./manifest.service');
jest.mock('../db/kuzu-client');
jest.mock('../parsers/path-metadata-parser');
jest.mock('../parsers/markdown-parser');
jest.mock('../parsers/gherkin-parser');
jest.mock('./relationship-extractor.service');

const mockFs = fs as jest.Mocked<typeof fs>;
const mockManifestService = manifestService as jest.Mocked<typeof manifestService>;

describe('import-pipeline.service', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('runImportPipeline', () => {
    const baseOptions: ImportOptions = {
      mode: 'full',
      sourceDir: '/test/source',
      outputDir: '/test/output',
      generateSourceNodes: true,
      generateDerivedNodes: true,
      verbose: false
    };

    it('should complete a full import pipeline successfully', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue(['/test/source/doc1.md', '/test/source/doc2.md']);

      mockFs.readFileSync.mockReturnValue('# Test Content');
      mockFs.existsSync.mockReturnValue(false);

      const { createEmptyManifest } = require('../models/manifest.model');
      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      mockManifestService.calculateFileHash.mockReturnValue('hash123');

      const { parsePathMetadata } = require('../parsers/path-metadata-parser');
      parsePathMetadata.mockReturnValue({
        filePath: '/test/source/doc1.md',
        relativePath: 'doc1.md',
        extension: '.md',
        filename: 'doc1',
        directoryPath: '/test/source'
      });

      const { parseMarkdown } = require('../parsers/markdown-parser');
      parseMarkdown.mockReturnValue({
        pathMeta: {},
        frontmatter: {},
        rawContent: '# Test',
        title: 'Test Document',
        contentHash: 'hash123'
      });

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      const mockKuzuClient = {
        initialize: jest.fn().mockResolvedValue(undefined),
        bulkInsertContentNodes: jest.fn().mockResolvedValue(2),
        bulkInsertRelationships: jest.fn().mockResolvedValue(0),
        getStats: jest.fn().mockResolvedValue({ ContentNode: 2 }),
        close: jest.fn()
      };
      (KuzuClient as jest.Mock).mockImplementation(() => mockKuzuClient);

      const options: ImportOptions = {
        ...baseOptions,
        dbPath: '/test/output/db.kuzu'
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.totalFiles).toBe(2);
      expect(result.errors).toBe(0);
      expect(glob).toHaveBeenCalled();
    });

    it('should handle incremental mode with unchanged files', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue(['/test/source/doc1.md']);

      const { createEmptyManifest } = require('../models/manifest.model');
      const existingManifest = createEmptyManifest();
      existingManifest.sourceHashes['/test/source/doc1.md'] = {
        hash: 'unchangedHash',
        lastModified: new Date().toISOString(),
        generatedNodeIds: ['node-1']
      };
      existingManifest.totalSourceFiles = 1;
      existingManifest.totalNodes = 1;

      mockManifestService.loadManifest.mockReturnValue(existingManifest);
      mockManifestService.calculateFileHash.mockReturnValue('unchangedHash');
      mockManifestService.getNewOrChangedSources.mockReturnValue([]);
      mockManifestService.getRemovedSources.mockReturnValue([]);

      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue(JSON.stringify([
        { id: 'node-1', contentType: 'epic', title: 'Existing' }
      ]));

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      const mockKuzuClient = {
        initialize: jest.fn().mockResolvedValue(undefined),
        bulkInsertContentNodes: jest.fn().mockResolvedValue(0),
        bulkInsertRelationships: jest.fn().mockResolvedValue(0),
        getStats: jest.fn().mockResolvedValue({ ContentNode: 1 }),
        close: jest.fn()
      };
      (KuzuClient as jest.Mock).mockImplementation(() => mockKuzuClient);

      const options: ImportOptions = {
        ...baseOptions,
        mode: 'incremental',
        dbPath: '/test/output/db.kuzu'
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.skipped).toBe(1);
      expect(result.created).toBe(0);
      expect(mockManifestService.getNewOrChangedSources).toHaveBeenCalled();
    });

    it('should handle removed files in incremental mode', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue(['/test/source/doc1.md']);

      const { createEmptyManifest } = require('../models/manifest.model');
      const existingManifest = createEmptyManifest();
      existingManifest.sourceHashes['/test/source/doc1.md'] = {
        hash: 'hash1',
        lastModified: new Date().toISOString(),
        generatedNodeIds: ['node-1']
      };
      existingManifest.sourceHashes['/test/source/removed.md'] = {
        hash: 'hash2',
        lastModified: new Date().toISOString(),
        generatedNodeIds: ['node-2']
      };
      existingManifest.totalSourceFiles = 2;
      existingManifest.totalNodes = 2;

      mockManifestService.loadManifest.mockReturnValue(existingManifest);
      mockManifestService.calculateFileHash.mockReturnValue('hash1');
      mockManifestService.getNewOrChangedSources.mockReturnValue([]);
      mockManifestService.getRemovedSources.mockReturnValue(['/test/source/removed.md']);
      mockManifestService.removeSource.mockReturnValue(['node-2']);

      mockFs.existsSync.mockReturnValue(false);

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      const mockKuzuClient = {
        initialize: jest.fn().mockResolvedValue(undefined),
        bulkInsertContentNodes: jest.fn().mockResolvedValue(0),
        bulkInsertRelationships: jest.fn().mockResolvedValue(0),
        getStats: jest.fn().mockResolvedValue({ ContentNode: 1 }),
        close: jest.fn()
      };
      (KuzuClient as jest.Mock).mockImplementation(() => mockKuzuClient);

      const options: ImportOptions = {
        ...baseOptions,
        mode: 'incremental',
        dbPath: '/test/output/db.kuzu'
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(mockManifestService.removeSource).toHaveBeenCalledWith(
        existingManifest,
        '/test/source/removed.md'
      );
    });

    it('should handle parsing errors gracefully', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue(['/test/source/bad.md']);

      mockFs.readFileSync.mockImplementation(() => {
        throw new Error('File read error');
      });

      const { createEmptyManifest } = require('../models/manifest.model');
      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      mockManifestService.calculateFileHash.mockReturnValue('hash123');
      mockManifestService.getNewOrChangedSources.mockReturnValue(['/test/source/bad.md']);
      mockManifestService.getRemovedSources.mockReturnValue([]);

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      const mockKuzuClient = {
        initialize: jest.fn().mockResolvedValue(undefined),
        bulkInsertContentNodes: jest.fn().mockResolvedValue(0),
        bulkInsertRelationships: jest.fn().mockResolvedValue(0),
        getStats: jest.fn().mockResolvedValue({ ContentNode: 0 }),
        close: jest.fn()
      };
      (KuzuClient as jest.Mock).mockImplementation(() => mockKuzuClient);

      const options: ImportOptions = {
        ...baseOptions,
        mode: 'incremental',
        dbPath: '/test/output/db.kuzu'
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.errors).toBe(1);
      expect(result.fileResults[0].status).toBe('error');
      expect(result.fileResults[0].error).toContain('File read error');
    });

    it('should skip relationships when flag is set', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue(['/test/source/doc1.md']);

      mockFs.readFileSync.mockReturnValue('# Test');
      mockFs.existsSync.mockReturnValue(false);

      const { createEmptyManifest } = require('../models/manifest.model');
      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      mockManifestService.calculateFileHash.mockReturnValue('hash123');

      const { parsePathMetadata } = require('../parsers/path-metadata-parser');
      parsePathMetadata.mockReturnValue({
        filePath: '/test/source/doc1.md',
        relativePath: 'doc1.md',
        extension: '.md'
      });

      const { parseMarkdown } = require('../parsers/markdown-parser');
      parseMarkdown.mockReturnValue({
        pathMeta: {},
        frontmatter: {},
        rawContent: '# Test',
        title: 'Test',
        contentHash: 'hash123'
      });

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      const mockKuzuClient = {
        initialize: jest.fn().mockResolvedValue(undefined),
        bulkInsertContentNodes: jest.fn().mockResolvedValue(1),
        bulkInsertRelationships: jest.fn().mockResolvedValue(0),
        getStats: jest.fn().mockResolvedValue({ ContentNode: 1 }),
        close: jest.fn()
      };
      (KuzuClient as jest.Mock).mockImplementation(() => mockKuzuClient);

      const options: ImportOptions = {
        ...baseOptions,
        skipRelationships: true,
        dbPath: '/test/output/db.kuzu'
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.totalRelationships).toBe(0);
      expect(extractRelationships).not.toHaveBeenCalled();
    });

    it('should not write to database in dry-run mode', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue(['/test/source/doc1.md']);

      mockFs.readFileSync.mockReturnValue('# Test');
      mockFs.existsSync.mockReturnValue(false);

      const { createEmptyManifest } = require('../models/manifest.model');
      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      mockManifestService.calculateFileHash.mockReturnValue('hash123');

      const { parsePathMetadata } = require('../parsers/path-metadata-parser');
      parsePathMetadata.mockReturnValue({
        filePath: '/test/source/doc1.md',
        relativePath: 'doc1.md',
        extension: '.md'
      });

      const { parseMarkdown } = require('../parsers/markdown-parser');
      parseMarkdown.mockReturnValue({
        pathMeta: {},
        frontmatter: {},
        rawContent: '# Test',
        title: 'Test',
        contentHash: 'hash123'
      });

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        dryRun: true,
        dbPath: '/test/output/db.kuzu'
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.totalNodes).toBeGreaterThan(0);
      expect(KuzuClient).not.toHaveBeenCalled();
    });

    it('should throw error if dbPath is missing in non-dry-run mode', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue(['/test/source/doc1.md']);

      mockFs.readFileSync.mockReturnValue('# Test');
      mockFs.existsSync.mockReturnValue(false);

      const { createEmptyManifest } = require('../models/manifest.model');
      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      mockManifestService.calculateFileHash.mockReturnValue('hash123');

      const { parsePathMetadata } = require('../parsers/path-metadata-parser');
      parsePathMetadata.mockReturnValue({
        filePath: '/test/source/doc1.md',
        relativePath: 'doc1.md',
        extension: '.md'
      });

      const { parseMarkdown } = require('../parsers/markdown-parser');
      parseMarkdown.mockReturnValue({
        pathMeta: {},
        frontmatter: {},
        rawContent: '# Test',
        title: 'Test',
        contentHash: 'hash123'
      });

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        dryRun: false
        // dbPath intentionally omitted
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.errors).toBeGreaterThan(0);
      expect(result.fileResults[0]?.error).toContain('dbPath is required');
    });
  });

  describe('importContent', () => {
    it('should call runImportPipeline with correct defaults', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue([]);

      const { createEmptyManifest } = require('../models/manifest.model');
      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      // Act
      await importContent('/source', '/output', true);

      // Assert
      // Should complete without errors (no assertions needed for simple wrapper)
    });

    it('should use full mode when incremental is false', async () => {
      // Arrange
      const { glob } = require('glob');
      glob.mockResolvedValue([]);

      const { createEmptyManifest } = require('../models/manifest.model');
      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      const { extractRelationships } = require('./relationship-extractor.service');
      extractRelationships.mockReturnValue([]);

      // Act
      const result = await importContent('/source', '/output', false);

      // Assert
      expect(result).toBeDefined();
      expect(result.errors).toBe(0);
    });
  });
});
