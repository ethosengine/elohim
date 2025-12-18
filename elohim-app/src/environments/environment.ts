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
    adminUrl: 'wss://holochain-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',  // Fallback for deployed mode
    // Auth URL - uses local admin-proxy in dev (port 8888)
    // In production, this should point to the admin-proxy HTTP endpoint
    authUrl: 'http://localhost:8888',
    proxyApiKey: 'dev-elohim-auth-2024',  // Authenticated access (not admin)
    useLocalProxy: true,  // Auto-detect Che and use local dev-proxy
  }
};