export const environment = {
  production: true,
  logLevel: 'error' as 'debug' | 'info' | 'error',
  environment: 'production',
  gitHash: process.env['GIT_HASH'] || 'unknown'
};