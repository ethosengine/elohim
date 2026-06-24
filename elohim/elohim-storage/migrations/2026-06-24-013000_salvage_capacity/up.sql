-- Source of truth: salvage-capacity gossip (`SalvageCapacityAd` on topic
-- 'elohim/storage/salvage'). Category C operational projection — rebuildable
-- from gossip replay. Feeds the Phase-3 salvage candidate pool: opt-in,
-- always-on peers advertise spare capacity; `salvage_pass` ranks fresh,
-- opted-in entries via the placement strategy seam.
--
-- Identity namespace is `agent_cid` throughout (the canonical join key the
-- XOR placement metric never crosses). Manifest counterpart: the salvage
-- `custody-blob` commitment authored when this peer self-selects as a holder.
CREATE TABLE salvage_capacity (
    agent_cid TEXT NOT NULL PRIMARY KEY,
    spare_bytes INTEGER NOT NULL,
    archetype TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    seq INTEGER NOT NULL
);
CREATE INDEX idx_salvage_capacity_archetype ON salvage_capacity(archetype);
CREATE INDEX idx_salvage_capacity_last_seen ON salvage_capacity(last_seen_at);
