export const environment = {
  production: false,
  logLevel: 'debug' as 'debug' | 'info' | 'error',
  environment: 'staging',
  gitHash: process.env['GIT_HASH'] || 'unknown'
};