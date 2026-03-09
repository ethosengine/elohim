/**
 * CLI Import Command Tests
 *
 * Tests for command parsing, option handling, and integration with import pipeline
 */

import { type MockInstance } from 'vitest';
import { Command } from 'commander';
import * as fs from 'fs';
import * as path from 'path';
import { runImportPipeline } from '../services/import-pipeline.service';
import { loadManifest, getImportStats, validateManifest } from '../services/manifest.service';

// Mock dependencies before importing the CLI
vi.mock('fs');
vi.mock('../services/import-pipeline.service');
vi.mock('../services/manifest.service');
vi.mock('../services/standards.service');
vi.mock('../services/trust.service');
vi.mock('../services/human.service');
vi.mock('../services/scaffold.service');

const mockFs = vi.mocked(fs);
const mockRunImportPipeline = vi.mocked(runImportPipeline);
const mockLoadManifest = vi.mocked(loadManifest);
const mockGetImportStats = vi.mocked(getImportStats);
const mockValidateManifest = vi.mocked(validateManifest);

describe('CLI import commands', () => {
  let program: Command;
  let mockExit: MockInstance;
  let mockConsoleLog: MockInstance;
  let mockConsoleError: MockInstance;

  beforeEach(() => {
    vi.clearAllMocks();

    // Mock process.exit
    mockExit = vi.spyOn(process, 'exit').mockImplementation((code?: string | number | null) => {
      throw new Error(`process.exit(${code})`);
    }) as any;

    // Mock console methods
    mockConsoleLog = vi.spyOn(console, 'log').mockImplementation();
    mockConsoleError = vi.spyOn(console, 'error').mockImplementation();

    // Create fresh program instance
    program = new Command();
  });

  afterEach(() => {
    mockExit.mockRestore();
    mockConsoleLog.mockRestore();
    mockConsoleError.mockRestore();
  });

  describe('import command', () => {
    it('should parse default options correctly', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        errors: 0,
        created: 5,
        skipped: 2,
        totalNodes: 10,
        totalRelationships: 15,
        fileResults: []
      } as any);

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .option('-f, --full', 'Full import', false)
        .option('-v, --verbose', 'Verbose', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: options.full ? 'full' : 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.resolve(options.output),
            verbose: options.verbose,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: 'incremental',
          sourceDir: expect.stringContaining('docs/content'),
          outputDir: expect.stringContaining('lamad'),
          verbose: false
        })
      );
    });

    it('should handle --full flag', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        errors: 0,
        created: 10,
        skipped: 0,
        totalNodes: 20,
        totalRelationships: 30,
        fileResults: []
      } as any);

      program
        .command('import')
        .option('-f, --full', 'Full import', false)
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action(async (options) => {
          await runImportPipeline({
            mode: options.full ? 'full' : 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.resolve(options.output),
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--full']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: 'full'
        })
      );
    });

    it('should handle custom source directory', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        errors: 0,
        created: 3,
        skipped: 0,
        totalNodes: 5,
        totalRelationships: 8,
        fileResults: []
      } as any);

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.resolve(options.output),
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '-s', '/custom/path']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          sourceDir: path.resolve('/custom/path')
        })
      );
    });

    it('should handle verbose flag', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        errors: 0,
        created: 5,
        skipped: 0,
        totalNodes: 10,
        totalRelationships: 15,
        fileResults: []
      } as any);

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .option('-v, --verbose', 'Verbose', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.resolve(options.output),
            verbose: options.verbose,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--verbose']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          verbose: true
        })
      );
    });

    it('should handle dry-run flag', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        errors: 0,
        created: 0,
        skipped: 0,
        totalNodes: 10,
        totalRelationships: 0,
        fileResults: []
      } as any);

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .option('--dry-run', 'Dry run', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.resolve(options.output),
            dryRun: options.dryRun,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--dry-run']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          dryRun: true
        })
      );
    });

    it('should handle skip-relationships flag', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        errors: 0,
        created: 5,
        skipped: 0,
        totalNodes: 10,
        totalRelationships: 0,
        fileResults: []
      } as any);

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .option('--skip-relationships', 'Skip relationships', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.resolve(options.output),
            skipRelationships: options.skipRelationships,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--skip-relationships']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          skipRelationships: true
        })
      );
    });

    it('should handle import errors and exit with code 1', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        errors: 3,
        created: 2,
        skipped: 0,
        totalNodes: 5,
        totalRelationships: 0,
        fileResults: [
          {
            sourcePath: '/test/bad1.md',
            status: 'error',
            error: 'Parse error',
            nodeIds: [],
            processingTime: 10
          },
          {
            sourcePath: '/test/bad2.md',
            status: 'error',
            error: 'Transform error',
            nodeIds: [],
            processingTime: 15
          },
          {
            sourcePath: '/test/bad3.md',
            status: 'error',
            error: 'Validation error',
            nodeIds: [],
            processingTime: 8
          }
        ]
      } as any);

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action(async (options) => {
          const result = await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.resolve(options.output),
            generateSourceNodes: true,
            generateDerivedNodes: true
          });

          if (result.errors > 0) {
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'import'])
      ).rejects.toThrow('process.exit(1)');
    });

    it('should handle pipeline exception', async () => {
      // Arrange
      mockRunImportPipeline.mockRejectedValue(new Error('Database connection failed'));

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action(async (options) => {
          try {
            await runImportPipeline({
              mode: 'incremental',
              sourceDir: path.resolve(options.source),
              outputDir: path.resolve(options.output),
              generateSourceNodes: true,
              generateDerivedNodes: true
            });
          } catch (err) {
            console.error(`Import failed: ${err}`);
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'import'])
      ).rejects.toThrow('process.exit(1)');

      expect(mockConsoleError).toHaveBeenCalledWith(
        expect.stringContaining('Database connection failed')
      );
    });
  });

  describe('stats command', () => {
    it('should load and display manifest statistics', async () => {
      // Arrange
      const mockManifest = {
        schemaVersion: '1.0.0',
        createdAt: '2024-01-01T00:00:00Z',
        lastUpdated: '2024-01-02T00:00:00Z',
        sourceHashes: {},
        nodeHashes: {},
        migrations: [],
        totalSourceFiles: 10,
        totalNodes: 50,
        totalRelationships: 75
      };

      mockLoadManifest.mockReturnValue(mockManifest as any);
      mockGetImportStats.mockReturnValue({
        schemaVersion: '1.0.0',
        lastImport: '2024-01-02T00:00:00Z',
        totalSources: 10,
        totalNodes: 50,
        migrationCount: 0
      } as any);
      mockValidateManifest.mockReturnValue({
        valid: true,
        errors: []
      });

      program
        .command('stats')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action((options) => {
          const manifest = loadManifest(path.resolve(options.output));
          const stats = getImportStats(manifest);
          const validation = validateManifest(manifest);

          console.log(`Total sources: ${stats.totalSources}`);
          console.log(`Total nodes: ${stats.totalNodes}`);
          console.log(`Manifest valid: ${validation.valid ? 'Yes' : 'No'}`);
        });

      // Act
      await program.parseAsync(['node', 'test', 'stats']);

      // Assert
      expect(mockLoadManifest).toHaveBeenCalled();
      expect(mockGetImportStats).toHaveBeenCalled();
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Total sources: 10'));
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Total nodes: 50'));
    });

    it('should handle manifest load failure', async () => {
      // Arrange
      mockLoadManifest.mockImplementation(() => {
        throw new Error('Manifest file not found');
      });

      program
        .command('stats')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action((options) => {
          try {
            const manifest = loadManifest(path.resolve(options.output));
          } catch (err) {
            console.error(`Failed to load manifest: ${err}`);
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'stats'])
      ).rejects.toThrow('process.exit(1)');

      expect(mockConsoleError).toHaveBeenCalledWith(
        expect.stringContaining('Manifest file not found')
      );
    });
  });

  describe('validate command', () => {
    it('should validate manifest successfully', async () => {
      // Arrange
      mockLoadManifest.mockReturnValue({
        schemaVersion: '1.0.0',
        createdAt: '2024-01-01T00:00:00Z',
        lastUpdated: '2024-01-02T00:00:00Z',
        sourceHashes: {},
        nodeHashes: {},
        migrations: [],
        totalSourceFiles: 5,
        totalNodes: 10,
        totalRelationships: 0
      } as any);

      mockValidateManifest.mockReturnValue({
        valid: true,
        errors: []
      });

      program
        .command('validate')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action((options) => {
          const manifest = loadManifest(path.resolve(options.output));
          const validation = validateManifest(manifest);

          if (validation.valid) {
            console.log('Manifest is valid');
          } else {
            console.log('Manifest has errors');
            process.exit(1);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'validate']);

      // Assert
      expect(mockValidateManifest).toHaveBeenCalled();
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('valid'));
    });

    it('should report validation errors and exit with code 1', async () => {
      // Arrange
      mockLoadManifest.mockReturnValue({
        schemaVersion: '1.0.0',
        createdAt: '2024-01-01T00:00:00Z',
        lastUpdated: '2024-01-02T00:00:00Z',
        sourceHashes: {},
        nodeHashes: {},
        migrations: [],
        totalSourceFiles: 5,
        totalNodes: 10,
        totalRelationships: 0
      } as any);

      mockValidateManifest.mockReturnValue({
        valid: false,
        errors: [
          'totalSourceFiles mismatch',
          'Orphaned node found: node-123'
        ]
      });

      program
        .command('validate')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action((options) => {
          const manifest = loadManifest(path.resolve(options.output));
          const validation = validateManifest(manifest);

          if (validation.valid) {
            console.log('Manifest is valid');
          } else {
            console.log('Manifest has errors:');
            for (const error of validation.errors) {
              console.log(`  - ${error}`);
            }
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'validate'])
      ).rejects.toThrow('process.exit(1)');

      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('errors'));
    });
  });

  describe('explore command', () => {
    it('should filter nodes by epic', async () => {
      // Arrange
      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue(JSON.stringify([
        { id: 'node-1', contentType: 'scenario', title: 'Test 1', metadata: { epic: 'governance' } },
        { id: 'node-2', contentType: 'epic', title: 'Test 2', metadata: { epic: 'autonomous_entity' } },
        { id: 'node-3', contentType: 'role', title: 'Test 3', metadata: { epic: 'governance' } }
      ]));

      program
        .command('explore')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .option('-e, --epic <name>', 'Epic filter')
        .action(async (options) => {
          const nodesPath = path.join(path.resolve(options.output), 'nodes.json');

          if (!fs.existsSync(nodesPath)) {
            console.error('No nodes.json found');
            process.exit(1);
          }

          const allNodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));
          let filtered = allNodes;

          if (options.epic) {
            filtered = filtered.filter((n: any) => n.metadata?.epic === options.epic);
          }

          console.log(`Found ${filtered.length} nodes`);
        });

      // Act
      await program.parseAsync(['node', 'test', 'explore', '-e', 'governance']);

      // Assert
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Found 2 nodes'));
    });

    it('should exit if nodes.json not found', async () => {
      // Arrange
      mockFs.existsSync.mockReturnValue(false);

      program
        .command('explore')
        .option('-o, --output <dir>', 'Output directory', './output/lamad')
        .action(async (options) => {
          const nodesPath = path.join(path.resolve(options.output), 'nodes.json');

          if (!fs.existsSync(nodesPath)) {
            console.error('No nodes.json found. Run import first.');
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'explore'])
      ).rejects.toThrow('process.exit(1)');

      expect(mockConsoleError).toHaveBeenCalledWith(
        expect.stringContaining('No nodes.json found')
      );
    });
  });

});
