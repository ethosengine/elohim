import { Environment, LogLevel } from './environment.types';

export const environment: Environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'development',
  gitHash: 'local-dev',
  // Holochain Edge Node configuration
  // In Eclipse Che: auto-detects and uses local dev-proxy with path-based routing
  // In deployed mode: uses remote admin-proxy with API key authentication
  holochain: {
    adminUrl: 'wss://doorway-dev.elohim.host',
    appUrl: 'wss://doorway-dev.elohim.host',  // Fallback for deployed mode
    // Auth URL - uses local admin-proxy in dev (port 8888)
    // In production, this should point to the admin-proxy HTTP endpoint
    authUrl: 'http://localhost:8888',
    proxyApiKey: 'dev-elohim-auth-2024',  // Authenticated access (not admin)
    useLocalProxy: true,  // Auto-detect Che and use local dev-proxy
    // Connection mode: 'auto' detects Tauri→direct, browser→doorway
    connectionMode: 'auto',
    // elohim-storage sidecar URL (for direct mode blob storage)
    storageUrl: 'http://localhost:8090',
  }
};