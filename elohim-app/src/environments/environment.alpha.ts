import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration
  holochain: {
    adminUrl: 'wss://holochain-alpha.elohim.host/admin',
    appUrl: 'wss://holochain-alpha.elohim.host',
  }
};
