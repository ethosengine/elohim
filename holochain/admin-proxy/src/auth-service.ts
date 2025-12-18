/**
 * Authentication Service for Hosted Humans.
 *
 * Provides core authentication logic including:
 * - User registration (creating credentials after Holochain identity exists)
 * - Login (password validation and token generation)
 * - Token validation and refresh
 *
 * This service coordinates between the UserStore and JWT module to provide
 * a clean API for the auth routes.
 */

import type { Config } from './config.js';
import { getUserStore, type CreateUserInput, type StoredUser } from './user-store.js';
import {
  generateToken,
  verifyToken,
  type TokenPayload,
  type DecodedToken,
} from './jwt.js';

// =============================================================================
// Types
// =============================================================================

/** Input for registering auth credentials */
export interface RegisterAuthInput {
  /** Holochain human ID (from register_human zome result) */
  humanId: string;
  /** Holochain agent public key (hex string) */
  agentPubKey: string;
  /** User identifier (email or username) */
  identifier: string;
  /** Type of identifier */
  identifierType: 'email' | 'username';
  /** Password for authentication */
  password: string;
}

/** Input for login */
export interface LoginInput {
  /** User identifier (email or username) */
  identifier: string;
  /** Password */
  password: string;
}

/** Successful authentication result */
export interface AuthSuccess {
  success: true;
  /** JWT access token */
  token: string;
  /** Holochain human ID */
  humanId: string;
  /** Holochain agent public key */
  agentPubKey: string;
  /** Token expiration time (ISO string) */
  expiresAt: string;
  /** User identifier */
  identifier: string;
}

/** Failed authentication result */
export interface AuthFailure {
  success: false;
  /** Error message */
  error: string;
  /** Error code for client handling */
  code?: 'INVALID_CREDENTIALS' | 'USER_EXISTS' | 'IDENTITY_EXISTS' | 'NOT_ENABLED' | 'VALIDATION_ERROR';
}

/** Authentication result union */
export type AuthResult = AuthSuccess | AuthFailure;

/** Token validation result */
export interface TokenValidation {
  valid: boolean;
  humanId?: string;
  agentPubKey?: string;
  identifier?: string;
  error?: string;
}

// =============================================================================
// Validation Helpers
// =============================================================================

/**
 * Validate email format.
 */
function isValidEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

/**
 * Validate password strength.
 * Requirements:
 * - Minimum 8 characters
 * - At least one letter
 * - At least one number (optional for now, just enforce length)
 */
function isValidPassword(password: string): { valid: boolean; error?: string } {
  if (password.length < 8) {
    return { valid: false, error: 'Password must be at least 8 characters' };
  }
  return { valid: true };
}

/**
 * Validate username format.
 */
function isValidUsername(username: string): boolean {
  // 3-30 chars, alphanumeric, underscores, hyphens
  const usernameRegex = /^[a-zA-Z0-9_-]{3,30}$/;
  return usernameRegex.test(username);
}

// =============================================================================
// Auth Service
// =============================================================================

/**
 * Authentication service for hosted human login/registration.
 */
export class AuthService {
  constructor(private readonly config: Config) {}

  /**
   * Register auth credentials for a Holochain identity.
   *
   * This is called AFTER the user has registered with Holochain (register_human zome).
   * It creates password credentials that allow them to re-authenticate later.
   *
   * @param input - Registration data including Holochain identity and password
   * @returns Auth result with token on success
   */
  async register(input: RegisterAuthInput): Promise<AuthResult> {
    // Check if password auth is enabled
    if (!this.config.enablePasswordAuth) {
      return {
        success: false,
        error: 'Password authentication is not enabled',
        code: 'NOT_ENABLED',
      };
    }

    // Validate identifier
    if (input.identifierType === 'email') {
      if (!isValidEmail(input.identifier)) {
        return {
          success: false,
          error: 'Invalid email format',
          code: 'VALIDATION_ERROR',
        };
      }
    } else {
      if (!isValidUsername(input.identifier)) {
        return {
          success: false,
          error: 'Invalid username format (3-30 alphanumeric characters, underscores, or hyphens)',
          code: 'VALIDATION_ERROR',
        };
      }
    }

    // Validate password
    const passwordValidation = isValidPassword(input.password);
    if (!passwordValidation.valid) {
      return {
        success: false,
        error: passwordValidation.error!,
        code: 'VALIDATION_ERROR',
      };
    }

    // Create user in store
    const userStore = getUserStore();
    const createInput: CreateUserInput = {
      humanId: input.humanId,
      agentPubKey: input.agentPubKey,
      identifier: input.identifier,
      identifierType: input.identifierType,
      password: input.password,
    };

    const result = await userStore.createUser(createInput);

    if (!result.success) {
      // Map store errors to auth result codes
      const code = result.error?.includes('identifier already exists')
        ? 'USER_EXISTS'
        : result.error?.includes('already has credentials')
        ? 'IDENTITY_EXISTS'
        : 'VALIDATION_ERROR';

      return {
        success: false,
        error: result.error!,
        code,
      };
    }

    // Generate token for immediate use
    const tokenPayload: TokenPayload = {
      humanId: input.humanId,
      agentPubKey: input.agentPubKey,
      identifier: input.identifier.toLowerCase().trim(),
      version: 1,
    };

    const token = generateToken(tokenPayload, this.config);
    const expiresAt = new Date(
      Date.now() + this.config.jwtExpirySeconds * 1000
    ).toISOString();

    console.log(`[AuthService] Registered user: ${input.identifier} -> ${input.humanId}`);

    return {
      success: true,
      token,
      humanId: input.humanId,
      agentPubKey: input.agentPubKey,
      expiresAt,
      identifier: input.identifier.toLowerCase().trim(),
    };
  }

  /**
   * Login with identifier and password.
   *
   * @param input - Login credentials
   * @returns Auth result with token on success
   */
  async login(input: LoginInput): Promise<AuthResult> {
    // Check if password auth is enabled
    if (!this.config.enablePasswordAuth) {
      return {
        success: false,
        error: 'Password authentication is not enabled',
        code: 'NOT_ENABLED',
      };
    }

    // Validate password
    const userStore = getUserStore();
    const result = await userStore.validatePassword(input.identifier, input.password);

    if (!result.success) {
      return {
        success: false,
        error: result.error!,
        code: 'INVALID_CREDENTIALS',
      };
    }

    const user = result.data!;

    // Update last login
    await userStore.updateLastLogin(user.humanId);

    // Generate token
    const tokenPayload: TokenPayload = {
      humanId: user.humanId,
      agentPubKey: user.agentPubKey,
      identifier: user.identifier,
      version: 1,
    };

    const token = generateToken(tokenPayload, this.config);
    const expiresAt = new Date(
      Date.now() + this.config.jwtExpirySeconds * 1000
    ).toISOString();

    console.log(`[AuthService] Login successful: ${user.identifier}`);

    return {
      success: true,
      token,
      humanId: user.humanId,
      agentPubKey: user.agentPubKey,
      expiresAt,
      identifier: user.identifier,
    };
  }

  /**
   * Validate a JWT token and return the associated user info.
   *
   * @param token - JWT token string
   * @returns Validation result with user info if valid
   */
  validateToken(token: string): TokenValidation {
    const result = verifyToken(token, this.config);

    if (!result.valid) {
      return {
        valid: false,
        error: result.error,
      };
    }

    const payload = result.payload!;

    return {
      valid: true,
      humanId: payload.humanId,
      agentPubKey: payload.agentPubKey,
      identifier: payload.identifier,
    };
  }

  /**
   * Refresh an existing token (if still valid or recently expired).
   *
   * @param token - Current JWT token
   * @returns New auth result with fresh token
   */
  async refresh(token: string): Promise<AuthResult> {
    // For now, just validate and re-issue
    // Future: support refresh tokens for longer sessions
    const validation = this.validateToken(token);

    if (!validation.valid) {
      return {
        success: false,
        error: validation.error ?? 'Invalid token',
        code: 'INVALID_CREDENTIALS',
      };
    }

    // Re-issue token
    const tokenPayload: TokenPayload = {
      humanId: validation.humanId!,
      agentPubKey: validation.agentPubKey!,
      identifier: validation.identifier!,
      version: 1,
    };

    const newToken = generateToken(tokenPayload, this.config);
    const expiresAt = new Date(
      Date.now() + this.config.jwtExpirySeconds * 1000
    ).toISOString();

    return {
      success: true,
      token: newToken,
      humanId: validation.humanId!,
      agentPubKey: validation.agentPubKey!,
      expiresAt,
      identifier: validation.identifier!,
    };
  }

  /**
   * Get user info from a valid token.
   *
   * @param token - JWT token string
   * @returns User info if token is valid, null otherwise
   */
  async getCurrentUser(token: string): Promise<{
    humanId: string;
    agentPubKey: string;
    identifier: string;
  } | null> {
    const validation = this.validateToken(token);

    if (!validation.valid) {
      return null;
    }

    return {
      humanId: validation.humanId!,
      agentPubKey: validation.agentPubKey!,
      identifier: validation.identifier!,
    };
  }
}

// =============================================================================
// Singleton Instance
// =============================================================================

let authServiceInstance: AuthService | null = null;

/**
 * Get the auth service instance.
 */
export function getAuthService(config: Config): AuthService {
  if (!authServiceInstance) {
    authServiceInstance = new AuthService(config);
  }
  return authServiceInstance;
}

/**
 * Reset the auth service (for testing).
 */
export function resetAuthService(): void {
  authServiceInstance = null;
}
