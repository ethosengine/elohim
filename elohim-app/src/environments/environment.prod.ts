import { LogLevel } from './environment.types';

export const environment = {
  production: true,
  logLevel: 'error' as LogLevel,
  environment: 'production',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true,  // Use embedded Kuzu WASM database
  // Holochain Edge Node configuration (prod - not yet deployed)
  // Single proxy URL handles admin operations with authentication
  // TODO: Update proxyApiKey when prod Edge Node is deployed
  holochain: {
    adminUrl: 'wss://holochain.elohim.host',
    appUrl: 'wss://holochain.elohim.host',
    proxyApiKey: undefined,  // Set when prod is deployed
  }
};