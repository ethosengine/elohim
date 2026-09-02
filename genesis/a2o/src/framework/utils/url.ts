/**
 * URL utilities shared across step definitions and framework code.
 */

/**
 * Resolve the app URL from a doorway URL.
 *
 * doorway-alpha.elohim.host  -> alpha.elohim.host
 * doorway-staging.elohim.host -> staging.elohim.host
 * doorway.elohim.host        -> elohim.host
 * localhost:8888              -> localhost:4200
 */
export function doorwayToAppUrl(doorwayUrl: string): string {
  // E2E_APP_URL wins when set. On the household mesh the DOORWAY serves the app
  // itself, so app and portal are one origin and a portal return-URL is a plain
  // same-origin redirect. The localhost:4200 default below is the split-origin
  // local-dev shape (`ng serve` beside a doorway), which is a different thing
  // and must stay the default for that workflow.
  const override = process.env['E2E_APP_URL'];
  if (override) {
    let o = override;
    while (o.endsWith('/')) o = o.slice(0, -1);
    return o;
  }
  if (doorwayUrl.includes('localhost:8888')) return 'http://localhost:4200';
  return doorwayUrl
    .replace('doorway-alpha.', 'alpha.')
    .replace('doorway-staging.', 'staging.')
    .replace('doorway.', '');
}
