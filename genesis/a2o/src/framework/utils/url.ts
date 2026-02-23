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
  if (doorwayUrl.includes('localhost:8888')) return 'http://localhost:4200';
  return doorwayUrl
    .replace('doorway-alpha.', 'alpha.')
    .replace('doorway-staging.', 'staging.')
    .replace('doorway.', '');
}
