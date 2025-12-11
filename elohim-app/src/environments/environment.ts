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
  // In Eclipse Che: auto-detects and uses local dev-proxy with path-based routing
  // In deployed mode: uses remote admin-proxy with API key authentication
  holochain: {
    adminUrl: 'wss://holochain-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',  // Fallback for deployed mode
    proxyApiKey: 'dev-elohim-auth-2024',  // Authenticated access (not admin)
    useLocalProxy: true,  // Auto-detect Che and use local dev-proxy
  }
};