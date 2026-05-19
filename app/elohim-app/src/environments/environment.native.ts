/**
 * Native Environment Configuration
 *
 * Used for Tauri (native desktop) and other direct conductor deployments.
 * Connects directly to local Holochain conductor without Doorway proxy.
 *
 * Connection path:
 *   App → Local Conductor (ws://localhost:4444)
 *   App → elohim-storage sidecar (http://localhost:8090)
 */

import { Environment, LogLevel } from './environment.types';

export const environment: Environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'native',
  gitHash: 'GIT_HASH_PLACEHOLDER',

  // Direct connection to local Holochain conductor
  holochain: {
    adminUrl: 'ws://localhost:4444',
    appUrl: 'ws://localhost:4445',
    // Direct mode - no proxy authentication needed
    connectionMode: 'direct',
    // elohim-storage sidecar for blob storage
    storageUrl: 'http://localhost:8090',
  },

  // Projection API disabled in direct mode (no Doorway)
  projectionApi: {
    enabled: false,
  },
  features: {
    // Routes topology fetches through /api/v1/graphql on the local
    // elohim-storage sidecar. See environment.ts for rationale.
    // Flipped 2026-05-19 (L6 viewer.* symmetry pass).
    useGraphqlTopology: true,
  },
};
