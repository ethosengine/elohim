import { Environment, LogLevel } from './environment.types';

// Shared configuration values
// IMPORTANT: Update PROD_API_KEY when changing production credentials
const DOORWAY_PROD_WSS = 'wss://doorway.elohim.host';
const DOORWAY_PROD_HTTPS = 'https://doorway.elohim.host';
const PROD_API_KEY = 'CHANGE-ME-prod-elohim-auth-2024';

export const environment: Environment = {
  production: true,
  logLevel: 'error' as LogLevel,
  environment: 'production',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (legacy - used by HolochainClientService)
  holochain: {
    adminUrl: DOORWAY_PROD_WSS,
    appUrl: DOORWAY_PROD_WSS,
    // IMPORTANT: This key must match the secret in edgenode-prod.yaml
    proxyApiKey: PROD_API_KEY,
  },
  // ElohimClient configuration (new - mode-aware content client)
  client: {
    doorwayUrl: DOORWAY_PROD_HTTPS,
    apiKey: PROD_API_KEY,
    nodeUrls: [],
    holochainAppId: 'elohim',
    holochainConductorUrl: DOORWAY_PROD_WSS,
  },
};
