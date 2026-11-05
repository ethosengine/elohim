-- Reverse of the address crossing: the pre-crossing shapes, empty. Rows are not
-- carried in either direction (clean break).
DROP INDEX IF EXISTS idx_content_blob_cid;
ALTER TABLE content RENAME COLUMN server_blob_cid TO server_blob_hash;
ALTER TABLE content ADD COLUMN blob_hash TEXT;
CREATE INDEX idx_content_blob_hash ON content(blob_hash);

DROP TABLE IF EXISTS shard_locations;
CREATE TABLE shard_locations (
    shard_hash    TEXT NOT NULL,
    peer_id       TEXT NOT NULL,
    h_app_id      TEXT NOT NULL DEFAULT 'lamad',
    status        TEXT NOT NULL DEFAULT 'announced',
    first_seen    TEXT NOT NULL DEFAULT (datetime('now')),
    last_verified TEXT,
    PRIMARY KEY (shard_hash, peer_id)
);
CREATE INDEX idx_shard_locations_peer   ON shard_locations(peer_id);
CREATE INDEX idx_shard_locations_status ON shard_locations(status);

DROP TABLE IF EXISTS shard_manifests;
CREATE TABLE shard_manifests (
    content_id          TEXT NOT NULL,
    h_app_id            TEXT NOT NULL DEFAULT 'lamad',
    blob_hash           TEXT NOT NULL,
    blob_cid            TEXT,
    encoding            TEXT NOT NULL DEFAULT 'none',
    data_shard_count    INTEGER NOT NULL DEFAULT 1,
    parity_shard_count  INTEGER NOT NULL DEFAULT 0,
    shard_hashes_json   TEXT NOT NULL DEFAULT '[]',
    total_size_bytes    INTEGER NOT NULL,
    shard_size_bytes    INTEGER NOT NULL,
    mime_type           TEXT NOT NULL DEFAULT 'application/octet-stream',
    reach               TEXT NOT NULL DEFAULT 'commons',
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (content_id, h_app_id)
);
CREATE INDEX idx_shard_manifests_blob     ON shard_manifests(blob_hash);
CREATE INDEX idx_shard_manifests_encoding ON shard_manifests(encoding);

DROP TABLE IF EXISTS placement_gaps;
CREATE TABLE placement_gaps (
    id                          TEXT PRIMARY KEY NOT NULL,
    content_id                  TEXT NOT NULL,
    shard_hash                  TEXT NOT NULL,
    h_app_id                    TEXT NOT NULL,
    requested_steward_count     INTEGER NOT NULL,
    achieved_steward_count      INTEGER NOT NULL,
    contract_coverage           REAL NOT NULL,
    gap_kind                    TEXT NOT NULL,
    first_seen_at               TEXT NOT NULL,
    last_seen_at                TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_placement_gaps_unique
    ON placement_gaps(content_id, shard_hash, h_app_id, gap_kind);
CREATE INDEX idx_placement_gaps_content   ON placement_gaps(content_id);
CREATE INDEX idx_placement_gaps_kind      ON placement_gaps(gap_kind);
CREATE INDEX idx_placement_gaps_last_seen ON placement_gaps(last_seen_at);

DROP TABLE IF EXISTS peer_blob_inventory;
CREATE TABLE peer_blob_inventory (
    peer_id            TEXT NOT NULL,
    blob_hash          TEXT NOT NULL,
    last_seen_at       TEXT NOT NULL,
    source             TEXT NOT NULL CHECK (source IN ('gossip-snapshot', 'gossip-delta', 'fetch-success', 'self-custody')),
    sequence           INTEGER NOT NULL,
    blake3_hash        TEXT,
    transport_affinity TEXT NULL,
    PRIMARY KEY (peer_id, blob_hash)
);
CREATE INDEX idx_peer_blob_inventory_blob   ON peer_blob_inventory(blob_hash);
CREATE INDEX idx_peer_blob_inventory_recent ON peer_blob_inventory(last_seen_at);
CREATE INDEX idx_peer_blob_inventory_blake3 ON peer_blob_inventory(blake3_hash) WHERE blake3_hash IS NOT NULL;

DROP TABLE IF EXISTS peer_inventory_cursor;
CREATE TABLE peer_inventory_cursor (
    peer_id           TEXT NOT NULL PRIMARY KEY,
    last_sequence     INTEGER NOT NULL,
    last_updated      TEXT NOT NULL,
    last_content_hash TEXT
);

DROP TABLE IF EXISTS held_envelopes;

DROP TABLE IF EXISTS custodian_shares;
CREATE TABLE custodian_shares (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    governance_action_cid TEXT    NOT NULL,
    custodian_cid         TEXT    NOT NULL,
    share_index           INTEGER NOT NULL,
    share_data            BLOB    NOT NULL,
    share_blob_hash       TEXT    NOT NULL,
    created_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    superseded_at         TEXT
);
CREATE INDEX idx_custodian_shares_gov_action_custodian
    ON custodian_shares(governance_action_cid, custodian_cid);
CREATE INDEX idx_custodian_shares_custodian
    ON custodian_shares(custodian_cid);

DROP TABLE IF EXISTS blob_arrivals;
CREATE TABLE blob_arrivals (
    blob_hash    TEXT NOT NULL,
    arrived_via  TEXT NOT NULL,
    placed_by    TEXT NOT NULL DEFAULT '',
    named_for    TEXT,
    arrived_at   TEXT NOT NULL,
    withdrawn_at TEXT,
    PRIMARY KEY (blob_hash, arrived_via, placed_by)
);

DROP TABLE IF EXISTS blob_unnamed_watch;
CREATE TABLE blob_unnamed_watch (
    blob_hash        TEXT PRIMARY KEY NOT NULL,
    first_unnamed_at TEXT NOT NULL,
    passes           INTEGER NOT NULL DEFAULT 0
);

DROP TABLE IF EXISTS projection_events;
CREATE TABLE projection_events (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    blob_hash           TEXT NOT NULL,
    projector_agent_cid TEXT NOT NULL,
    emitted_at          TEXT NOT NULL,
    source_action_hash  TEXT NOT NULL UNIQUE
);
CREATE INDEX idx_projection_events_blob_emitted
    ON projection_events (blob_hash, emitted_at DESC);
