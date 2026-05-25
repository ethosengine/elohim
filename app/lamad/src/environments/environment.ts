// TODO(B18c-followup): This is a lamad-bundle-local environment file.
// When doorway serves lamad at /lamad/, these values are injected at build time.
// Mirrors elohim-app/src/environments/environment.ts — keep in sync
// until a shared environment package is extracted (post-MVP).

export const environment = {
  production: false,
  logLevel: 'debug',
  environment: 'development',
  gitHash: 'local-dev',
  cache: {
    preferWasm: true,
  },
  client: {
    doorwayUrl: 'http://localhost:8888',
    apiKey: 'dev-elohim-auth-2024',
    storageUrl: 'http://localhost:8090',
    nodeUrls: [] as string[],
    holochainHAppId: 'elohim',
    holochainConductorUrl: 'ws://localhost:8888',
  },
};
