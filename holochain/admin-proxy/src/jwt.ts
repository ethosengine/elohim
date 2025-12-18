/**
 * JWT Token Handling for Hosted Human Authentication.
 *
 * Provides functions for generating and validating JWT tokens used to
 * authenticate hosted humans to the edge node.
 *
 * Security notes:
 * - Tokens are signed with HS256 (HMAC-SHA256)
 * - Default expiry is 1 hour
 * - In production, JWT_SECRET should be a strong random value from environment
 */

import jwt from 'jsonwebtoken';
import type { Config } from './config.js';

// =============================================================================
// Types
// =============================================================================

/** Payload stored in JWT token */
export interface TokenPayload {
  /** Holochain human ID */
  humanId: string;
  /** Holochain agent public key (hex string) */
  agentPubKey: string;
  /** User identifier (email/username) */
  identifier: string;
  /** Token version (for future invalidation) */
  version: number;
}

/** Decoded token with standard JWT claims */
export interface DecodedToken extends TokenPayload {
  /** Issued at (Unix timestamp) */
  iat: number;
  /** Expiration time (Unix timestamp) */
  exp: number;
}

/** Result of token validation */
export interface TokenValidationResult {
  valid: boolean;
  payload?: DecodedToken;
  error?: string;
}

// =============================================================================
// Token Generation
// =============================================================================

/**
 * Generate a JWT token for an authenticated user.
 *
 * @param payload - User data to encode in token
 * @param config - Server configuration (for secret and expiry)
 * @returns Signed JWT token string
 */
export function generateToken(
  payload: TokenPayload,
  config: Config
): string {
  const secret = config.jwtSecret;
  const expirySeconds = config.jwtExpirySeconds;

  const token = jwt.sign(payload, secret, {
    algorithm: 'HS256',
    expiresIn: expirySeconds,
  });

  return token;
}

/**
 * Generate a refresh token with longer expiry.
 * (For future use - allows token refresh without re-authentication)
 *
 * @param payload - User data to encode
 * @param config - Server configuration
 * @returns Signed refresh token
 */
export function generateRefreshToken(
  payload: TokenPayload,
  config: Config
): string {
  const secret = config.jwtSecret;
  // Refresh tokens last 7 days
  const expirySeconds = 7 * 24 * 60 * 60;

  const token = jwt.sign(
    { ...payload, type: 'refresh' },
    secret,
    {
      algorithm: 'HS256',
      expiresIn: expirySeconds,
    }
  );

  return token;
}

// =============================================================================
// Token Validation
// =============================================================================

/**
 * Verify and decode a JWT token.
 *
 * @param token - JWT token string
 * @param config - Server configuration (for secret)
 * @returns Validation result with decoded payload if valid
 */
export function verifyToken(
  token: string,
  config: Config
): TokenValidationResult {
  try {
    const secret = config.jwtSecret;
    const decoded = jwt.verify(token, secret, {
      algorithms: ['HS256'],
    }) as DecodedToken;

    return {
      valid: true,
      payload: decoded,
    };
  } catch (err) {
    if (err instanceof jwt.TokenExpiredError) {
      return {
        valid: false,
        error: 'Token expired',
      };
    }

    if (err instanceof jwt.JsonWebTokenError) {
      return {
        valid: false,
        error: 'Invalid token',
      };
    }

    return {
      valid: false,
      error: 'Token validation failed',
    };
  }
}

/**
 * Decode a token without verification (for debugging).
 * WARNING: Do not trust the payload without verification!
 *
 * @param token - JWT token string
 * @returns Decoded payload or null if malformed
 */
export function decodeToken(token: string): DecodedToken | null {
  try {
    const decoded = jwt.decode(token) as DecodedToken | null;
    return decoded;
  } catch {
    return null;
  }
}

/**
 * Check if a token is close to expiring (within threshold).
 * Useful for proactive token refresh.
 *
 * @param token - Decoded token with exp claim
 * @param thresholdSeconds - Seconds before expiry to consider "expiring soon"
 * @returns True if token expires within threshold
 */
export function isTokenExpiringSoon(
  token: DecodedToken,
  thresholdSeconds: number = 300
): boolean {
  const nowSeconds = Math.floor(Date.now() / 1000);
  return token.exp - nowSeconds < thresholdSeconds;
}

/**
 * Get remaining time until token expiry.
 *
 * @param token - Decoded token with exp claim
 * @returns Seconds until expiry (negative if already expired)
 */
export function getTokenTimeRemaining(token: DecodedToken): number {
  const nowSeconds = Math.floor(Date.now() / 1000);
  return token.exp - nowSeconds;
}

// =============================================================================
// Token Extraction
// =============================================================================

/**
 * Extract token from Authorization header.
 * Supports "Bearer <token>" format.
 *
 * @param authHeader - Authorization header value
 * @returns Token string or null if not found/invalid format
 */
export function extractTokenFromHeader(authHeader: string | null): string | null {
  if (!authHeader) {
    return null;
  }

  // Support "Bearer <token>" format
  if (authHeader.startsWith('Bearer ')) {
    return authHeader.slice(7).trim();
  }

  // Also support raw token (for flexibility)
  if (!authHeader.includes(' ')) {
    return authHeader.trim();
  }

  return null;
}

/**
 * Extract token from URL query parameter.
 *
 * @param url - Full URL string
 * @param paramName - Query parameter name (default: 'token')
 * @returns Token string or null if not found
 */
export function extractTokenFromUrl(
  url: string,
  paramName: string = 'token'
): string | null {
  try {
    const parsed = new URL(url, 'http://localhost');
    return parsed.searchParams.get(paramName);
  } catch {
    return null;
  }
}
