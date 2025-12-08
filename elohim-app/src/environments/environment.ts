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
  holochain: {
    adminUrl: 'ws://localhost:4444',
    appUrl: 'ws://localhost:4445',
    // For Che workspace, use deployed Edge Node instead:
    // adminUrl: 'wss://holochain-dev.elohim.host/admin',
    // appUrl: 'wss://holochain-dev.elohim.host',
  }
};