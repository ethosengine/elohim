/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/exchange-session-response.schema.json -- DO NOT EDIT */

/**
 * Source of truth: doorway operational session state (Operational, Category C). GET /auth/exchange-session exchanges a single-use transfer token for a full JWT session; reconstructed per request, never persisted as substrate truth. Rust wire authority: doorway/doorway-service/src/routes/auth_routes.rs ExchangeSessionResponse (validated by that crate's tests/schema_contract.rs).
 */
export interface ExchangeSessionResponse {
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
   * First reachable portal host URL for this human, when the human is a steward and at least one registered host responds to a health probe. Absent when not a steward or no host is reachable (serde skip_serializing_if)
   */
  portalHostUrl?: string;
}
