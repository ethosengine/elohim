import { Environment, LogLevel } from './environment.types';

// Shared configuration values
const DOORWAY_DEV_WSS = 'wss://doorway-dev.elohim.host';
const DOORWAY_DEV_HTTPS = 'https://doorway-dev.elohim.host';
const DEV_API_KEY = 'dev-elohim-auth-2024';

export const environment: Environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (legacy - used by HolochainClientService)
  // Alpha and Staging share doorway-dev for pre-production testing
  holochain: {
    adminUrl: DOORWAY_DEV_WSS,
    appUrl: DOORWAY_DEV_WSS,
    proxyApiKey: DEV_API_KEY,
  },
  // ElohimClient configuration (new - mode-aware content client)
  // Browser mode: all content via doorway proxy
  client: {
    doorwayUrl: DOORWAY_DEV_HTTPS,
    apiKey: DEV_API_KEY,
    nodeUrls: [], // No personal nodes in alpha
    holochainAppId: 'elohim',
    holochainConductorUrl: DOORWAY_DEV_WSS,
  },
};
