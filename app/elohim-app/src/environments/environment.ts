import { Environment, LogLevel } from './environment.types';

// @coverage: 100.0% (2026-03-03)

export const environment: Environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'development',
  gitHash: 'local-dev',
  // Holochain Edge Node configuration
  // In a development workspace: HTTP surfaces route same-origin through the
  // dev-server proxy; the doorway's own origin comes from app/workspace-runtime/
  // In deployed mode: uses remote admin-proxy with API key authentication
  holochain: {
    adminUrl: 'wss://doorway-alpha.elohim.host',
    appUrl: 'wss://doorway-alpha.elohim.host', // Fallback for deployed mode
    // Auth URL - uses local admin-proxy in dev (port 8888)
    // In production, this should point to the admin-proxy HTTP endpoint
    authUrl: 'http://localhost:8888',
    proxyApiKey: 'dev-elohim-auth-2024', // Authenticated access (not admin)
    useLocalProxy: true, // HTTP surfaces are reverse-proxied at same origin
    // Connection mode: 'auto' detects Tauri→direct, browser→doorway
    connectionMode: 'auto',
    // elohim-storage sidecar URL (for direct mode blob storage)
    storageUrl: 'http://localhost:8090',
  },
  features: {
    // Routes topology fetches through /api/v1/graphql. Vitest parity at
    // shefa/services/topology-parity.spec.ts asserts structural equivalence
    // with the REST path. Flipped 2026-05-19 (L6 viewer.* symmetry pass).
    useGraphqlTopology: true,
    // Dev build: seed the /debug nav entry visible (UI visibility only).
    showDebug: true,
  },
  // ElohimClient configuration
  // Drives content operations (browser→doorway, tauri→local storage)
  client: {
    doorwayUrl: 'http://localhost:8888',
    apiKey: 'dev-elohim-auth-2024',
    // Direct storage URL for /db/* routes (bypasses doorway in local dev)
    storageUrl: 'http://localhost:8090',
    // For Tauri mode: personal nodes and conductor
    nodeUrls: [], // No personal nodes in dev
    holochainHAppId: 'elohim',
    holochainConductorUrl: 'ws://localhost:8888',
  },
};
