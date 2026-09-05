/**
 * Authentication Models for Hosted Human Login/Logout.
 *
 * These types define the authentication layer that sits on top of the
 * Holochain identity system, enabling hosted humans to login/logout.
 *
 * Design notes:
 * - AuthProvider interface allows swappable auth implementations (password, passkey, OAuth)
 * - AuthState tracks the current authentication status
 * - All tokens are stored in localStorage with defined keys
 */

// =============================================================================
// Storage Keys
// =============================================================================

/** localStorage key for auth token */
export const AUTH_TOKEN_KEY = 'elohim-auth-token';

/** localStorage key for auth provider type */
export const AUTH_PROVIDER_KEY = 'elohim-auth-provider';

/** localStorage key for auth expiry timestamp */
export const AUTH_EXPIRY_KEY = 'elohim-auth-expiry';

/** localStorage key for stored identifier */
export const AUTH_IDENTIFIER_KEY = 'elohim-auth-identifier';

/** localStorage key for human ID (persisted across page refreshes) */
export const AUTH_HUMAN_ID_KEY = 'elohim-auth-human-id';

/** localStorage key for agent public key (persisted across page refreshes) */
export const AUTH_AGENT_PUB_KEY_KEY = 'elohim-auth-agent-pub-key';

/** localStorage key for installed app ID (multi-conductor routing) */
export const AUTH_INSTALLED_APP_ID_KEY = 'elohim-installed-app-id';

// =============================================================================
// Auth Provider Types
// =============================================================================

/** Supported authentication provider types */
export type AuthProviderType = 'password' | 'passkey' | 'oauth' | 'tauri';

/** Identifier type for authentication */
export type IdentifierType = 'email' | 'username';

// =============================================================================
// Credentials
// =============================================================================

/** Credentials for password-based login */
export interface PasswordCredentials {
  type: 'password';
  identifier: string;
  password: string;
}

/** Credentials for passkey-based login (future) */
export interface PasskeyCredentials {
  type: 'passkey';
  assertion: unknown; // WebAuthn assertion object
}

/** Credentials for OAuth login (future) */
export interface OAuthCredentials {
  type: 'oauth';
  provider: string;
  token: string;
}

/** Union of all credential types */
export type AuthCredentials = PasswordCredentials | PasskeyCredentials | OAuthCredentials;

// @coverage: 75.0% (2026-02-24)

// There is no RegisterCredentials type here any more. Registration is not a
// capability of this app: a hosted human's account is created at their
// doorway's own portal, and profile fields (bio, affinities, reach, location)
// are set on the profile surface after sign-in, never at sign-up.

// =============================================================================
// Auth Results
// =============================================================================

/** Successful authentication result */
export interface AuthSuccess {
  success: true;
  /** JWT access token */
  token: string;
  /** Holochain human ID */
  humanId: string;
  /** Holochain agent public key */
  agentPubKey: string;
  /** Token expiration - Unix timestamp (seconds) or ISO string */
  expiresAt: number | string;
  /** User identifier */
  identifier: string;
  /** Holochain installed app ID for this user (multi-conductor routing) */
  installedAppId?: string;
}

/** Failed authentication result */
export interface AuthFailure {
  success: false;
  /** User-friendly error message */
  error: string;
  /** Error code for programmatic handling */
  code?: AuthErrorCode;
}

/** Authentication error codes */
export type AuthErrorCode =
  | 'INVALID_CREDENTIALS'
  | 'USER_EXISTS'
  | 'IDENTITY_EXISTS'
  | 'NOT_ENABLED'
  | 'VALIDATION_ERROR'
  | 'NETWORK_ERROR'
  | 'TOKEN_EXPIRED';

/** Authentication result union */
export type AuthResult = AuthSuccess | AuthFailure;

// =============================================================================
// Auth State
// =============================================================================

/** Complete authentication state */
export interface AuthState {
  /** Whether user is currently authenticated */
  isAuthenticated: boolean;
  /** Current JWT token (null if not authenticated) */
  token: string | null;
  /** Holochain human ID (null if not authenticated) */
  humanId: string | null;
  /** Holochain agent public key (null if not authenticated) */
  agentPubKey: string | null;
  /** Token expiration time (null if not authenticated) */
  expiresAt: Date | null;
  /** Auth provider used (null if not authenticated) */
  provider: AuthProviderType | null;
  /** User identifier (email/username) */
  identifier: string | null;
  /** Whether an auth operation is in progress */
  isLoading: boolean;
  /** Current error message (null if no error) */
  error: string | null;
}

/** Initial auth state (not authenticated) */
export const INITIAL_AUTH_STATE: AuthState = {
  isAuthenticated: false,
  token: null,
  humanId: null,
  agentPubKey: null,
  expiresAt: null,
  provider: null,
  identifier: null,
  isLoading: false,
  error: null,
};

// =============================================================================
// Auth Provider Interface
// =============================================================================

/**
 * Abstract interface for authentication providers.
 *
 * Implementations:
 * - OAuthAuthProvider: authorization-code flow against the human's doorway
 * - PasskeyAuthProvider (future): WebAuthn passkey authentication
 *
 * No implementation registers a credential — apps are relying parties, so
 * there is no `register` capability on this interface and no provider that
 * takes a password. This abstraction allows swapping authentication methods
 * without changing the rest of the application.
 */
export interface AuthProvider {
  /** Provider type identifier */
  readonly type: AuthProviderType;

  /**
   * Authenticate with credentials.
   *
   * @param credentials - Provider-specific credentials
   * @returns Authentication result with token on success
   */
  login(credentials: AuthCredentials): Promise<AuthResult>;

  /**
   * Clear authentication state.
   * Called on logout.
   */
  logout(): Promise<void>;

  /**
   * Refresh an expiring token (optional).
   *
   * @param token - Current JWT token
   * @returns New authentication result
   */
  refreshToken?(token: string): Promise<AuthResult>;
}

// =============================================================================
// API Types (matching edge node)
// =============================================================================

// POST /auth/register and POST /auth/login have no request type here: this app
// posts to neither. Both are the doorway portal's own surfaces.

/**
 * Auth wire shapes — the schema-contract-pinned generated contracts,
 * re-exported from @elohim/identity (single source; Rust wire authority:
 * doorway/doorway-service/src/routes/auth_routes.rs, validated by that
 * crate's schema_contract tests). The drifted hand-written duplicates
 * (AuthResponse with `expiresAt: number | string`, and CurrentUserResponse —
 * a renamed /auth/me shape) were retired (auth-wire plan Task 4).
 *
 * Note: the wire `expiresAt` is a Unix-seconds number. The app-level
 * AuthSuccess above deliberately keeps the looser `number | string` because
 * non-wire flows (Tauri/OAuth handoff) feed ISO strings through
 * parseExpiryDate.
 */
export type { AuthResponse, HumanProfileResponse, MeResponse } from '@elohim/identity';

/** Response from auth endpoints on error */
export interface AuthErrorResponse {
  error: string;
  code?: AuthErrorCode;
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Check if a token is expired or expiring soon.
 *
 * @param expiresAt - Token expiration date
 * @param thresholdMs - Buffer time before expiry (default: 5 minutes)
 * @returns True if token is expired or expiring soon
 */
export function isTokenExpiringSoon(
  expiresAt: Date | null,
  thresholdMs: number = 5 * 60 * 1000
): boolean {
  if (!expiresAt) return true;
  return Date.now() + thresholdMs >= expiresAt.getTime();
}

/**
 * Parse expiry value to Date.
 *
 * Handles multiple formats:
 * - ISO date string: "2026-01-13T10:15:47Z"
 * - Unix timestamp (seconds): 1736776547 or "1736776547"
 * - Unix timestamp (milliseconds): 1736776547000 or "1736776547000"
 *
 * @param expiresAt - Expiry value (ISO string, Unix timestamp number, or timestamp string)
 * @returns Date object or null if invalid
 */
export function parseExpiryDate(expiresAt: string | number | null | undefined): Date | null {
  if (expiresAt === null || expiresAt === undefined) return null;

  try {
    // Handle numeric timestamps (from server returning u64)
    const numValue = typeof expiresAt === 'number' ? expiresAt : Number(expiresAt);

    if (!Number.isNaN(numValue) && numValue > 0) {
      // Distinguish between seconds and milliseconds timestamps
      // Timestamps in seconds: < 10000000000 (before year 2286)
      // Timestamps in milliseconds: >= 10000000000
      const msTimestamp = numValue < 10000000000 ? numValue * 1000 : numValue;
      const date = new Date(msTimestamp);
      if (!Number.isNaN(date.getTime())) {
        return date;
      }
    }

    // Try parsing as ISO string (fallback for non-numeric strings)
    if (typeof expiresAt === 'string') {
      const date = new Date(expiresAt);
      if (!Number.isNaN(date.getTime())) {
        return date;
      }
    }

    return null;
  } catch {
    return null;
  }
}

export { type ProfileReach } from './identity.model';
