/**
 * Connection Strategy Factory
 *
 * Creates the appropriate connection strategy based on environment detection.
 *
 * Auto-detection logic:
 * 1. Check for Tauri (__TAURI__ global) → Direct mode
 * 2. Check for Node.js (process.versions.node) → Direct mode
 * 3. Otherwise → Doorway mode (browser)
 *
 * Usage:
 * ```typescript
 * import { createConnectionStrategy, detectConnectionMode } from './connection-strategy-factory';
 *
 * // Auto-detect mode
 * const strategy = createConnectionStrategy('auto');
 *
 * // Force specific mode
 * const doorwayStrategy = createConnectionStrategy('doorway');
 * const directStrategy = createConnectionStrategy('direct');
 * ```
 *
 * @packageDocumentation
 */

import type { ConnectionMode, IConnectionStrategy } from './connection-strategy';
import { DoorwayConnectionStrategy } from './doorway-connection-strategy';
import { DirectConnectionStrategy } from './direct-connection-strategy';
import { TauriConnectionStrategy } from './tauri-connection-strategy';

/** Extended connection mode type including 'tauri' */
export type ExtendedConnectionMode = ConnectionMode | 'tauri';

/**
 * Detect the appropriate connection mode based on environment.
 *
 * Detection order:
 * 1. Tauri (native app) → 'tauri' (uses TauriConnectionStrategy with session management)
 * 2. Node.js (CLI tools) → 'direct'
 * 3. Browser (web app) → 'doorway'
 */
export function detectConnectionMode(): Exclude<ExtendedConnectionMode, 'auto'> {
  // Check for Tauri environment (native app)
  if (typeof window !== 'undefined' && '__TAURI__' in window) {
    console.log('[ConnectionStrategy] Detected Tauri environment → tauri mode');
    return 'tauri';
  }

  // Check for Node.js environment (CLI tools, tests)
  if (typeof process !== 'undefined' && process.versions?.node !== undefined) {
    console.log('[ConnectionStrategy] Detected Node.js environment → direct mode');
    return 'direct';
  }

  // Default to doorway for browser
  console.log('[ConnectionStrategy] Detected browser environment → doorway mode');
  return 'doorway';
}

/**
 * Create a connection strategy instance.
 *
 * @param mode - Connection mode ('auto', 'doorway', 'direct', or 'tauri')
 * @returns Connection strategy instance
 *
 * @example
 * ```typescript
 * // Auto-detect based on environment
 * const strategy = createConnectionStrategy('auto');
 *
 * // Force doorway mode for browser testing
 * const doorwayStrategy = createConnectionStrategy('doorway');
 *
 * // Force tauri mode with session management
 * const tauriStrategy = createConnectionStrategy('tauri');
 * ```
 */
export function createConnectionStrategy(mode: ExtendedConnectionMode = 'auto'): IConnectionStrategy {
  const resolvedMode = mode === 'auto' ? detectConnectionMode() : mode;

  switch (resolvedMode) {
    case 'tauri':
      console.log('[ConnectionStrategy] Creating TauriConnectionStrategy');
      return new TauriConnectionStrategy();

    case 'direct':
      console.log('[ConnectionStrategy] Creating DirectConnectionStrategy');
      return new DirectConnectionStrategy();

    case 'doorway':
    default:
      console.log('[ConnectionStrategy] Creating DoorwayConnectionStrategy');
      return new DoorwayConnectionStrategy();
  }
}

/**
 * Check if a specific connection mode is supported in the current environment.
 *
 * @param mode - Connection mode to check
 * @returns true if the mode is supported
 */
export function isConnectionModeSupported(mode: Exclude<ExtendedConnectionMode, 'auto'>): boolean {
  switch (mode) {
    case 'tauri':
      return new TauriConnectionStrategy().isSupported();
    case 'direct':
      return new DirectConnectionStrategy().isSupported();
    case 'doorway':
      return new DoorwayConnectionStrategy().isSupported();
    default:
      return false;
  }
}

/**
 * Get all supported connection modes in the current environment.
 *
 * @returns Array of supported connection modes
 */
export function getSupportedConnectionModes(): Array<Exclude<ExtendedConnectionMode, 'auto'>> {
  const modes: Array<Exclude<ExtendedConnectionMode, 'auto'>> = [];

  if (isConnectionModeSupported('tauri')) {
    modes.push('tauri');
  }
  if (isConnectionModeSupported('direct')) {
    modes.push('direct');
  }
  if (isConnectionModeSupported('doorway')) {
    modes.push('doorway');
  }

  return modes;
}
