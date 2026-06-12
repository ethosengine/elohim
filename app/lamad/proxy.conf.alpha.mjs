/**
 * Angular Dev Server Proxy Configuration — LIVE ALPHA TARGET.
 *
 * Mirror of app/elohim-app/proxy.conf.alpha.mjs for the lamad bundle: proxies
 * substrate reads to the deployed alpha doorway so the local dev server (your
 * in-progress bundle code) renders live deployed-peer data — no local stack,
 * no seeding. Read-mostly polish loops only; never drive write flows through
 * this config against alpha.
 */
const target = process.env.DOORWAY_TARGET ?? 'https://doorway-alpha.elohim.host';

export default [
  {
    context: ['/api', '/db', '/blob', '/apps', '/epr-head', '/account', '/health', '/p2p'],
    target,
    secure: true,
    changeOrigin: true,
    logLevel: 'debug',
  },
];
