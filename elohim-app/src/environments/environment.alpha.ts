import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration
  // Alpha uses dev instance until alpha holochain is deployed
  holochain: {
    adminUrl: 'wss://holochain-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',
    proxyApiKey: 'dev-elohim-auth-2024',
  }
};
