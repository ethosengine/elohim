-- P2P Coherence: Retroactive classification for pre-refactor tables
-- These tables were created before the 4-sprint P2P coherence refactor
-- and need source-of-truth documentation + schema alignment.

-- imagodei_observations: Source of truth: private source chain (agent-scoped in imagodei DNA)
-- Classification: B2 (Agent-Scoped + Attestation) — raw observations are private to the
-- observing elohim context, but aggregate trust effects (trust_delta) produce public
-- attestations when thresholds are crossed. Same pattern as votes/governance_signals.
-- dht_anchor_hash populated when trust attestation is issued.
ALTER TABLE imagodei_observations ADD COLUMN dht_anchor_hash TEXT;

-- schedules: Source of truth: private source chain (agent-scoped, future imagodei DNA)
-- Classification: B (Agent-Scoped) — personal scheduling data (RRULE patterns for when
-- to study/practice). Not shared with peers, not reconstructable from other data.
-- No dht_anchor_hash needed — B entities have no public DHT entry to link to.

-- local_sessions: Source of truth: device-local (ephemeral session state)
-- Classification: C (Operational) — reconstructable from identity handoff.
-- No dht_anchor_hash needed.
