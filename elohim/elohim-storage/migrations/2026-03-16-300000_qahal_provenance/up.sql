-- P2P Coherence Sprint 4: Qahal (Governance) Provenance
-- Adds dht_anchor_hash to governance pillar tables.
-- Completes the 4-sprint P2P coherence refactor across all pillars.

-- proposals: Source of truth: DHT (governance entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to proposal ActionHash
ALTER TABLE proposals ADD COLUMN dht_anchor_hash TEXT;

-- proposal_options: Source of truth: DHT (derived from proposal via Link)
-- Classification: A2 (Derived) — dht_anchor_hash links to parent proposal ActionHash
ALTER TABLE proposal_options ADD COLUMN dht_anchor_hash TEXT;

-- votes: Source of truth: private source chain (agent-scoped ballot)
-- Classification: B2 (Agent-Scoped + Attestation) — raw vote is private,
-- dht_anchor_hash populated when tally Attestation is issued
ALTER TABLE votes ADD COLUMN dht_anchor_hash TEXT;

-- ranked_votes: Source of truth: private source chain (agent-scoped ballot)
-- Classification: B2 — same pattern as votes
ALTER TABLE ranked_votes ADD COLUMN dht_anchor_hash TEXT;

-- governance_signals: Source of truth: private source chain (agent-scoped reaction)
-- Classification: B2 — raw signal is private, aggregate Attestation is notarized
ALTER TABLE governance_signals ADD COLUMN dht_anchor_hash TEXT;

-- governance_states: Source of truth: DHT (derived from proposal lifecycle)
-- Classification: A2 (Derived) — dht_anchor_hash links to parent proposal ActionHash
ALTER TABLE governance_states ADD COLUMN dht_anchor_hash TEXT;

-- challenges: Source of truth: DHT (governance entry)
-- Classification: A (Notarized) — challenges are public acts, must be witnessed
ALTER TABLE challenges ADD COLUMN dht_anchor_hash TEXT;

-- appeals: Source of truth: DHT (governance entry)
-- Classification: A (Notarized) — appeals are public acts, must be witnessed
ALTER TABLE appeals ADD COLUMN dht_anchor_hash TEXT;

-- statements: Source of truth: DHT (Polis sensemaking entry)
-- Classification: A (Notarized) — community-visible statements
ALTER TABLE statements ADD COLUMN dht_anchor_hash TEXT;

-- statement_votes: Source of truth: private source chain (agent-scoped stance)
-- Classification: B2 — private stance, clustered aggregate is notarized
ALTER TABLE statement_votes ADD COLUMN dht_anchor_hash TEXT;

-- precedents: Source of truth: DHT (governance memory)
-- Classification: A (Notarized) — immutable governance precedents
ALTER TABLE precedents ADD COLUMN dht_anchor_hash TEXT;

-- discussions: Source of truth: SQLite (operational)
-- Classification: C (Operational) — reconstructable from thread references
-- No dht_anchor_hash needed.

-- comments: Source of truth: SQLite (operational)
-- Classification: C (Operational) — reconstructable from parent references
-- No dht_anchor_hash needed.
