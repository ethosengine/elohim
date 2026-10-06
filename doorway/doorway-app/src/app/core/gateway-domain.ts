/**
 * The domain a doorway's accounts live under, from the host the portal is
 * served on: `doorway-alpha.elohim.host` serves accounts at
 * `alpha.elohim.host`. Any other host is its own account domain.
 *
 * Display only — the doorway re-qualifies every identifier itself
 * (`normalize_identifier` in doorway-service auth_routes.rs).
 */
export function gatewayDomain(hostname: string): string {
  return hostname.startsWith('doorway-') ? hostname.replace(/^doorway-/, '') : hostname;
}
