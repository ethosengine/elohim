export const environment = {
  production: false,
  logLevel: 'debug' as 'debug' | 'info' | 'error',
  environment: 'staging',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true  // Use embedded Kuzu WASM database
};