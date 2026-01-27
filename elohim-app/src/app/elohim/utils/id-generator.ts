/**
 * Utility for generating unique IDs with prefixes
 *
 * NOTE: These IDs are for local client-side identification (map IDs, extensions,
 * negotiations). They are NOT used for cryptographic security. For secure IDs,
 * use the cryptographic ID generators in the appropriate service.
 */

/**
 * Generate a unique ID with a given prefix
 * @param prefix - Prefix for the ID (e.g., 'map-domain', 'ext', 'nego')
 * @returns A unique ID string
 * @example
 * generateId('map-domain') // returns 'map-domain-1701234567890-abc123xyz'
 */
export function generateId(prefix: string): string {
  const timestamp = Date.now();
  // NOSONAR: Math.random() is safe here - IDs are for local client-side identification,
  // not cryptographic security. The timestamp provides uniqueness guarantee.
  // eslint-disable-next-line sonarjs/pseudo-random
  const random = Math.random().toString(36).substring(2, 11);
  return `${prefix}-${timestamp}-${random}`;
}

/**
 * Generate a map ID for a specific map type
 * @param mapType - Type of map ('domain', 'person', 'collective')
 * @returns A unique map ID
 */
export function generateMapId(mapType: 'domain' | 'person' | 'collective'): string {
  return generateId(`map-${mapType}`);
}

/**
 * Generate an extension ID
 * @returns A unique extension ID
 */
export function generateExtensionId(): string {
  return generateId('ext');
}

/**
 * Generate a negotiation ID
 * @returns A unique negotiation ID
 */
export function generateNegotiationId(): string {
  return generateId('nego');
}
