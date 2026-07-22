import { Environment, LogLevel } from './environment.types';

// Shared configuration values
const DOORWAY_ALPHA_WSS = 'wss://doorway-alpha.elohim.host';
const DOORWAY_ALPHA_HTTPS = 'https://doorway-alpha.elohim.host';
const DEV_API_KEY = 'dev-elohim-auth-2024';

export const environment: Environment = {
  production: false,
  logLevel: 'debug' as LogLevel,
  environment: 'alpha',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  // Holochain Edge Node configuration (legacy - used by HolochainClientService)
  // Alpha environment uses doorway-alpha for pre-production testing
  holochain: {
    adminUrl: DOORWAY_ALPHA_WSS,
    appUrl: DOORWAY_ALPHA_WSS,
    proxyApiKey: DEV_API_KEY,
  },
  // ElohimClient configuration (new - mode-aware content client)
  // Browser mode: all content via doorway proxy
  client: {
    doorwayUrl: DOORWAY_ALPHA_HTTPS,
    // Doorway-B apex on the same alpha substrate — multi-host failover
    // ("logical anycast", 2026-07-16 dual-WAN utility-plane failover design
    // §3a). The apiBaseUrlInterceptor retries GET/HEAD against this host on
    // a status-0 (network-level) failure of the primary; writes are never
    // auto-retried. See genesis/orchestrator/manifests/doorway/alpha-b.yaml.
    doorwayFallbacks: ['https://elohim.host'],
    apiKey: DEV_API_KEY,
    nodeUrls: [], // No personal nodes in alpha
    holochainHAppId: 'elohim',
    holochainConductorUrl: DOORWAY_ALPHA_WSS,
  },
  cache: {
    // WASM bundle is fetched per-build from Harbor at ${happVersion}; when the
    // DNA pipeline hasn't yet published a matching artifact for the app's
    // commit, the asset 404s. TypeScript fallback is functionally complete.
    preferWasm: false,
  },
  features: {
    // Routes topology fetches through /api/v1/graphql. See environment.ts
    // for rationale. Flipped 2026-05-19 (L6 viewer.* symmetry pass).
    useGraphqlTopology: true,
    // Alpha: /debug nav entry hidden by default (still URL-reachable; gating is
    // visibility, not access — the endpoints it reads are public).
    showDebug: false,
  },
};
