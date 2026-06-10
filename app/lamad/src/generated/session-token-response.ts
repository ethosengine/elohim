/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/session-token-response.schema.json -- DO NOT EDIT */

/**
 * Source of truth: doorway in-memory session-transfer store (Operational, Category C). Short-lived (60 s TTL) single-use token minted by GET /auth/session-token for cross-app session handoff; never persisted. Rust wire authority: doorway/doorway-service/src/routes/auth_routes.rs SessionTokenResponse (validated by that crate's tests/schema_contract.rs).
 */
export interface SessionTokenResponse {
  /**
   * Short-lived single-use transfer token (60 s TTL)
   */
  sessionToken: string;
  /**
   * Unix timestamp (seconds) when the transfer token expires
   */
  expiresAt: number;
}
