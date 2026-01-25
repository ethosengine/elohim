import { Environment, LogLevel } from './environment.types';

export const environment: Environment = {
  production: true,
  logLevel: 'error' as LogLevel,
  environment: 'production',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (legacy - used by HolochainClientService)
  holochain: {
    adminUrl: 'wss://doorway.elohim.host',
    appUrl: 'wss://doorway.elohim.host',
    // IMPORTANT: This key must match the secret in edgenode-prod.yaml
    proxyApiKey: 'CHANGE-ME-prod-elohim-auth-2024',
  },
  // ElohimClient configuration (new - mode-aware content client)
  // IMPORTANT: Update both apiKey values when changing production credentials
  client: {
    doorwayUrl: 'https://doorway.elohim.host',
    apiKey: 'CHANGE-ME-prod-elohim-auth-2024',
    nodeUrls: [],
    holochainAppId: 'elohim',
    holochainConductorUrl: 'wss://doorway.elohim.host',
  },
};
