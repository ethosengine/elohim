-- P2P Coherence Sprint 1: Shefa Provenance
-- Adds dht_anchor_hash to unanchored shefa tables.
-- Each table's source of truth is documented inline.

-- stewardship_allocations: Source of truth: DHT (derived from Agreement via Link)
-- Classification: A2 (Derived) — anchored via parent Agreement's ActionHash
ALTER TABLE stewardship_allocations ADD COLUMN dht_anchor_hash TEXT;

-- steward_credentials: Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — maps to Attestation with type=credential
ALTER TABLE steward_credentials ADD COLUMN dht_anchor_hash TEXT;

-- premium_gates: Source of truth: DHT (Link on Content entry in lamad DNA)
-- Classification: A (Notarized) — anchored via parent Content's ActionHash
ALTER TABLE premium_gates ADD COLUMN dht_anchor_hash TEXT;

-- access_grants: Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — maps to Attestation with type=access
ALTER TABLE access_grants ADD COLUMN dht_anchor_hash TEXT;

-- steward_affinity: Source of truth: SQLite (operational)
-- Classification: C (Operational) — reconstructable from economic_events curation acts
-- Reconstruction: re-derive from economic_events WHERE action='curate' grouped by steward+content
-- No dht_anchor_hash needed.

-- Source-of-truth comments for already-anchored tables:
-- economic_events: Source of truth: DHT (EconomicEvent in lamad DNA). dht_anchor_hash added in migration 2026-03-10-100000.
-- rea_commitments: Source of truth: DHT (Commitment in lamad DNA). dht_anchor_hash added in migration 2026-03-10-100000.
-- agreements: Source of truth: DHT (Agreement in lamad DNA). dht_anchor_hash present since table creation.
-- stewarded_nodes: Source of truth: DHT (StewardedResource in lamad DNA). dht_anchor_hash present since table creation.
-- node_stewardship: Source of truth: SQLite (operational). Derived from stewarded_nodes relationships. No dht_anchor_hash needed.
