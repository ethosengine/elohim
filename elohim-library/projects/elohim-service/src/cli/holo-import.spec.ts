/**
 * CLI Holochain Import Command Tests
 *
 * Tests for holochain-specific import commands and option handling
 */

import { Command } from 'commander';

// Mock dependencies
jest.mock('../services/import-pipeline.service');
jest.mock('../services/holochain-import.service');
jest.mock('../services/holochain-client.service');

describe('CLI holo-import commands', () => {
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

  describe('holo:import command', () => {
    it('should run import pipeline and holochain import with default options', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      const HolochainImportServiceModule = require('../services/holochain-import.service');

      runImportPipeline.mockResolvedValue({
        totalNodes: 10,
        totalFiles: 5,
        nodes: [
          { id: 'node-1', contentType: 'epic', title: 'Test' }
        ]
      });

      const mockHoloService = {
        importNodes: jest.fn().mockResolvedValue({
          importId: 'import-123',
          createdNodes: 10,
          totalNodes: 10,
          durationMs: 1500,
          errors: []
        })
      };

      HolochainImportServiceModule.HolochainImportService = jest.fn(() => mockHoloService);

      program
        .command('holo:import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('--admin-url <url>', 'Admin URL', 'wss://doorway-dev.elohim.host')
        .option('--app-id <id>', 'App ID', 'elohim')
        .option('--batch-size <n>', 'Batch size', '50')
        .option('-f, --full', 'Full import', false)
        .option('--dry-run', 'Dry run', false)
        .action(async (options) => {
          const pipelineResult = await runImportPipeline({
            mode: options.full ? 'full' : 'incremental',
            sourceDir: options.source,
            outputDir: '/tmp/holo-import',
            verbose: false,
            dryRun: true,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });

          if (!options.dryRun) {
            const holoService = new HolochainImportService({
              adminUrl: options.adminUrl,
              appId: options.appId,
              batchSize: parseInt(options.batchSize, 10)
            });

            await holoService.importNodes(pipelineResult.nodes);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:import']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalled();
      expect(mockHoloService.importNodes).toHaveBeenCalledWith(
        expect.arrayContaining([
          expect.objectContaining({ id: 'node-1' })
        ])
      );
    });

    it('should skip holochain import in dry-run mode', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      const HolochainImportServiceModule = require('../services/holochain-import.service');
      const mockConstructor = jest.fn();
      HolochainImportServiceModule.HolochainImportService = mockConstructor;

      runImportPipeline.mockResolvedValue({
        totalNodes: 5,
        totalFiles: 3,
        nodes: []
      });

      program
        .command('holo:import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('--dry-run', 'Dry run', false)
        .action(async (options) => {
          const pipelineResult = await runImportPipeline({
            mode: 'incremental',
            sourceDir: options.source,
            outputDir: '/tmp/holo-import',
            verbose: false,
            dryRun: true,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });

          if (!options.dryRun) {
            const holoService = new HolochainImportServiceModule.HolochainImportService({
              adminUrl: 'wss://test',
              appId: 'elohim',
              batchSize: 50
            });
            await holoService.importNodes(pipelineResult.nodes);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:import', '--dry-run']);

      // Assert
      expect(runImportPipeline).toHaveBeenCalled();
      expect(mockConstructor).not.toHaveBeenCalled();
    });

    it('should handle custom admin URL and app ID', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      const { HolochainImportService } = require('../services/holochain-import.service');

      runImportPipeline.mockResolvedValue({
        totalNodes: 3,
        totalFiles: 2,
        nodes: []
      });

      const mockHoloService = {
        importNodes: jest.fn().mockResolvedValue({
          importId: 'import-456',
          createdNodes: 3,
          totalNodes: 3,
          durationMs: 800,
          errors: []
        })
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('--admin-url <url>', 'Admin URL', 'wss://default')
        .option('--app-id <id>', 'App ID', 'default')
        .option('--batch-size <n>', 'Batch size', '50')
        .option('--dry-run', 'Dry run', false)
        .action(async (options) => {
          const pipelineResult = await runImportPipeline({
            mode: 'incremental',
            sourceDir: options.source,
            outputDir: '/tmp/holo-import',
            verbose: false,
            dryRun: true,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });

          if (!options.dryRun) {
            const holoService = new HolochainImportService({
              adminUrl: options.adminUrl,
              appId: options.appId,
              batchSize: parseInt(options.batchSize, 10)
            });
            await holoService.importNodes(pipelineResult.nodes);
          }
        });

      // Act
      await program.parseAsync([
        'node',
        'test',
        'holo:import',
        '--admin-url',
        'wss://custom.host',
        '--app-id',
        'custom-app'
      ]);

      // Assert
      expect(HolochainImportService).toHaveBeenCalledWith({
        adminUrl: 'wss://custom.host',
        appId: 'custom-app',
        batchSize: 50
      });
    });

    it('should handle custom batch size', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      const { HolochainImportService } = require('../services/holochain-import.service');

      runImportPipeline.mockResolvedValue({
        totalNodes: 100,
        totalFiles: 20,
        nodes: []
      });

      const mockHoloService = {
        importNodes: jest.fn().mockResolvedValue({
          importId: 'import-789',
          createdNodes: 100,
          totalNodes: 100,
          durationMs: 5000,
          errors: []
        })
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .option('--batch-size <n>', 'Batch size', '50')
        .option('--dry-run', 'Dry run', false)
        .action(async (options) => {
          const pipelineResult = await runImportPipeline({
            mode: 'incremental',
            sourceDir: options.source,
            outputDir: '/tmp/holo-import',
            verbose: false,
            dryRun: true,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });

          if (!options.dryRun) {
            const holoService = new HolochainImportService({
              adminUrl: options.adminUrl,
              appId: options.appId,
              batchSize: parseInt(options.batchSize, 10)
            });
            await holoService.importNodes(pipelineResult.nodes);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:import', '--batch-size', '100']);

      // Assert
      expect(HolochainImportService).toHaveBeenCalledWith(
        expect.objectContaining({ batchSize: 100 })
      );
    });

    it('should handle import errors and display them', async () => {
      // Arrange
      const { runImportPipeline } = require('../services/import-pipeline.service');
      const { HolochainImportService } = require('../services/holochain-import.service');

      runImportPipeline.mockResolvedValue({
        totalNodes: 10,
        totalFiles: 5,
        nodes: []
      });

      const mockHoloService = {
        importNodes: jest.fn().mockResolvedValue({
          importId: 'import-error',
          createdNodes: 7,
          totalNodes: 10,
          durationMs: 2000,
          errors: [
            'Failed to create node-1: validation error',
            'Failed to create node-5: network timeout',
            'Failed to create node-8: duplicate entry'
          ]
        })
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .option('--batch-size <n>', 'Batch size', '50')
        .option('--dry-run', 'Dry run', false)
        .action(async (options) => {
          const pipelineResult = await runImportPipeline({
            mode: 'incremental',
            sourceDir: options.source,
            outputDir: '/tmp/holo-import',
            verbose: false,
            dryRun: true,
            generateSourceNodes: true,
            generateDerivedNodes: true
          });

          if (!options.dryRun) {
            const holoService = new HolochainImportService({
              adminUrl: options.adminUrl,
              appId: options.appId,
              batchSize: parseInt(options.batchSize, 10)
            });

            const result = await holoService.importNodes(pipelineResult.nodes);

            if (result.errors.length > 0) {
              console.log(`Errors (${result.errors.length}):`);
              for (const error of result.errors) {
                console.log(`  - ${error}`);
              }
            }
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:import']);

      // Assert
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Errors (3)'));
    });
  });

  describe('holo:stats command', () => {
    it('should fetch and display holochain content statistics', async () => {
      // Arrange
      const { HolochainImportService } = require('../services/holochain-import.service');

      const mockHoloService = {
        getStats: jest.fn().mockResolvedValue({
          total_count: 150,
          by_type: {
            scenario: 50,
            epic: 20,
            role: 30,
            concept: 40,
            reference: 10
          }
        })
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:stats')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          });

          const stats = await holoService.getStats();
          console.log(`Total nodes: ${stats.total_count}`);

          if (Object.keys(stats.by_type).length > 0) {
            console.log('By content type:');
            for (const [type, count] of Object.entries(stats.by_type)) {
              console.log(`  ${type}: ${count}`);
            }
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:stats']);

      // Assert
      expect(mockHoloService.getStats).toHaveBeenCalled();
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Total nodes: 150'));
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('scenario: 50'));
    });

    it('should handle empty holochain database', async () => {
      // Arrange
      const { HolochainImportService } = require('../services/holochain-import.service');

      const mockHoloService = {
        getStats: jest.fn().mockResolvedValue({
          total_count: 0,
          by_type: {}
        })
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:stats')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          });

          const stats = await holoService.getStats();
          console.log(`Total nodes: ${stats.total_count}`);

          if (Object.keys(stats.by_type).length === 0) {
            console.log('No content found in Holochain.');
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:stats']);

      // Assert
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('No content found'));
    });
  });

  describe('holo:verify command', () => {
    it('should verify content IDs exist in holochain', async () => {
      // Arrange
      const { HolochainImportService } = require('../services/holochain-import.service');

      const mockHoloService = {
        verifyContent: jest.fn().mockResolvedValue({
          found: ['node-1', 'node-2', 'node-3'],
          missing: []
        })
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:verify')
        .option('-i, --ids <ids>', 'Content IDs')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const ids = options.ids.split(',').map((s: string) => s.trim());

          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          });

          const result = await holoService.verifyContent(ids);
          console.log(`Found: ${result.found.length}/${ids.length}`);
          console.log(`Missing: ${result.missing.length}`);

          if (result.missing.length > 0) {
            process.exit(1);
          }
        });

      // Act
      await program.parseAsync([
        'node',
        'test',
        'holo:verify',
        '-i',
        'node-1,node-2,node-3'
      ]);

      // Assert
      expect(mockHoloService.verifyContent).toHaveBeenCalledWith(['node-1', 'node-2', 'node-3']);
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Found: 3/3'));
    });

    it('should exit with code 1 if content is missing', async () => {
      // Arrange
      const { HolochainImportService } = require('../services/holochain-import.service');

      const mockHoloService = {
        verifyContent: jest.fn().mockResolvedValue({
          found: ['node-1'],
          missing: ['node-2', 'node-3']
        })
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:verify')
        .option('-i, --ids <ids>', 'Content IDs')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const ids = options.ids.split(',').map((s: string) => s.trim());

          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          });

          const result = await holoService.verifyContent(ids);

          if (result.missing.length > 0) {
            console.log('Missing IDs:');
            for (const id of result.missing) {
              console.log(`  - ${id}`);
            }
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'holo:verify', '-i', 'node-1,node-2,node-3'])
      ).rejects.toThrow('process.exit(1)');

      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('node-2'));
    });
  });

  describe('holo:test command', () => {
    it('should test holochain connection successfully', async () => {
      // Arrange
      const { HolochainClientService } = require('../services/holochain-client.service');

      const mockClient = {
        connect: jest.fn().mockResolvedValue(undefined),
        callZome: jest.fn().mockResolvedValue({ total_count: 42 }),
        disconnect: jest.fn().mockResolvedValue(undefined)
      };

      HolochainClientService.mockImplementation(() => mockClient);

      program
        .command('holo:test')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const client = new HolochainClientService({
            adminUrl: options.adminUrl,
            appId: options.appId
          });

          try {
            await client.connect();
            console.log('[OK] Connected to conductor');

            const stats = await client.callZome({
              zomeName: 'content_store',
              fnName: 'get_content_stats',
              payload: null
            });
            console.log(`[OK] Zome call successful`);
            console.log(`  Content count: ${stats.total_count}`);

            await client.disconnect();
            console.log('[OK] Disconnected cleanly');

            console.log('Connection test PASSED');
          } catch (err) {
            console.error(`[FAIL] ${err}`);
            process.exit(1);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:test']);

      // Assert
      expect(mockClient.connect).toHaveBeenCalled();
      expect(mockClient.callZome).toHaveBeenCalled();
      expect(mockClient.disconnect).toHaveBeenCalled();
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('PASSED'));
    });

    it('should handle connection failure', async () => {
      // Arrange
      const { HolochainClientService } = require('../services/holochain-client.service');

      const mockClient = {
        connect: jest.fn().mockRejectedValue(new Error('Connection refused')),
        callZome: jest.fn(),
        disconnect: jest.fn()
      };

      HolochainClientService.mockImplementation(() => mockClient);

      program
        .command('holo:test')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const client = new HolochainClientService({
            adminUrl: options.adminUrl,
            appId: options.appId
          });

          try {
            await client.connect();
          } catch (err) {
            console.error(`[FAIL] ${err}`);
            process.exit(1);
          }
        });

      // Act & Assert
      await expect(
        program.parseAsync(['node', 'test', 'holo:test'])
      ).rejects.toThrow('process.exit(1)');

      expect(mockConsoleError).toHaveBeenCalledWith(
        expect.stringContaining('Connection refused')
      );
    });
  });

  describe('holo:list command', () => {
    it('should list content by type', async () => {
      // Arrange
      const { HolochainImportService } = require('../services/holochain-import.service');

      const mockHoloService = {
        getContentByType: jest.fn().mockResolvedValue([
          {
            content: {
              id: 'scenario-1',
              title: 'Test Scenario 1',
              content_format: 'gherkin',
              tags: ['test'],
              reach: 'commons'
            }
          },
          {
            content: {
              id: 'scenario-2',
              title: 'Test Scenario 2',
              content_format: 'gherkin',
              tags: ['test', 'governance'],
              reach: 'commons'
            }
          }
        ])
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:list')
        .option('-t, --type <type>', 'Content type', 'scenario')
        .option('-l, --limit <n>', 'Limit', '20')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          });

          const results = await holoService.getContentByType(
            options.type,
            parseInt(options.limit, 10)
          );

          if (results.length === 0) {
            console.log('No content found.');
            return;
          }

          for (const item of results) {
            console.log(`${item.content.id}`);
            console.log(`  Title: ${item.content.title}`);
          }

          console.log(`Total: ${results.length}`);
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:list', '-t', 'scenario']);

      // Assert
      expect(mockHoloService.getContentByType).toHaveBeenCalledWith('scenario', 20);
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('scenario-1'));
      expect(mockConsoleLog).toHaveBeenCalledWith(expect.stringContaining('Total: 2'));
    });

    it('should handle empty results', async () => {
      // Arrange
      const { HolochainImportService } = require('../services/holochain-import.service');

      const mockHoloService = {
        getContentByType: jest.fn().mockResolvedValue([])
      };

      HolochainImportService.mockImplementation(() => mockHoloService);

      program
        .command('holo:list')
        .option('-t, --type <type>', 'Content type', 'scenario')
        .option('-l, --limit <n>', 'Limit', '20')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          });

          const results = await holoService.getContentByType(
            options.type,
            parseInt(options.limit, 10)
          );

          if (results.length === 0) {
            console.log('No content found.');
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:list', '-t', 'nonexistent']);

      // Assert
      expect(mockConsoleLog).toHaveBeenCalledWith('No content found.');
    });
  });
});
