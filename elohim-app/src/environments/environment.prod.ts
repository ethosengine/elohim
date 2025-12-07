export const environment = {
  production: true,
  logLevel: 'error' as 'debug' | 'info' | 'error',
  environment: 'production',
  gitHash: 'GIT_HASH_PLACEHOLDER',
  useKuzuDb: true  // Use embedded Kuzu WASM database
};