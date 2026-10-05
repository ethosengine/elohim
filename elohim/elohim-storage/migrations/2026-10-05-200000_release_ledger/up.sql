-- Source of truth: local (operational, Category C). What this peer's release
-- adoption controller has verified, in the order it first saw each release.
--
-- A peer's content row for a channel holds only the channel's current head, so
-- nothing else on a peer can name "the releases before this one". The retention
-- pass reads this ledger to find the latest N releases of a channel and lets
-- the bytes of older ones go.
--
-- Not rebuilt from peers: a lost ledger starts again from the next release this
-- peer verifies. Bytes of releases the ledger does not name are never touched,
-- so a lost row keeps bytes; it cannot release them.
CREATE TABLE release_ledger (
    -- First-seen order on this peer. "Latest" is the highest seq of a channel.
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id TEXT NOT NULL,
    -- The release version's action hash, as the adoption controller names it.
    release_cid TEXT NOT NULL,
    artifact_class TEXT NOT NULL,
    -- [{"sha256": hex, "blobCid": "bafkrei…", "bytes": n, "filename": "…"}]
    artifacts_json TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    UNIQUE (channel_id, release_cid)
);
