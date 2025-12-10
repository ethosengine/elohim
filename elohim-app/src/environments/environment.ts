import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'development',
  gitHash: 'local-dev',
  // Use Kuzu WASM embedded graph database
  // Loaded via script tag injection from /assets/wasm/kuzu-wasm.js
  useKuzuDb: true,
  // Holochain Edge Node configuration
  // Single proxy URL handles admin operations with authentication
  // App interface proxying will be added in Phase 2
  holochain: {
    adminUrl: 'wss://holochain-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',  // Same URL - proxy will route both
    proxyApiKey: 'dev-elohim-auth-2024',  // Authenticated access (not admin)
  }
};