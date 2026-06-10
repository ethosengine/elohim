/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/me-response.schema.json -- DO NOT EDIT */

/**
 * Source of truth: doorway operational session state (Operational, Category C). GET /auth/me — current-session info decoded from the bearer JWT plus the issuing doorway's identity; reconstructed per request, never persisted. The unauthenticated path returns 401 with no MeResponse body. Rust wire authority: doorway/doorway-service/src/routes/auth_routes.rs MeResponse (validated by that crate's tests/schema_contract.rs).
 */
export interface MeResponse {
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
   * Doorway permission level from the JWT claims — Display form of doorway's PermissionLevel enum: 'PUBLIC' | 'AUTHENTICATED' | 'ADMIN'
   */
  permissionLevel: string;
  /**
   * Doorway that issued this token (for federation). Absent when not configured (serde skip_serializing_if)
   */
  doorwayId?: string;
  /**
   * Doorway URL for cross-doorway validation. Absent when not configured (serde skip_serializing_if)
   */
  doorwayUrl?: string;
  /**
   * Whether the caller has a valid session. Always true on the 200 path; the unauthenticated path returns 401 (no MeResponse body)
   */
  authenticated: boolean;
  /**
   * Auth mode for the current session: 'doorway-host' (doorway runs the conductor) or 'peer-conductor' (conductor on a peer-managed instance). MVP: always 'doorway-host'
   */
  trustMode: string;
  authority: AuthorityRef;
  /**
   * Conductor's reachable URL or peer-id descriptor when trustMode is 'peer-conductor'. Absent in MVP (Mode B substrate deferred; serde skip_serializing_if)
   */
  conductorEndpoint?: string;
}
/**
 * Human-readable authority reference surfaced by the portal trust-indicator. Derived from the issuing doorway's URL + id for doorway-host mode
 */
export interface AuthorityRef {
  /**
   * Human-readable label rendered by the trust-indicator chip
   */
  label: string;
  /**
   * Optional stable identifier (doorway_id slug or conductor peer_id). Absent when unknown (serde skip_serializing_if)
   */
  id?: string;
}
