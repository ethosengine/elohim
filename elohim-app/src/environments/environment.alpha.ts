import { Environment, LogLevel } from './environment.types';

export const environment: Environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (legacy - used by HolochainClientService)
  // Alpha and Staging share doorway-dev for pre-production testing
  holochain: {
    adminUrl: 'wss://doorway-dev.elohim.host',
    appUrl: 'wss://doorway-dev.elohim.host',
    proxyApiKey: 'dev-elohim-auth-2024',
  },
  // ElohimClient configuration (new - mode-aware content client)
  // Browser mode: all content via doorway proxy
  client: {
    doorwayUrl: 'https://doorway-dev.elohim.host',
    apiKey: 'dev-elohim-auth-2024',
    nodeUrls: [],  // No personal nodes in alpha
    holochainAppId: 'elohim',
    holochainConductorUrl: 'wss://doorway-dev.elohim.host',
  },
};
