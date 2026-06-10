/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/account-response.schema.json -- DO NOT EDIT */

/**
 * Source of truth: doorway operational account record (Operational, Category C) — the doorway user store (UserDoc) joined with per-account usage counters (storage, projection queries, bandwidth) and hosting/stewardship state; reconstructed per request, never persisted as a new entity. Distinct from epr:schema:view:account-view (the DHT-derived account-management snapshot). Rust wire authority: doorway/doorway-service/src/routes/auth_routes.rs AccountResponse (validated by that crate's tests/schema_contract.rs).
 */
export interface AccountResponse {
  /**
   * Holochain human ID for the account
   */
  humanId: string;
  /**
   * Login identifier (e.g. email) of the account
   */
  identifier: string;
  /**
   * Doorway permission level — Display form of doorway's PermissionLevel enum: 'PUBLIC' | 'AUTHENTICATED' | 'ADMIN'
   */
  permissionLevel: string;
  /**
   * Bytes of hosted storage currently used by the account
   */
  storageBytes: number;
  /**
   * Hosted storage quota in bytes
   */
  storageLimit: number;
  /**
   * storageBytes as a percentage of storageLimit
   */
  storagePercent: number;
  /**
   * Projection queries used today
   */
  projectionQueries: number;
  /**
   * Daily projection-query quota
   */
  dailyQueryLimit: number;
  /**
   * projectionQueries as a percentage of dailyQueryLimit
   */
  queriesPercent: number;
  /**
   * Bandwidth bytes used today
   */
  bandwidthBytes: number;
  /**
   * Daily bandwidth quota in bytes
   */
  dailyBandwidthLimit: number;
  /**
   * bandwidthBytes as a percentage of dailyBandwidthLimit
   */
  bandwidthPercent: number;
  /**
   * Conductor hosting this account's cell, when hosted. Absent otherwise (serde skip_serializing_if)
   */
  conductorId?: string;
  /**
   * True when the account's stewardship is substrate-confirmed. Always present (no skip on this struct)
   */
  isSteward: boolean;
  /**
   * ISO-8601 timestamp when stewardship was confirmed. Absent when not a steward (serde skip_serializing_if)
   */
  stewardshipAt?: string;
  /**
   * Whether the custodial key bundle has been exported by the human
   */
  keyExported: boolean;
  /**
   * ISO-8601 timestamp when the account was created. Absent when unknown (serde skip_serializing_if)
   */
  createdAt?: string;
  /**
   * ISO-8601 timestamp of the last login. Absent when never logged in (serde skip_serializing_if)
   */
  lastLoginAt?: string;
}
