import { Environment, LogLevel } from './environment.types';

export const environment: Environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration
  // Alpha and Staging share doorway-dev for pre-production testing
  // This consolidated architecture enables RNA version testing before production
  holochain: {
    adminUrl: 'wss://doorway-dev.elohim.host',
    appUrl: 'wss://doorway-dev.elohim.host',
    proxyApiKey: 'dev-elohim-auth-2024',
  }
};
