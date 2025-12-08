import { LogLevel } from './environment.types';

export const environment = {
  production: true,
  logLevel: 'error' as LogLevel,
  environment: 'production',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true,  // Use embedded Kuzu WASM database
  // Holochain Edge Node configuration (prod - not yet deployed)
  holochain: {
    adminUrl: 'wss://holochain.elohim.host/admin',
    appUrl: 'wss://holochain.elohim.host',
  }
};