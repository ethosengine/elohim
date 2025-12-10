import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true,
  // Holochain Edge Node configuration
  // Single proxy URL handles admin operations with authentication
  holochain: {
    adminUrl: 'wss://holochain-alpha.elohim.host',
    appUrl: 'wss://holochain-alpha.elohim.host',
    proxyApiKey: 'alpha-elohim-auth-2024',
  }
};
