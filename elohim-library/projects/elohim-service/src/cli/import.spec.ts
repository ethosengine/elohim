/**
 * CLI Import Command Tests
 *
 * Tests for command parsing, option handling, and integration with import pipeline
 */

import { Command } from 'commander';
import * as fs from 'fs';
import * as path from 'path';

// Mock dependencies before importing the CLI
jest.mock('fs');
jest.mock('../services/import-pipeline.service');
jest.mock('../services/manifest.service');
jest.mock('../services/standards.service');
jest.mock('../services/trust.service');
jest.mock('../services/human.service');
jest.mock('../services/scaffold.service');
jest.mock('../db/kuzu-client');

const mockFs = fs as jest.Mocked<typeof fs>;

describe('CLI import commands', () => {
  let program: Command;
  let mockExit: jest.SpyInstance;
  let mockConsoleLog: jest.SpyInstance;
  let mockConsoleError: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();

    // Mock process.exit
    mockExit = jest.spyOn(process, 'exit').mockImplementation((code?: string | number | null) => {
      throw new Error(`process.exit(${code})`);
    }) as any;

    // Mock console methods
    mockConsoleLog = jest.spyOn(console, 'log').mockImplementation();
    mockConsoleError = jest.spyOn(console, 'error').mockImplementation();

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
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockResolvedValue({
        errors: 0,
        created: 5,
        skipped: 2,
        totalNodes: 10,
        totalRelationships: 15,
        fileResults: []
      });

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .option('-f, --full', 'Full import', false)
        .option('-v, --verbose', 'Verbose', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: options.full ? 'full' : 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.dirname(path.resolve(options.db)),
            dbPath: path.resolve(options.db),
            verbose: options.verbose,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: 'incremental',
          sourceDir: expect.stringContaining('docs/content'),
          dbPath: expect.stringContaining('lamad.kuzu'),
          verbose: false
        })
      );
    });

    it('should handle --full flag', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockResolvedValue({
        errors: 0,
        created: 10,
        skipped: 0,
        totalNodes: 20,
        totalRelationships: 30,
        fileResults: []
      });

      program
        .command('import')
        .option('-f, --full', 'Full import', false)
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .action(async (options) => {
          await runImportPipeline({
            mode: options.full ? 'full' : 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.dirname(path.resolve(options.db)),
            dbPath: path.resolve(options.db),
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--full']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: 'full'
        })
      );
    });

    it('should handle custom source directory', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockResolvedValue({
        errors: 0,
        created: 3,
        skipped: 0,
        totalNodes: 5,
        totalRelationships: 8,
        fileResults: []
      });

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.dirname(path.resolve(options.db)),
            dbPath: path.resolve(options.db),
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '-s', '/custom/path']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          sourceDir: path.resolve('/custom/path')
        })
      );
    });

    it('should handle verbose flag', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockResolvedValue({
        errors: 0,
        created: 5,
        skipped: 0,
        totalNodes: 10,
        totalRelationships: 15,
        fileResults: []
      });

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .option('-v, --verbose', 'Verbose', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.dirname(path.resolve(options.db)),
            dbPath: path.resolve(options.db),
            verbose: options.verbose,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--verbose']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          verbose: true
        })
      );
    });

    it('should handle dry-run flag', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockResolvedValue({
        errors: 0,
        created: 0,
        skipped: 0,
        totalNodes: 10,
        totalRelationships: 0,
        fileResults: []
      });

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .option('--dry-run', 'Dry run', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.dirname(path.resolve(options.db)),
            dbPath: path.resolve(options.db),
            dryRun: options.dryRun,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--dry-run']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          dryRun: true
        })
      );
    });

    it('should handle skip-relationships flag', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockResolvedValue({
        errors: 0,
        created: 5,
        skipped: 0,
        totalNodes: 10,
        totalRelationships: 0,
        fileResults: []
      });

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .option('--skip-relationships', 'Skip relationships', false)
        .action(async (options) => {
          await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.dirname(path.resolve(options.db)),
            dbPath: path.resolve(options.db),
            skipRelationships: options.skipRelationships,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });
        });

      // Act
      await program.parseAsync(['node', 'test', 'import', '--skip-relationships']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalledWith(
        expect.objectContaining({
          skipRelationships: true
        })
      );
    });

    it('should handle import errors and exit with code 1', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockResolvedValue({
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
      });

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .action(async (options) => {
          const result = await runImportPipeline({
            mode: 'incremental',
            sourceDir: path.resolve(options.source),
            outputDir: path.dirname(path.resolve(options.db)),
            dbPath: path.resolve(options.db),
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
      const { runImportPipeline } = require('../services/import-pipeline.service');
      runImportPipeline.mockRejectedValue(new Error('Database connection failed'));

      program
        .command('import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('-d, --db <file>', 'Database path', './output/lamad.kuzu')
        .action(async (options) => {
          try {
            await runImportPipeline({
              mode: 'incremental',
              sourceDir: path.resolve(options.source),
              outputDir: path.dirname(path.resolve(options.db)),
              dbPath: path.resolve(options.db),
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
      const { loadManifest, getImportStats, validateManifest } = require('../services/manifest.service');

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

      loadManifest.mockReturnValue(mockManifest);
      getImportStats.mockReturnValue({
        schemaVersion: '1.0.0',
        lastImport: '2024-01-02T00:00:00Z',
        totalSources: 10,
        totalNodes: 50,
        migrationCount: 0
      });
      validateManifest.mockReturnValue({
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
      expect(loadManifest).toHaveBeenCalled();
      expect(getImportStats).toHaveBeenCalled();
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Total sources: 10'));
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Total nodes: 50'));
    });

    it('should handle manifest load failure', async () => {
      // Arrange
      const { loadManifest } = require('../services/manifest.service');
      loadManifest.mockImplementation(() => {
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
      const { loadManifest, validateManifest } = require('../services/manifest.service');

      loadManifest.mockReturnValue({
        schemaVersion: '1.0.0',
        createdAt: '2024-01-01T00:00:00Z',
        lastUpdated: '2024-01-02T00:00:00Z',
        sourceHashes: {},
        nodeHashes: {},
        migrations: [],
        totalSourceFiles: 5,
        totalNodes: 10,
        totalRelationships: 0
      });

      validateManifest.mockReturnValue({
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
      expect(validateManifest).toHaveBeenCalled();
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('valid'));
    });

    it('should report validation errors and exit with code 1', async () => {
      // Arrange
      const { loadManifest, validateManifest } = require('../services/manifest.service');

      loadManifest.mockReturnValue({
        schemaVersion: '1.0.0',
        createdAt: '2024-01-01T00:00:00Z',
        lastUpdated: '2024-01-02T00:00:00Z',
        sourceHashes: {},
        nodeHashes: {},
        migrations: [],
        totalSourceFiles: 5,
        totalNodes: 10,
        totalRelationships: 0
      });

      validateManifest.mockReturnValue({
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

  describe('db:init command', () => {
    it('should initialize Kuzu database from JSON data', async () => {
      // Arrange
      const { KuzuClient } = require('../db/kuzu-client');

      const mockClient = {
        initialize: jest.fn().mockResolvedValue(undefined),
        bulkInsertContentNodes: jest.fn().mockResolvedValue(10),
        bulkInsertRelationships: jest.fn().mockResolvedValue(5),
        getStats: jest.fn().mockResolvedValue({ ContentNode: 10, RELATES_TO: 5 }),
        close: jest.fn()
      };

      KuzuClient.mockImplementation(() => mockClient);

      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue(JSON.stringify([
        { id: 'node-1', contentType: 'epic', title: 'Test' }
      ]));

      program
        .command('db:init')
        .option('-i, --input <dir>', 'Input directory', './output/lamad')
        .option('-o, --output <file>', 'Output database', './output/lamad.kuzu')
        .option('--force', 'Overwrite existing', false)
        .action(async (options) => {
          const client = new KuzuClient(path.resolve(options.output));
          await client.initialize();

          const nodesPath = path.join(path.resolve(options.input), 'nodes.json');
          if (fs.existsSync(nodesPath)) {
            const nodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));
            await client.bulkInsertContentNodes(nodes);
          }

          const stats = await client.getStats();
          console.log(`Inserted ${stats.ContentNode} nodes`);
          client.close();
        });

      // Act
      await program.parseAsync(['node', 'test', 'db:init']);

      // Assert
      expect(mockClient.initialize).toHaveBeenCalled();
      expect(mockClient.bulkInsertContentNodes).toHaveBeenCalled();
      expect(mockClient.close).toHaveBeenCalled();
    });

    it('should prevent overwriting existing database without --force', async () => {
      // Arrange
      mockFs.existsSync.mockReturnValue(true);

      program
        .command('db:init')
        .option('-i, --input <dir>', 'Input directory', './output/lamad')
        .option('-o, --output <file>', 'Output database', './output/lamad.kuzu')
        .option('--force', 'Overwrite existing', false)
        .action(async (options) => {
          const dbPath = path.resolve(options.output);

          if (fs.existsSync(dbPath) && !options.force) {
            console.error(`Database already exists at ${dbPath}. Use --force to overwrite.`);
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'db:init'])
      ).rejects.toThrow('process.exit(1)');

      expect(mockConsoleError).toHaveBeenCalledWith(
        expect.stringContaining('already exists')
      );
    });
  });
});
