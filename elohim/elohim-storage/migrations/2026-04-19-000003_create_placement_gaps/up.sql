-- Source of truth: local (operational Category C).
-- Rebuilt from shard_locations + rea_commitments + humans.household_id at startup.
-- NO dht_anchor_hash: this is derivable, not notarized.
--
-- gap_kind values for Plan 1: 'under-committed', 'contracts-short',
-- 'peers-unavailable'. Plans 3-4 add 'unrecoverable', 'attested-breach'.
CREATE TABLE IF NOT EXISTS placement_gaps (
    id                          TEXT PRIMARY KEY NOT NULL,
    content_id                  TEXT NOT NULL,
    shard_hash                  TEXT NOT NULL,
    h_app_id                    TEXT NOT NULL,
    requested_household_count   INTEGER NOT NULL,
    achieved_household_count    INTEGER NOT NULL,
    contract_coverage           REAL NOT NULL,
    gap_kind                    TEXT NOT NULL,
    first_seen_at               TEXT NOT NULL,
    last_seen_at                TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_placement_gaps_unique
    ON placement_gaps(content_id, shard_hash, h_app_id, gap_kind);

CREATE INDEX IF NOT EXISTS idx_placement_gaps_content
    ON placement_gaps(content_id);

CREATE INDEX IF NOT EXISTS idx_placement_gaps_kind
    ON placement_gaps(gap_kind);

CREATE INDEX IF NOT EXISTS idx_placement_gaps_last_seen
    ON placement_gaps(last_seen_at);
