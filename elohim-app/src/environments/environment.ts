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
  // Admin access through authenticated proxy (publicly accessible)
  // App WebSocket still uses direct connection after auth token is issued
  holochain: {
    adminUrl: 'wss://holochain-proxy-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',
    proxyApiKey: 'dev-elohim-auth-2024',  // Authenticated access (not admin)
  }
};