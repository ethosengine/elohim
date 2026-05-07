/**
 * Deterministic Peer ID Derivation — Single Source of Truth
 *
 * The peer_id derivation formula is the load-bearing contract between:
 * - seed-agent-bindings: writes peer_identity_bindings DHT entries
 * - seed-commitments: writes rea_commitments with peer_id-keyed provider/receiver
 *
 * Drift here causes cluster_view + reciprocity_view SQL filters to silently
 * return empty results. Extract to a single shared utility to prevent divergence.
 *
 * **Stage 1 (current)**: Deterministic test-only peer_id. Not a valid libp2p
 * PeerId (suffix is hex, not base58btc) — opaque string at the projection layer.
 *
 * **Stage 2 (future)**: Generate keypair-derived IDs that round-trip through
 * `PeerId::from_str` and are valid libp2p identifiers.
 */

import { createHash } from 'node:crypto';

export type Archetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Deterministic test-only peer_id. Stage 1: not a valid libp2p PeerId
 * (suffix is hex, not base58btc) — opaque string at the projection layer.
 *
 * Same (humanId, archetype) → same peer_id across re-runs. The 12D3KooW prefix
 * mimics libp2p shape for ergonomic logs only.
 *
 * **Single source of truth for the peer_id formula.** Both seed-agent-bindings
 * and seed-commitments must import this — drift means SQL view predicates
 * silently return empty.
 */
export function deterministicPeerId(humanId: string, archetype: Archetype): string {
  const digest = createHash('sha256')
    .update(`${humanId}:${archetype}`, 'utf8')
    .digest();
  // Lowercase hex suffix — deterministic, alphanumeric, and clearly not a
  // real libp2p PeerId. Stage 1 stores peer_id as an opaque string; Stage 2
  // will generate keypair-derived IDs that round-trip through PeerId::from_str.
  // Sliced to 38 chars to keep total length stable (12D3KooW + 38 = 46 chars).
  return `12D3KooW${digest.toString('hex').slice(0, 38)}`;
}
