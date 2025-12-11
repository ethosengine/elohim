/**
 * Utility for generating unique IDs with prefixes
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
