export const environment = {
  production: false,
  logLevel: 'debug' as 'debug' | 'info' | 'error',
  environment: 'development',
  gitHash: 'local-dev',
  // Use Kuzu WASM embedded graph database
  // Loaded via script tag injection from /assets/wasm/kuzu-wasm.js
  useKuzuDb: true
};