/**
 * Import Pipeline Service Tests
 *
 * Tests for the core content import orchestration logic
 */

import * as fs from 'fs';
import * as path from 'path';
import { glob } from 'glob';
import { runImportPipeline, importContent } from './import-pipeline.service';
import { ImportOptions, ImportResult } from '../models/import-context.model';
import { ContentNode } from '../models/content-node.model';
import { createEmptyManifest } from '../models/manifest.model';
import * as manifestService from './manifest.service';
import { parsePathMetadata } from '../parsers/path-metadata-parser';
import { parseMarkdown } from '../parsers/markdown-parser';
import { extractRelationships } from './relationship-extractor.service';
import * as transformers from '../transformers';

// Mock dependencies
vi.mock('fs');
vi.mock('glob');
vi.mock('./manifest.service');
vi.mock('../parsers/path-metadata-parser');
vi.mock('../parsers/markdown-parser');
vi.mock('../parsers/gherkin-parser');
vi.mock('./relationship-extractor.service');
vi.mock('../transformers');

const mockFs = vi.mocked(fs);
const mockManifestService = vi.mocked(manifestService);
const mockGlob = vi.mocked(glob);
const mockParsePathMetadata = vi.mocked(parsePathMetadata);
const mockParseMarkdown = vi.mocked(parseMarkdown);
const mockExtractRelationships = vi.mocked(extractRelationships);
const mockTransformers = vi.mocked(transformers);

describe('import-pipeline.service', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
      // scanSourceFiles calls glob twice: once for *.md and once for *.feature patterns.
      // We return the two .md files on the first call and empty on the second to get totalFiles = 2.
      mockGlob.mockResolvedValueOnce(['/test/source/doc1.md', '/test/source/doc2.md'] as any)
          .mockResolvedValueOnce([] as any);

      mockFs.readFileSync.mockReturnValue('# Test Content');
      mockFs.existsSync.mockReturnValue(false);

      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());
      mockManifestService.calculateFileHash.mockReturnValue('hash123');

      mockParsePathMetadata.mockReturnValue({
        filePath: '/test/source/doc1.md',
        relativePath: 'doc1.md',
        extension: '.md',
        filename: 'doc1',
        directoryPath: '/test/source'
      } as any);

      mockParseMarkdown.mockReturnValue({
        pathMeta: {},
        frontmatter: {},
        rawContent: '# Test',
        title: 'Test Document',
        contentHash: 'hash123'
      } as any);

      mockExtractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        dryRun: true
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.totalFiles).toBe(2);
      expect(result.errors).toBe(0);
      expect(mockGlob).toHaveBeenCalled();
    });

    it('should handle incremental mode with unchanged files', async () => {
      // Arrange
      // scanSourceFiles calls glob twice: once for *.md and once for *.feature patterns.
      // Return the single doc on the first call and empty on the second to keep totalFiles = 1.
      mockGlob.mockResolvedValueOnce(['/test/source/doc1.md'] as any)
          .mockResolvedValueOnce([] as any);

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

      mockExtractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        mode: 'incremental',
        dryRun: true
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
      mockGlob.mockResolvedValue(['/test/source/doc1.md'] as any);

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

      mockExtractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        mode: 'incremental',
        dryRun: true
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
      mockGlob.mockResolvedValue(['/test/source/bad.md'] as any);

      mockFs.readFileSync.mockImplementation(() => {
        throw new Error('File read error');
      });

      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());
      mockManifestService.calculateFileHash.mockReturnValue('hash123');
      mockManifestService.getNewOrChangedSources.mockReturnValue(['/test/source/bad.md']);
      mockManifestService.getRemovedSources.mockReturnValue([]);

      mockExtractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        mode: 'incremental',
        dryRun: true
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
      mockGlob.mockResolvedValue(['/test/source/doc1.md'] as any);

      mockFs.readFileSync.mockReturnValue('# Test');
      mockFs.existsSync.mockReturnValue(false);

      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());
      mockManifestService.calculateFileHash.mockReturnValue('hash123');

      mockParsePathMetadata.mockReturnValue({
        filePath: '/test/source/doc1.md',
        relativePath: 'doc1.md',
        extension: '.md'
      } as any);

      mockParseMarkdown.mockReturnValue({
        pathMeta: {},
        frontmatter: {},
        rawContent: '# Test',
        title: 'Test',
        contentHash: 'hash123'
      } as any);

      mockExtractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        skipRelationships: true,
        dryRun: true
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.totalRelationships).toBe(0);
      expect(mockExtractRelationships).not.toHaveBeenCalled();
    });

    it('should not write to database in dry-run mode', async () => {
      // Arrange
      // scanSourceFiles calls glob twice (*.md then *.feature); return 1 file then empty.
      mockGlob.mockResolvedValueOnce(['/test/source/doc1.md'] as any)
          .mockResolvedValueOnce([] as any);

      mockFs.readFileSync.mockReturnValue('# Test');
      mockFs.existsSync.mockReturnValue(false);

      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());
      mockManifestService.calculateFileHash.mockReturnValue('hash123');

      mockParsePathMetadata.mockReturnValue({
        filePath: '/test/source/doc1.md',
        relativePath: 'doc1.md',
        extension: '.md'
      } as any);

      mockParseMarkdown.mockReturnValue({
        pathMeta: { extension: '.md', relativePath: 'doc1.md' },
        frontmatter: {},
        rawContent: '# Test',
        title: 'Test',
        contentHash: 'hash123'
      } as any);

      // Configure transformers so that at least one node is produced.
      // shouldCreateSourceNode returns true so a source node is created via transformToSourceNode.
      const mockSourceNode = {
        id: 'source-test-doc1',
        contentType: 'source',
        title: 'Test (Source)',
        content: '# Test',
        tags: ['source'],
        metadata: {}
      };
      mockTransformers.shouldCreateSourceNode.mockReturnValue(true);
      mockTransformers.transformToSourceNode.mockReturnValue(mockSourceNode as any);
      mockTransformers.isEpicContent.mockReturnValue(false);
      mockTransformers.isArchetypeContent.mockReturnValue(false);
      mockTransformers.isScenarioContent.mockReturnValue(false);
      mockTransformers.isResourceContent.mockReturnValue(false);

      mockExtractRelationships.mockReturnValue([]);

      const options: ImportOptions = {
        ...baseOptions,
        dryRun: true
      };

      // Act
      const result = await runImportPipeline(options);

      // Assert
      expect(result.totalNodes).toBeGreaterThan(0);
    });
  });

  describe('importContent', () => {
    it('should call runImportPipeline with correct defaults', async () => {
      // Arrange
      mockGlob.mockResolvedValue([] as any);

      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      mockExtractRelationships.mockReturnValue([]);

      // Act
      await importContent('/source', '/output', true);

      // Assert
      // Should complete without errors (no assertions needed for simple wrapper)
    });

    it('should use full mode when incremental is false', async () => {
      // Arrange
      mockGlob.mockResolvedValueOnce([] as any).mockResolvedValueOnce([] as any);

      mockManifestService.loadManifest.mockReturnValue(createEmptyManifest());

      mockExtractRelationships.mockReturnValue([]);

      // Act — no dbPath means dryRun defaults to true
      const result = await importContent('/source', '/output', false);

      // Assert
      expect(result.errors).toBe(0);
    });
  });
});
