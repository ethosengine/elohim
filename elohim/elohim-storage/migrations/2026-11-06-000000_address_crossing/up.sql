-- Re-genesis 1, the address crossing: every byte address in this store becomes a
-- CIDv1 raw + BLAKE3-256 (`bafkr4i…`, 59 characters, admitted by full decode).
--
-- Clean break, no row migration: the household and alpha are recast, so a
-- column that held a `sha256-<hex>` spelling, a bare hex digest or a raw+sha2
-- CID is dropped and recreated under its `*_cid` name, never rewritten. No
-- table keeps an old-profile column beside the new one (no dual read).

-- Source of truth: DHT (`Content` entry, elohim DNA). A read projection; only
-- the address columns cross. `blob_hash` is gone, `blob_cid` is the byte CID,
-- `server_blob_hash` becomes `server_blob_cid`. Addresses minted under the old
-- profile are cleared rather than carried.
DROP INDEX IF EXISTS idx_content_blob_hash;
ALTER TABLE content DROP COLUMN blob_hash;
ALTER TABLE content RENAME COLUMN server_blob_hash TO server_blob_cid;
UPDATE content SET blob_cid = NULL, server_blob_cid = NULL;
CREATE INDEX idx_content_blob_cid ON content(blob_cid);

-- Source of truth: local (operational, Category C). Which peers hold which
-- shard, keyed by the shard's byte CID. Rebuilt from shard acks and custody
-- announcements.
DROP TABLE IF EXISTS shard_locations;
CREATE TABLE shard_locations (
    shard_cid     TEXT NOT NULL,
    peer_id       TEXT NOT NULL,
    h_app_id      TEXT NOT NULL DEFAULT 'lamad',
    status        TEXT NOT NULL DEFAULT 'announced',
    first_seen    TEXT NOT NULL DEFAULT (datetime('now')),
    last_verified TEXT,
    PRIMARY KEY (shard_cid, peer_id)
);
CREATE INDEX idx_shard_locations_peer   ON shard_locations(peer_id);
CREATE INDEX idx_shard_locations_status ON shard_locations(status);

-- Source of truth: local (operational, Category C), the projection of the DNA
-- `ShardManifest` shape per content item. Every manifest records the
-- parameters it was minted under (`profile_json` = {chunker, erasure}); readers
-- use the recorded parameters, never this node's configuration.
--   chunks_json  = [{cid, len}]                       (the FastCDC chunks)
--   stripes_json = [{index, byte_len, shards[{cid, len, index}]}]
--                  (empty when the erasure id is "none")
DROP TABLE IF EXISTS shard_manifests;
CREATE TABLE shard_manifests (
    content_id    TEXT NOT NULL,
    h_app_id      TEXT NOT NULL DEFAULT 'lamad',
    blob_cid      TEXT NOT NULL,
    byte_len      BIGINT NOT NULL,
    mime_type     TEXT NOT NULL DEFAULT 'application/octet-stream',
    profile_json  TEXT NOT NULL,
    chunks_json   TEXT NOT NULL,
    stripes_json  TEXT NOT NULL DEFAULT '[]',
    reach         TEXT NOT NULL DEFAULT 'commons',
    author_id     TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    verified_at   TEXT,
    PRIMARY KEY (content_id, h_app_id)
);
CREATE INDEX idx_shard_manifests_blob ON shard_manifests(blob_cid);

-- Source of truth: local (operational, Category C). Placement gaps per custody
-- unit, keyed by the unit's byte CID.
DROP TABLE IF EXISTS placement_gaps;
CREATE TABLE placement_gaps (
    id                          TEXT PRIMARY KEY NOT NULL,
    content_id                  TEXT NOT NULL,
    shard_cid                   TEXT NOT NULL,
    h_app_id                    TEXT NOT NULL,
    requested_steward_count     INTEGER NOT NULL,
    achieved_steward_count      INTEGER NOT NULL,
    contract_coverage           REAL NOT NULL,
    gap_kind                    TEXT NOT NULL,
    first_seen_at               TEXT NOT NULL,
    last_seen_at                TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_placement_gaps_unique
    ON placement_gaps(content_id, shard_cid, h_app_id, gap_kind);
CREATE INDEX idx_placement_gaps_content   ON placement_gaps(content_id);
CREATE INDEX idx_placement_gaps_kind      ON placement_gaps(gap_kind);
CREATE INDEX idx_placement_gaps_last_seen ON placement_gaps(last_seen_at);

-- Source of truth: signed inventory envelopes on `elohim/inventory/blob/2` and
-- negentropy-v1 recon (operational, Category C). Which byte CIDs each peer
-- testified to holding. The `blake3_hash` alias column is gone: the byte CID
-- is the BLAKE3 address. Snapshots are gone, so `gossip-snapshot` is not a
-- source; `recon` is. `transport_affinity` is gone with the per-request iroh/libp2p
-- blob router: there is one byte path (the blob store, served over libp2p).
DROP TABLE IF EXISTS peer_blob_inventory;
CREATE TABLE peer_blob_inventory (
    peer_id            TEXT NOT NULL,
    blob_cid           TEXT NOT NULL,
    last_seen_at       TEXT NOT NULL,
    source             TEXT NOT NULL CHECK (source IN ('gossip-delta', 'recon', 'fetch-success', 'self-custody')),
    sequence           INTEGER NOT NULL,
    PRIMARY KEY (peer_id, blob_cid)
);
CREATE INDEX idx_peer_blob_inventory_blob   ON peer_blob_inventory(blob_cid);
CREATE INDEX idx_peer_blob_inventory_recent ON peer_blob_inventory(last_seen_at);

-- Source of truth: local (operational, Category C). The per-peer delta cursor.
-- `last_content_hash` deduplicated snapshots, which no longer exist; a cursor
-- re-bases only after a completed recon.
DROP TABLE IF EXISTS peer_inventory_cursor;
CREATE TABLE peer_inventory_cursor (
    peer_id        TEXT NOT NULL PRIMARY KEY,
    last_sequence  INTEGER NOT NULL,
    last_updated   TEXT NOT NULL
);

-- Source of truth: local (operational, Category C). Inventory envelopes under
-- a scheme or epoch this build does not implement: retained opaque, bounded
-- (the dispatcher evicts the oldest beyond `HELD_ENVELOPE_CAP`), never
-- projected into `peer_blob_inventory` and never counted toward custody.
CREATE TABLE held_envelopes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    topic       TEXT NOT NULL,
    reason      TEXT NOT NULL,
    bytes       BLOB NOT NULL,
    received_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Source of truth: local (T22 stewardship-rotation write target). The share's
-- integrity address is the byte CID of `share_data`.
DROP TABLE IF EXISTS custodian_shares;
CREATE TABLE custodian_shares (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    governance_action_cid TEXT    NOT NULL,
    custodian_cid         TEXT    NOT NULL,
    share_index           INTEGER NOT NULL,   -- 1-based index in the (m,n) scheme
    share_data            BLOB    NOT NULL,   -- raw share bytes
    share_blob_cid        TEXT    NOT NULL,   -- byte CID of share_data; checked on read
    created_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    superseded_at         TEXT                -- set when a rotation supersedes this share
);
CREATE INDEX idx_custodian_shares_gov_action_custodian
    ON custodian_shares(governance_action_cid, custodian_cid);
CREATE INDEX idx_custodian_shares_custodian
    ON custodian_shares(custodian_cid);

-- Source of truth: local (operational, Category C). How each blob in this
-- peer's blob store got here, keyed by its byte CID. A lost row is fail-safe:
-- a blob with no record is never let go by the retention pass.
DROP TABLE IF EXISTS blob_arrivals;
CREATE TABLE blob_arrivals (
    blob_cid     TEXT NOT NULL,
    -- self-put | self-fetch | adoption | placed
    arrived_via  TEXT NOT NULL,
    -- For `placed`: the agent that placed it, or its transport label. Empty for
    -- this peer's own acts.
    placed_by    TEXT NOT NULL DEFAULT '',
    -- What it arrived for, when the path knows: a release CID for `adoption`.
    named_for    TEXT,
    arrived_at   TEXT NOT NULL,
    withdrawn_at TEXT,
    PRIMARY KEY (blob_cid, arrived_via, placed_by)
);

-- Source of truth: local (operational, Category C). Blobs the retention pass
-- reads as this peer's own and named by nothing, with the consecutive-pass
-- count. A lost row only delays letting a blob go.
DROP TABLE IF EXISTS blob_unnamed_watch;
CREATE TABLE blob_unnamed_watch (
    blob_cid         TEXT PRIMARY KEY NOT NULL,
    first_unnamed_at TEXT NOT NULL,
    passes           INTEGER NOT NULL DEFAULT 0
);

-- Source of truth: DHT (EconomicEvent, action='ack-projection'), rebuildable
-- from the rea_projection signal stream. Keyed by the projected blob's byte CID.
DROP TABLE IF EXISTS projection_events;
CREATE TABLE projection_events (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    blob_cid            TEXT NOT NULL,
    projector_agent_cid TEXT NOT NULL,
    emitted_at          TEXT NOT NULL,
    source_action_hash  TEXT NOT NULL UNIQUE  -- dedup by source DHT action
);
CREATE INDEX idx_projection_events_blob_emitted
    ON projection_events (blob_cid, emitted_at DESC);

-- Source of truth: DHT (Commitment entry). No DDL change: from this epoch
-- `rea_commitments.resource_classified_as` names a custody commitment's
-- resource by byte CID only (the writers mint it with `compute_bytes_cid`).
-- No row is rewritten; the household and alpha are recast.
