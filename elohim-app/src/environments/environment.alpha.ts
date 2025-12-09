import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true,
  // Holochain Edge Node configuration
  holochain: {
    adminUrl: 'wss://holochain-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',
  }
};
