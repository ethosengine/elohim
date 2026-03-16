-- P2P Coherence Sprint 2: Lamad (Content) Provenance
-- Adds dht_anchor_hash to unanchored content pillar tables.
-- DNA CLEANUP NOTE: PathChapter and PathStep exist as standalone entry types in lamad DNA (2 of 83).
-- They may be collapsible into Link metadata on LearningPath. Audit deferred to dedicated cleanup sprint.

-- content: Source of truth: DHT (Content entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Content ActionHash
ALTER TABLE content ADD COLUMN dht_anchor_hash TEXT;

-- paths: Source of truth: DHT (LearningPath entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to LearningPath ActionHash
ALTER TABLE paths ADD COLUMN dht_anchor_hash TEXT;

-- chapters: Source of truth: DHT (derived from LearningPath via Link)
-- Classification: A2 (Derived) — dht_anchor_hash links to parent LearningPath ActionHash
ALTER TABLE chapters ADD COLUMN dht_anchor_hash TEXT;

-- steps: Source of truth: DHT (derived from LearningPath via chapter chain Link)
-- Classification: A2 (Derived) — dht_anchor_hash links to parent LearningPath ActionHash
ALTER TABLE steps ADD COLUMN dht_anchor_hash TEXT;

-- relationships: Source of truth: DHT (Relationship entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Relationship ActionHash
ALTER TABLE relationships ADD COLUMN dht_anchor_hash TEXT;

-- content_mastery: Source of truth: private source chain (agent-scoped)
-- Classification: B2 (Agent-Scoped + Attestation) — raw mastery is private,
-- dht_anchor_hash populated only when mastery crosses threshold and Attestation is issued
ALTER TABLE content_mastery ADD COLUMN dht_anchor_hash TEXT;

-- content_attestations: Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Attestation ActionHash
ALTER TABLE content_attestations ADD COLUMN dht_anchor_hash TEXT;

-- knowledge_maps: Source of truth: SQLite (operational)
-- Classification: C (Operational) — personal sensemaking, reconstructable from content relationships
-- No dht_anchor_hash needed.
