/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: p2p/attention-tending.schema.json -- DO NOT EDIT */

/**
 * Source of truth: peer-private discernment signal per brainstorm §6.1. The user tends the shape of their attention: a TTL-bounded classification of a filter pattern (content kind, topic, author scope, etc.) that feeds the elohim's tender conversation and the collective's anonymous wisdom layer. Peer-private by default — never gossiped; stays on the agent's source chain. T5 lands the Visibility::Private HDI integrity entry; T15 lands the tending lifecycle service.
 */
export interface AttentionTending {
  /**
   * Pattern to tend: content kind, topic, author scope, etc. Schema is a stub for T2; T15 may extend.
   */
  filterSubject: {
    [k: string]: unknown;
  };
  /**
   * Graduated tending classification. values-forward: human values-aligned discernment — 30-day TTL. fatigue: attention/bandwidth exhaustion — 7-day TTL. scope-mismatch: content outside current context — 90-day TTL. safety: constitutional-floor concern — no expiry (enforced by tending lifecycle service at T15).
   */
  classification: 'values-forward' | 'fatigue' | 'scope-mismatch' | 'safety';
  /**
   * Optional human-readable reason.
   */
  reason?: string;
  /**
   * TTL in seconds. Minimum 1 hour to prevent permanent moats; classification-default TTLs (values-forward=30d, fatigue=7d, scope-mismatch=90d, safety=no-expiry) enforced by the lifecycle service at T15.
   */
  ttlSeconds: number;
  /**
   * Unix timestamps of tending events; the earliest is creation, later entries are re-tending. Non-empty required: an AttentionTending with no events is meaningless.
   *
   * @minItems 1
   */
  tendedAt: [number, ...number[]];
  /**
   * Scope where this tending applies: collective, mode, time-of-day, etc. Schema is a stub for T2; T15 may extend.
   */
  context: {
    [k: string]: unknown;
  };
  /**
   * Base64-encoded ed25519 public key of the tending agent.
   */
  signedBy: string;
  /**
   * Base64-encoded ed25519 signature over canonical_bytes(filterSubject || classification || ttlSeconds || tendedAt || context || signedBy).
   */
  signature: string;
}
