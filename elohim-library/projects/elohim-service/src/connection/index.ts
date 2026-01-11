/**
 * Connection Module
 *
 * Provides connection strategies for different Holochain deployment modes:
 *
 * - **Doorway Mode**: Routes through Doorway proxy (browser/web deployments)
 * - **Direct Mode**: Connects directly to local conductor (native/Tauri deployments)
 *
 * @example
 * ```typescript
 * import {
 *   createConnectionStrategy,
 *   detectConnectionMode,
 *   type IConnectionStrategy,
 *   type ConnectionConfig,
 * } from '@elohim/service/connection';
 *
 * // Auto-detect mode based on environment
 * const strategy = createConnectionStrategy('auto');
 *
 * // Connect to Holochain
 * const result = await strategy.connect({
 *   mode: 'doorway',
 *   adminUrl: 'wss://doorway-dev.elohim.host',
 *   appUrl: 'wss://doorway-dev.elohim.host',
 *   appId: 'elohim',
 * });
 * ```
 *
 * @packageDocumentation
 */

// Types and interfaces
export type {
  ConnectionMode,
  ConnectionConfig,
  ConnectionResult,
  ContentSourceConfig,
  SigningCredentials,
  IConnectionStrategy,
} from './connection-strategy';

// Strategy implementations
export { DoorwayConnectionStrategy } from './doorway-connection-strategy';
export { DirectConnectionStrategy } from './direct-connection-strategy';
export { TauriConnectionStrategy } from './tauri-connection-strategy';

// Tauri-specific types
export type {
  LocalSession,
  CreateSessionInput,
  NativeHandoffResponse,
  OAuthCallbackPayload,
} from './tauri-connection-strategy';

// Factory functions
export {
  createConnectionStrategy,
  detectConnectionMode,
  isConnectionModeSupported,
  getSupportedConnectionModes,
} from './connection-strategy-factory';

// Re-export SourceTier from content-resolver for convenience
export { SourceTier } from '../cache/content-resolver';
