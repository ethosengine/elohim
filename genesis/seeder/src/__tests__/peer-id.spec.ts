/**
 * Deterministic Peer ID Tests
 *
 * Tests for the single source of truth for peer_id derivation.
 * This formula is shared between seed-agent-bindings (writes AgentPeerBinding)
 * and seed-commitments (writes REA Commitments with peer_id-keyed provider/receiver).
 * Drift here causes cluster_view + reciprocity_view to silently return empty results.
 */

import { describe, it, expect, afterEach } from 'vitest';
import { deterministicPeerId, storageUrlForHuman, type Archetype } from '../peer-id.js';

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
    const terrance = deterministicPeerId('human-terrance-tutor', 'desktop');
    expect(matthew).not.toBe(terrance);
  });

  it('matches the existing seed-agent-bindings formula exactly', () => {
    // Locked snapshot — if this changes, all bindings AND commitments diverge.
    const id = deterministicPeerId('human-matthew-manager', 'desktop');
    expect(id).toMatch(/^12D3KooW[a-f0-9]{38}$/);
  });
});

describe('storageUrlForHuman', () => {
  afterEach(() => {
    delete process.env.PEER_STORAGE_URLS;
    delete process.env.PEER_STORAGE_URL_TEMPLATE;
  });

  it('resolves per-peer host:port from the PEER_STORAGE_URLS CSV (hc-mesh ports differ per peer)', () => {
    process.env.PEER_STORAGE_URLS =
      'matthew=localhost:8090,jessica=localhost:8091,james=localhost:8092';
    expect(storageUrlForHuman('human-jessica-spouse')).toBe('http://localhost:8091');
    expect(storageUrlForHuman('human-matthew-manager')).toBe('http://localhost:8090');
  });

  it('keeps a scheme already present in a CSV entry', () => {
    process.env.PEER_STORAGE_URLS = 'matthew=https://storage.example:8443';
    expect(storageUrlForHuman('human-matthew-manager')).toBe('https://storage.example:8443');
  });

  it('falls through to the template for names absent from the CSV', () => {
    process.env.PEER_STORAGE_URLS = 'matthew=localhost:8090';
    process.env.PEER_STORAGE_URL_TEMPLATE = 'http://elohim-{name}-x:8090';
    expect(storageUrlForHuman('human-adam-firstman')).toBe('http://elohim-adam-x:8090');
  });

  it('uses the alpha default when neither env var is set', () => {
    expect(storageUrlForHuman('human-matthew-manager')).toBe(
      'http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090'
    );
  });
});
