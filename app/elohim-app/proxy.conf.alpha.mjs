/**
 * Angular Dev Server Proxy Configuration — LIVE ALPHA TARGET (spike)
 *
 * Same contexts as proxy.conf.mjs, but proxies to the deployed alpha doorway
 * instead of a local doorway at :8888. This gives the local dev server (your
 * in-progress UI code) live deployed-peer data: no local stack, no seeding.
 *
 * Override the target with DOORWAY_TARGET. Intended for read-mostly polish
 * loops; do not drive write flows through this config against alpha.
 */
const target = process.env.DOORWAY_TARGET ?? 'https://doorway-alpha.elohim.host';

export default [
  {
    context: ['/api', '/db', '/blob', '/apps', '/epr-head', '/account', '/health', '/p2p', '/admin'],
    target,
    secure: true,
    changeOrigin: true,
    logLevel: 'debug',
  },
];
