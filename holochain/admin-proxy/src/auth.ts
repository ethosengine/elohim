import { PermissionLevel } from './permissions.js';
import type { Config } from './config.js';

/**
 * Validates an API key and returns the corresponding permission level.
 * Returns null if the key is invalid.
 *
 * In dev mode, grants ADMIN access to all connections (no key required).
 */
export function validateApiKey(
  apiKey: string | null,
  config: Config
): PermissionLevel | null {
  // Dev mode: grant full access to everyone
  if (config.devMode) {
    return PermissionLevel.ADMIN;
  }

  // No API key = public access (read-only)
  // Empty string is treated as invalid (user tried to provide key but it's empty)
  if (apiKey === null) {
    return PermissionLevel.PUBLIC;
  }

  if (apiKey === '') {
    return null;
  }

  if (apiKey === config.apiKeyAdmin) {
    return PermissionLevel.ADMIN;
  }

  if (apiKey === config.apiKeyAuthenticated) {
    return PermissionLevel.AUTHENTICATED;
  }

  // Invalid API key
  return null;
}

/**
 * Extract API key from WebSocket upgrade request URL
 */
export function extractApiKey(url: string, host: string): string | null {
  try {
    const parsed = new URL(url, `http://${host}`);
    return parsed.searchParams.get('apiKey');
  } catch {
    return null;
  }
}
