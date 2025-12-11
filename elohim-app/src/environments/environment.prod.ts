import { LogLevel } from './environment.types';

export const environment = {
  production: true,
  logLevel: 'error' as LogLevel,
  environment: 'production',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true  // Use embedded Kuzu WASM database
};