import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'staging',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (staging uses alpha for now)
  // Single proxy URL handles admin operations with authentication
  holochain: {
    adminUrl: 'wss://holochain-alpha.elohim.host',
    appUrl: 'wss://holochain-alpha.elohim.host',
    proxyApiKey: 'alpha-elohim-auth-2024',
  }
};