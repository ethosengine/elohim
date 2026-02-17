import { Environment, LogLevel } from './environment.types';

// Shared configuration values
const DOORWAY_ALPHA_WSS = 'wss://doorway-alpha.elohim.host';
const DOORWAY_ALPHA_HTTPS = 'https://doorway-alpha.elohim.host';
const DEV_API_KEY = 'dev-elohim-auth-2024';

export const environment: Environment = {
  production: false,
  logLevel: 'info' as LogLevel,
  environment: 'staging',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (legacy - used by HolochainClientService)
  // Staging environment uses doorway-alpha for pre-production testing
  holochain: {
    adminUrl: DOORWAY_ALPHA_WSS,
    appUrl: DOORWAY_ALPHA_WSS,
    proxyApiKey: DEV_API_KEY,
  },
  // ElohimClient configuration (new - mode-aware content client)
  client: {
    doorwayUrl: DOORWAY_ALPHA_HTTPS,
    apiKey: DEV_API_KEY,
    nodeUrls: [],
    holochainAppId: 'elohim',
    holochainConductorUrl: DOORWAY_ALPHA_WSS,
  },
};
