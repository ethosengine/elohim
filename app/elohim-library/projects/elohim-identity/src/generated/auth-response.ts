/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/auth-response.schema.json -- DO NOT EDIT */

/**
 * Source of truth: doorway operational session state (Operational, Category C). Consolidated response of POST /auth/register, POST /auth/login, and POST /auth/refresh — a JWT session minted from the doorway's user store; reconstructed per request, never persisted as substrate truth. Rust wire authority: doorway/doorway-service/src/routes/auth_routes.rs AuthResponse (validated by that crate's tests/schema_contract.rs).
 */
export interface AuthResponse {
  /**
   * Doorway-issued JWT bearer token
   */
  token: string;
  /**
   * Holochain human ID for the authenticated session
   */
  humanId: string;
  /**
   * Holochain agent public key for the authenticated session
   */
  agentPubKey: string;
  /**
   * Login identifier (e.g. email) of the authenticated human
   */
  identifier: string;
  /**
   * Unix timestamp (seconds) when the JWT expires (the `exp` claim)
   */
  expiresAt: number;
  /**
   * Doorway that issued this token (for federation). Absent when not configured (serde skip_serializing_if)
   */
  doorwayId?: string;
  /**
   * Doorway URL for cross-doorway validation. Absent when not configured (serde skip_serializing_if)
   */
  doorwayUrl?: string;
  /**
   * Holochain installed app ID for this user (multi-conductor routing). Absent when not applicable (serde skip_serializing_if)
   */
  installedAppId?: string;
  profile?: HumanProfileResponse;
  /**
   * True when the human's stewardship is substrate-confirmed. ABSENT when false (serde skip_serializing_if = Not::not), so presence implies true
   */
  isSteward?: boolean;
  /**
   * First reachable portal host URL for this human, when isSteward is true and at least one registered host responds to a health probe. Absent when not a steward or no host is reachable (serde skip_serializing_if)
   */
  portalHostUrl?: string;
}
/**
 * Human profile (returned on registration). Absent otherwise (serde skip_serializing_if)
 */
export interface HumanProfileResponse {
  /**
   * Human entry identifier
   */
  id: string;
  /**
   * Display name chosen at registration
   */
  displayName: string;
  /**
   * Optional bio/description. Absent when not provided (serde skip_serializing_if)
   */
  bio?: string;
  /**
   * User interests/affinities
   */
  affinities: string[];
  /**
   * Profile visibility (public, connections, private)
   */
  profileReach: string;
  /**
   * Optional location. Absent when not provided (serde skip_serializing_if)
   */
  location?: string;
  /**
   * ISO-8601 timestamp when the profile was created
   */
  createdAt: string;
  /**
   * ISO-8601 timestamp when the profile was last updated
   */
  updatedAt: string;
}
