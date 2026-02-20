/**
 * Service Container - Dependency Injection Container
 *
 * Provides centralized service instantiation and dependency management.
 * Allows for easy mocking in tests and flexible configuration.
 */

import * as fs from 'fs';

import { KuzuClient } from '../db/kuzu-client';
import { HolochainImportConfig } from '../models/holochain.model';
import { HolochainClientService } from '../services/holochain-client.service';
import { HolochainImportService } from '../services/holochain-import.service';

/**
 * File system operations interface (for testability)
 */
export interface IFileSystem {
  existsSync(path: string): boolean;
  readFileSync(path: string, encoding: BufferEncoding): string;
  writeFileSync(path: string, data: string, encoding: BufferEncoding): void;
  mkdirSync(path: string, options?: { recursive?: boolean }): void;
  readdirSync(path: string): string[];
  rmSync(path: string, options?: { recursive?: boolean }): void;
  statSync(path: string): { size: number };
}

/**
 * Console operations interface (for testability)
 */
export interface IConsole {
  log(...args: any[]): void;
  error(...args: any[]): void;
}

/**
 * Process operations interface (for testability)
 */
export interface IProcess {
  exit(code?: number): never;
  cwd(): string;
}

/**
 * Import pipeline service interface
 */
export interface IImportPipelineService {
  runImportPipeline(options: {
    mode: 'full' | 'incremental';
    sourceDir: string;
    outputDir: string;
    dbPath?: string;
    verbose?: boolean;
    dryRun?: boolean;
    generateSourceNodes?: boolean;
    generateDerivedNodes?: boolean;
    skipRelationships?: boolean;
  }): Promise<{
    errors: number;
    created: number;
    skipped: number;
    totalNodes: number;
    totalRelationships: number;
    totalFiles?: number;
    nodes: any[];
    fileResults: {
      sourcePath: string;
      status: 'created' | 'skipped' | 'error';
      error?: string;
      nodeIds: string[];
      processingTime: number;
    }[];
  }>;
}

/**
 * Manifest service interface
 */
export interface IManifestService {
  loadManifest(outputDir: string): any;
  getImportStats(manifest: any): {
    schemaVersion: string;
    lastImport: string;
    totalSources: number;
    totalNodes: number;
    migrationCount: number;
  };
  validateManifest(manifest: any): {
    valid: boolean;
    errors: string[];
  };
}

/**
 * Standards service interface
 */
export interface IStandardsService {
  generateCoverageReport(nodes: any[]): {
    total: number;
    coverage: Record<string, { count: number; total: number; percentage: number }>;
    errors: string[];
    allTargetsMet: boolean;
  };
  validateStandardsFields(node: any): string[];
}

/**
 * Trust service interface
 */
export interface ITrustService {
  loadAttestations(attestationsPath: string): Record<string, any[]>;
  enrichContentDirectory(
    contentDir: string,
    attestationsPath: string
  ): Promise<{
    processed: number;
    enriched: number;
    withAttestations: number;
    errors: string[];
  }>;
  updateContentIndexWithTrust(
    indexPath: string,
    attestationsByContent: Record<string, any[]>
  ): void;
}

/**
 * Scaffold service interface
 */
export interface IScaffoldService {
  scaffoldUserType(
    basePath: string,
    epic: string,
    userType: string
  ): {
    created: string[];
    skipped: string[];
    errors: string[];
  };
  scaffoldEpic(
    basePath: string,
    epic: string
  ): {
    created: string[];
    skipped: string[];
    errors: string[];
  };
  scaffoldAll(basePath: string): {
    created: string[];
    skipped: string[];
    errors: string[];
  };
  listEpicsAndUsers(): { epic: string; description: string; users: string[] }[];
}

/**
 * Human service interface
 */
export interface IHumanService {
  loadHumansData(filePath: string): any;
  createHuman(data: any): any;
  createRelationship(data: any): any;
  addHumanToFile(filePath: string, human: any): void;
  addRelationshipToFile(filePath: string, relationship: any): void;
  importHumansToLamad(
    sourcePath: string,
    outputDir: string
  ): Promise<{
    humansImported: number;
    relationshipsImported: number;
    errors: string[];
  }>;
  listHumanCategories(): string[];
  listRelationshipTypes(): { type: string; layer: string; intimacy: string }[];
}

/**
 * Service container configuration
 */
export interface ServiceContainerConfig {
  fileSystem?: IFileSystem;
  console?: IConsole;
  process?: IProcess;
  importPipeline?: IImportPipelineService;
  manifest?: IManifestService;
  standards?: IStandardsService;
  trust?: ITrustService;
  scaffold?: IScaffoldService;
  human?: IHumanService;
}

/**
 * Service container - manages all service dependencies
 */
export class ServiceContainer {
  public readonly fs: IFileSystem;
  public readonly console: IConsole;
  public readonly process: IProcess;
  public readonly importPipeline: IImportPipelineService;
  public readonly manifest: IManifestService;
  public readonly standards: IStandardsService;
  public readonly trust: ITrustService;
  public readonly scaffold: IScaffoldService;
  public readonly human: IHumanService;

  constructor(config: ServiceContainerConfig = {}) {
    // Use provided implementations or fall back to real implementations
    this.fs = config.fileSystem || this.createRealFileSystem();
    this.console = config.console || this.createRealConsole();
    this.process = config.process || this.createRealProcess();

    // Lazy-load service modules only if not provided (avoids import issues in tests)
    this.importPipeline = config.importPipeline || this.createRealImportPipeline();
    this.manifest = config.manifest || this.createRealManifest();
    this.standards = config.standards || this.createRealStandards();
    this.trust = config.trust || this.createRealTrust();
    this.scaffold = config.scaffold || this.createRealScaffold();
    this.human = config.human || this.createRealHuman();
  }

  /**
   * Create Kuzu database client
   */
  createKuzuClient(dbPath: string): KuzuClient {
    return new KuzuClient(dbPath);
  }

  /**
   * Create Holochain client service
   */
  createHolochainClient(config: { adminUrl: string; appId: string }): HolochainClientService {
    return new HolochainClientService(config);
  }

  /**
   * Create Holochain import service
   */
  createHolochainImport(config: HolochainImportConfig): HolochainImportService {
    return new HolochainImportService(config);
  }

  // Private factory methods for real implementations

  private createRealFileSystem(): IFileSystem {
    return {
      existsSync: fs.existsSync,
      readFileSync: fs.readFileSync,
      writeFileSync: fs.writeFileSync,
      mkdirSync: fs.mkdirSync,
      readdirSync: fs.readdirSync,
      rmSync: fs.rmSync,
      statSync: fs.statSync,
    };
  }

  private createRealConsole(): IConsole {
    return {
      log: console.log.bind(console),
      error: console.error.bind(console),
    };
  }

  private createRealProcess(): IProcess {
    return {
      exit: process.exit.bind(process),
      cwd: process.cwd.bind(process),
    } as IProcess;
  }

  private createRealImportPipeline(): IImportPipelineService {
    // Dynamic import to avoid circular dependencies
    const { runImportPipeline } = require('../services/import-pipeline.service');
    return { runImportPipeline };
  }

  private createRealManifest(): IManifestService {
    const {
      loadManifest,
      getImportStats,
      validateManifest,
    } = require('../services/manifest.service');
    return { loadManifest, getImportStats, validateManifest };
  }

  private createRealStandards(): IStandardsService {
    const {
      generateCoverageReport,
      validateStandardsFields,
    } = require('../services/standards.service');
    return { generateCoverageReport, validateStandardsFields };
  }

  private createRealTrust(): ITrustService {
    const {
      loadAttestations,
      enrichContentDirectory,
      updateContentIndexWithTrust,
    } = require('../services/trust.service');
    return { loadAttestations, enrichContentDirectory, updateContentIndexWithTrust };
  }

  private createRealScaffold(): IScaffoldService {
    const {
      scaffoldUserType,
      scaffoldEpic,
      scaffoldAll,
      listEpicsAndUsers,
    } = require('../services/scaffold.service');
    return { scaffoldUserType, scaffoldEpic, scaffoldAll, listEpicsAndUsers };
  }

  private createRealHuman(): IHumanService {
    const {
      loadHumansData,
      createHuman,
      createRelationship,
      addHumanToFile,
      addRelationshipToFile,
      importHumansToLamad,
      listHumanCategories,
      listRelationshipTypes,
    } = require('../services/human.service');
    return {
      loadHumansData,
      createHuman,
      createRelationship,
      addHumanToFile,
      addRelationshipToFile,
      importHumansToLamad,
      listHumanCategories,
      listRelationshipTypes,
    };
  }
}
