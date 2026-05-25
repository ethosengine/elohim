/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/public-surface-state.schema.json -- DO NOT EDIT */

/**
 * DNS/TLS/reachability surface for a doorway hostname. Source of truth: external probes (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface PublicSurfaceState {
  dnsResolves: boolean;
  /**
   * Resolved A/AAAA target, when DNS resolves.
   */
  dnsTarget?: string;
  tlsValid: boolean;
  /**
   * Days until TLS cert expires (negative if already expired).
   */
  tlsExpiresInDays?: number;
  /**
   * True when an external probe can reach the doorway.
   */
  publicReachable: boolean;
}
