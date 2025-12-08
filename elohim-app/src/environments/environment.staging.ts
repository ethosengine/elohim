import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'staging',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true,  // Use embedded Kuzu WASM database
  // Holochain Edge Node configuration (staging uses alpha for now)
  holochain: {
    adminUrl: 'wss://holochain-alpha.elohim.host/admin',
    appUrl: 'wss://holochain-alpha.elohim.host',
  }
};