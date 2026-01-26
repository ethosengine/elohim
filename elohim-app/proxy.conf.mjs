/**
 * Angular Dev Server Proxy Configuration (ESM format)
 *
 * Proxies API requests to doorway (localhost:8888) during development.
 * Using array format with context for Angular 19's esbuild builder compatibility.
 *
 * @see https://angular.dev/tools/cli/serve#proxying-to-a-backend-server
 */
export default [
  {
    context: ['/api', '/db', '/blob', '/apps', '/health'],
    target: 'http://localhost:8888',
    secure: false,
    changeOrigin: true,
    logLevel: 'debug',
  },
];
