import { Environment, LogLevel } from './environment.types';

export const environment: Environment = {
  production: true,
  logLevel: 'error' as LogLevel,
  environment: 'production',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (production)
  // Single proxy URL handles admin operations with authentication
  holochain: {
    adminUrl: 'wss://holochain.elohim.host',
    appUrl: 'wss://holochain.elohim.host',
    // IMPORTANT: This key must match the secret in edgenode-prod.yaml
    // Update both when changing production credentials
    proxyApiKey: 'CHANGE-ME-prod-elohim-auth-2024',
  }
};