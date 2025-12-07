import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'staging',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true  // Use embedded Kuzu WASM database
};