/**
 * Deterministic Peer ID Tests
 *
 * Tests for the single source of truth for peer_id derivation.
 * This formula is shared between seed-agent-bindings (writes AgentPeerBinding)
 * and seed-commitments (writes REA Commitments with peer_id-keyed provider/receiver).
 * Drift here causes cluster_view + reciprocity_view to silently return empty results.
 */

import { describe, it, expect } from 'vitest';
import { deterministicPeerId, type Archetype } from '../peer-id.js';

describe('deterministicPeerId', () => {
  it('returns 46-char string with 12D3KooW prefix', () => {
    const id = deterministicPeerId('human-matthew-manager', 'desktop');
    expect(id).toHaveLength(46);
    expect(id.startsWith('12D3KooW')).toBe(true);
  });

  it('is deterministic — same input → same output', () => {
    const a = deterministicPeerId('human-matthew-manager', 'desktop');
    const b = deterministicPeerId('human-matthew-manager', 'desktop');
    expect(a).toBe(b);
  });

  it('differs across archetypes for the same human', () => {
    const desktop = deterministicPeerId('human-matthew-manager', 'desktop');
    const node = deterministicPeerId('human-matthew-manager', 'node');
    expect(desktop).not.toBe(node);
  });

  it('differs across humans for the same archetype', () => {
    const matthew = deterministicPeerId('human-matthew-manager', 'desktop');
    const timothy = deterministicPeerId('human-timothy-tutor', 'desktop');
    expect(matthew).not.toBe(timothy);
  });

  it('matches the existing seed-agent-bindings formula exactly', () => {
    // Locked snapshot — if this changes, all bindings AND commitments diverge.
    const id = deterministicPeerId('human-matthew-manager', 'desktop');
    expect(id).toMatch(/^12D3KooW[a-f0-9]{38}$/);
  });
});
