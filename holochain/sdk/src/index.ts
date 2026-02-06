/**
 * Elohim Holochain SDK
 *
 * TypeScript SDK for Elohim Holochain services.
 * Provides high-level APIs for content, relationships, learning paths, and human network.
 *
 * @example
 * ```typescript
 * import { ElohimSDK } from '@elohim/holochain-sdk';
 *
 * const sdk = new ElohimSDK({
 *   adminUrl: 'ws://localhost:4444',
 *   appUrl: 'ws://localhost:4445',
 * });
 *
 * await sdk.connect();
 *
 * // Create content
 * const content = await sdk.content.create({
 *   id: 'my-content',
 *   content_type: 'concept',
 *   title: 'My Concept',
 *   description: 'A description',
 *   content: '# My Content',
 *   content_format: 'markdown',
 *   tags: ['test'],
 *   related_node_ids: [],
 *   reach: 'public',
 *   metadata_json: '{}',
 * });
 *
 * // Create relationship
 * await sdk.relationships.relatesTo('content-a', 'content-b');
 *
 * // Create learning path
 * const pathHash = await sdk.paths.createSimple('intro-path', 'Introduction', 'Get started');
 * await sdk.paths.addSteps('intro-path', [
 *   { resourceId: 'content-a' },
 *   { resourceId: 'content-b' },
 * ]);
 *
 * await sdk.disconnect();
 * ```
 */

import { HolochainConnection, type ConnectionState } from './connection.js';
import { ZomeClient } from './client/zome-client.js';
import { BatchExecutor, type BatchExecutorConfig } from './client/batch-executor.js';
import { ContentService } from './services/content.service.js';
import { RelationshipService } from './services/relationship.service.js';
import { PathService } from './services/path.service.js';
import { HumanService } from './services/human.service.js';
import { type ConnectionConfig } from './types.js';

/**
 * Main SDK facade - Spring-style service container
 *
 * Provides access to all services through a single entry point.
 */
export class ElohimSDK {
  private connection: HolochainConnection;
  private zomeClient: ZomeClient | null = null;

  // Service instances (lazy-initialized on connect)
  private _content: ContentService | null = null;
  private _relationships: RelationshipService | null = null;
  private _paths: PathService | null = null;
  private _humans: HumanService | null = null;

  constructor(config: ConnectionConfig) {
    this.connection = new HolochainConnection(config);
  }

  /**
   * Connect to the Holochain conductor
   */
  async connect(): Promise<void> {
    await this.connection.connect();
    this.zomeClient = new ZomeClient(this.connection);

    // Initialize services
    this._content = new ContentService(this.zomeClient);
    this._relationships = new RelationshipService(this.zomeClient);
    this._paths = new PathService(this.zomeClient);
    this._humans = new HumanService(this.zomeClient);
  }

  /**
   * Disconnect from the conductor
   */
  async disconnect(): Promise<void> {
    await this.connection.disconnect();
    this.zomeClient = null;
    this._content = null;
    this._relationships = null;
    this._paths = null;
    this._humans = null;
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.connection.isConnected();
  }

  /**
   * Get connection state
   */
  getConnectionState(): ConnectionState {
    return this.connection.getState();
  }

  /**
   * Content service - CRUD operations on content entries
   */
  get content(): ContentService {
    if (!this._content) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this._content;
  }

  /**
   * Relationship service - manage relationships between content
   */
  get relationships(): RelationshipService {
    if (!this._relationships) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this._relationships;
  }

  /**
   * Path service - learning path management
   */
  get paths(): PathService {
    if (!this._paths) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this._paths;
  }

  /**
   * Human service - agent network management
   */
  get humans(): HumanService {
    if (!this._humans) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this._humans;
  }

  /**
   * Get the low-level zome client (for advanced use)
   */
  getZomeClient(): ZomeClient {
    if (!this.zomeClient) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this.zomeClient;
  }

  /**
   * Get the underlying connection (for advanced use)
   */
  getConnection(): HolochainConnection {
    return this.connection;
  }

  /**
   * Create a batch executor with custom configuration
   */
  createBatchExecutor(config?: BatchExecutorConfig): BatchExecutor {
    if (!this.zomeClient) {
      throw new Error('Not connected. Call connect() first.');
    }
    return new BatchExecutor(this.zomeClient, config);
  }
}

/**
 * Create an SDK instance with the given configuration
 */
export function createSDK(config: ConnectionConfig): ElohimSDK {
  return new ElohimSDK(config);
}

// =============================================================================
// Re-exports
// =============================================================================

// Connection
export { HolochainConnection, createConnection } from './connection.js';
export type { ConnectionState } from './connection.js';

// Client
export { ZomeClient } from './client/zome-client.js';
export { BatchExecutor, createBatchExecutor } from './client/batch-executor.js';
export type { BatchExecutorConfig, BatchResult } from './client/batch-executor.js';

// Services
export { ContentService } from './services/content.service.js';
export { RelationshipService } from './services/relationship.service.js';
export { PathService } from './services/path.service.js';
export { HumanService } from './services/human.service.js';

// Types
export * from './types.js';
