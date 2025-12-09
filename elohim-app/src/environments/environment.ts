import { LogLevel } from './environment.types';

export const environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'development',
  gitHash: 'local-dev',
  // Use Kuzu WASM embedded graph database
  // Loaded via script tag injection from /assets/wasm/kuzu-wasm.js
  useKuzuDb: true,
  // Holochain Edge Node configuration
  // For local dev with port-forward: kubectl port-forward -n ethosengine deploy/elohim-edgenode-dev 4444:4444
  // For Che workspace or deployed testing, use: wss://holochain-dev.elohim.host
  holochain: {
    adminUrl: 'wss://holochain-dev.elohim.host',
    appUrl: 'wss://holochain-dev.elohim.host',
  }
};