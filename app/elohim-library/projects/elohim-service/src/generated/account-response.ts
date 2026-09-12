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
   * RFC3339 UTC timestamp (seconds precision) when stewardship was confirmed — stamped through routes::hosted_cell::rfc3339_utc_secs, the same helper hostedCellValidUntil uses, so one response never speaks two dialects of time. NOT bson::DateTime's Display, which is the time crate's own format and makes Angular's DatePipe throw NG02100. Absent when not a steward (serde skip_serializing_if)
   */
  stewardshipAt?: string;
  /**
   * Whether the custodial key bundle has been exported by the human
   */
  keyExported: boolean;
  /**
   * RFC3339 UTC timestamp (seconds precision) when the account was created — see stewardshipAt for why the format is pinned. Absent when unknown (serde skip_serializing_if)
   */
  createdAt?: string;
  /**
   * RFC3339 UTC timestamp (seconds precision) of the last login — see stewardshipAt for why the format is pinned. Absent when never logged in (serde skip_serializing_if)
   */
  lastLoginAt?: string;
  /**
   * The name this human registered under — a doorway-local projection of their Human profile on the DHT, so an account page can greet a person rather than an identifier. Absent on rows that predate the field (serde skip_serializing_if)
   */
  displayName?: string;
  /**
   * CID (entry hash) of the live hosted-cell delegates-compute commitment this doorway's pool peer notarized for the human — readable back from ANY peer projecting the substrate, which is the point: the promise is checkable by someone other than the doorway reporting it. Absent when no promise was recorded (serde skip_serializing_if)
   */
  hostedCellGrantCid?: string;
  /**
   * When the hosting is promised until (RFC3339 UTC, seconds precision). Absent when no promise was recorded (serde skip_serializing_if)
   */
  hostedCellValidUntil?: string;
  /**
   * The DISPLAY NAME of the steward of the pool peer that notarized this person's hosted cell — the household that is actually lending the machine, in the words a person uses. Resolved at read time from A-class facts: hostedCellGrantCid (the promise exists) plus the provider the POOL PEER named itself by on its own grant answer (elohim-storage api/compute_grants.rs writes it as that peer's own conductor cell key, and the grant surface refuses any other performer), resolved through imagodei get_human_by_agent_key to that Human's displayName. Absent when any link is missing. It is NEVER the doorway's own name, id or gateway hostname: the doorway ARRANGES hosting and a household PERFORMS it, and naming the arranger here is exactly what genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature forbids. It is never the commitment cid or the conductor id either — those name an address and a machine. Travels with hostedCellGrantCid or not at all (serde skip_serializing_if)
   */
  hostedByHousehold?: string;
}
