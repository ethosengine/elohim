import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'info' as LogLevel,
  environment: 'staging',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration
  // Alpha and Staging share holochain-dev for pre-production testing
  // This allows RNA version testing before production deployment
  holochain: {
    adminUrl: 'wss://holochain-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',
    proxyApiKey: 'dev-elohim-auth-2024',
  }
};