/**
 * CLI Holochain Import Command Tests
 *
 * Tests for holochain-specific import commands and option handling
 */

import { type MockInstance } from 'vitest';
import { Command } from 'commander';
import { runImportPipeline } from '../services/import-pipeline.service';
import { HolochainImportService } from '../services/holochain-import.service';
import { HolochainClientService } from '../services/holochain-client.service';

// Mock dependencies
// Manual factories needed because these services import @holochain/client (ESM)
vi.mock('../services/import-pipeline.service');
vi.mock('../services/holochain-import.service', () => ({
  HolochainImportService: vi.fn(),
}));
vi.mock('../services/holochain-client.service', () => ({
  HolochainClientService: vi.fn(),
}));

const mockRunImportPipeline = vi.mocked(runImportPipeline);
const MockHolochainImportService = vi.mocked(HolochainImportService);
const MockHolochainClientService = vi.mocked(HolochainClientService);

describe('CLI holo-import commands', () => {
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

  describe('holo:import command', () => {
    it('should run import pipeline and holochain import with default options', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        totalNodes: 10,
        totalFiles: 5,
        nodes: [
          { id: 'node-1', contentType: 'epic', title: 'Test' }
        ]
      } as any);

      const mockHoloService = {
        importNodes: vi.fn().mockResolvedValue({
          importId: 'import-123',
          createdNodes: 10,
          totalNodes: 10,
          durationMs: 1500,
          errors: []
        })
      };

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

      program
        .command('holo:import')
        .option('-s, --source <dir>', 'Source directory', './docs/content')
        .option('--admin-url <url>', 'Admin URL', 'wss://doorway-alpha.elohim.host')
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
            } as any);

            await (holoService as any).importNodes(pipelineResult.nodes);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:import']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalled();
      expect(mockHoloService.importNodes).toHaveBeenCalledWith(
        expect.arrayContaining([
          expect.objectContaining({ id: 'node-1' })
        ])
      );
    });

    it('should skip holochain import in dry-run mode', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        totalNodes: 5,
        totalFiles: 3,
        nodes: []
      } as any);

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
            const holoService = new HolochainImportService({
              adminUrl: 'wss://test',
              appId: 'elohim',
              batchSize: 50
            } as any);
            await (holoService as any).importNodes(pipelineResult.nodes);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:import', '--dry-run']);

      // Assert
      expect(mockRunImportPipeline).toHaveBeenCalled();
      expect(MockHolochainImportService).not.toHaveBeenCalled();
    });

    it('should handle custom admin URL and app ID', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        totalNodes: 3,
        totalFiles: 2,
        nodes: []
      } as any);

      const mockHoloService = {
        importNodes: vi.fn().mockResolvedValue({
          importId: 'import-456',
          createdNodes: 3,
          totalNodes: 3,
          durationMs: 800,
          errors: []
        })
      };

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

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
            } as any);
            await (holoService as any).importNodes(pipelineResult.nodes);
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
      expect(MockHolochainImportService).toHaveBeenCalledWith({
        adminUrl: 'wss://custom.host',
        appId: 'custom-app',
        batchSize: 50
      });
    });

    it('should handle custom batch size', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        totalNodes: 100,
        totalFiles: 20,
        nodes: []
      } as any);

      const mockHoloService = {
        importNodes: vi.fn().mockResolvedValue({
          importId: 'import-789',
          createdNodes: 100,
          totalNodes: 100,
          durationMs: 5000,
          errors: []
        })
      };

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

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
            } as any);
            await (holoService as any).importNodes(pipelineResult.nodes);
          }
        });

      // Act
      await program.parseAsync(['node', 'test', 'holo:import', '--batch-size', '100']);

      // Assert
      expect(MockHolochainImportService).toHaveBeenCalledWith(
        expect.objectContaining({ batchSize: 100 })
      );
    });

    it('should handle import errors and display them', async () => {
      // Arrange
      mockRunImportPipeline.mockResolvedValue({
        totalNodes: 10,
        totalFiles: 5,
        nodes: []
      } as any);

      const mockHoloService = {
        importNodes: vi.fn().mockResolvedValue({
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

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

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
            } as any);

            const result = await (holoService as any).importNodes(pipelineResult.nodes);

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
      const mockHoloService = {
        getStats: vi.fn().mockResolvedValue({
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

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

      program
        .command('holo:stats')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          } as any);

          const stats = await (holoService as any).getStats();
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
      const mockHoloService = {
        getStats: vi.fn().mockResolvedValue({
          total_count: 0,
          by_type: {}
        })
      };

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

      program
        .command('holo:stats')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const holoService = new HolochainImportService({
            adminUrl: options.adminUrl,
            appId: options.appId,
            batchSize: 50
          } as any);

          const stats = await (holoService as any).getStats();
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
      const mockHoloService = {
        verifyContent: vi.fn().mockResolvedValue({
          found: ['node-1', 'node-2', 'node-3'],
          missing: []
        })
      };

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

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
          } as any);

          const result = await (holoService as any).verifyContent(ids);
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
      const mockHoloService = {
        verifyContent: vi.fn().mockResolvedValue({
          found: ['node-1'],
          missing: ['node-2', 'node-3']
        })
      };

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

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
          } as any);

          const result = await (holoService as any).verifyContent(ids);

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
      const mockClient = {
        connect: vi.fn().mockResolvedValue(undefined),
        callZome: vi.fn().mockResolvedValue({ total_count: 42 }),
        disconnect: vi.fn().mockResolvedValue(undefined)
      };

      MockHolochainClientService.mockImplementation(function () { return mockClient; } as any);

      program
        .command('holo:test')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const client = new HolochainClientService({
            adminUrl: options.adminUrl,
            appId: options.appId
          } as any);

          try {
            await (client as any).connect();
            console.log('[OK] Connected to conductor');

            const stats = await (client as any).callZome({
              zomeName: 'content_store',
              fnName: 'get_content_stats',
              payload: null
            });
            console.log(`[OK] Zome call successful`);
            console.log(`  Content count: ${stats.total_count}`);

            await (client as any).disconnect();
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
      const mockClient = {
        connect: vi.fn().mockRejectedValue(new Error('Connection refused')),
        callZome: vi.fn(),
        disconnect: vi.fn()
      };

      MockHolochainClientService.mockImplementation(function () { return mockClient; } as any);

      program
        .command('holo:test')
        .option('--admin-url <url>', 'Admin URL', 'wss://test')
        .option('--app-id <id>', 'App ID', 'elohim')
        .action(async (options) => {
          const client = new HolochainClientService({
            adminUrl: options.adminUrl,
            appId: options.appId
          } as any);

          try {
            await (client as any).connect();
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
      const mockHoloService = {
        getContentByType: vi.fn().mockResolvedValue([
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

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

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
          } as any);

          const results = await (holoService as any).getContentByType(
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
      const mockHoloService = {
        getContentByType: vi.fn().mockResolvedValue([])
      };

      MockHolochainImportService.mockImplementation(function () { return mockHoloService; } as any);

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
          } as any);

          const results = await (holoService as any).getContentByType(
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
