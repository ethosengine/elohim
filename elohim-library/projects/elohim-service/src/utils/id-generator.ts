/**
 * Utility functions for generating and normalizing IDs
 */

/**
 * Normalizes an array of strings into a kebab-case ID
 * @param parts - Array of string parts to normalize
 * @returns A normalized kebab-case ID
 * @example
 * normalizeId(['My', 'Epic', 'Name']) // returns 'my-epic-name'
 * normalizeId(['user_profile', 'ADMIN']) // returns 'user-profile-admin'
 */
export function normalizeId(parts: string[]): string {
  return parts
    .map(p => p.toLowerCase().replace(/[^a-z0-9]/g, '-'))
    .join('-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}
