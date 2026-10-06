-- Source of truth: local (operational, Category C). How each blob in this
-- peer's blob store got here, written by the path that stored it.
--
-- The retention pass reads it to tell apart three things that look the same on
-- disk: bytes this peer brought here by its own act (it put them, fetched them,
-- or pulled them for a release), bytes another peer placed here, and bytes with
-- no record at all. Only the first kind may be let go when nothing names it.
-- Bytes another peer placed are that peer's to withdraw.
--
-- Not rebuilt from peers. A lost row is fail-safe: the blob then reads as
-- having no record, and a blob with no record is never let go by the pass.
CREATE TABLE blob_arrivals (
    -- The blob store's own spelling: sha256-<hex>.
    blob_hash TEXT NOT NULL,
    -- self-put | self-fetch | adoption | placed
    arrived_via TEXT NOT NULL,
    -- For `placed`: the agent that placed it, or its transport label when the
    -- agent could not be resolved. Empty for this peer's own acts.
    placed_by TEXT NOT NULL DEFAULT '',
    -- What it arrived for, when the path knows: a release CID for `adoption`.
    named_for TEXT,
    arrived_at TEXT NOT NULL,
    -- Set when the peer that placed it withdraws the placement.
    withdrawn_at TEXT,
    PRIMARY KEY (blob_hash, arrived_via, placed_by)
);

-- Source of truth: local (operational, Category C). One row per blob the
-- retention pass currently reads as this peer's own and named by nothing: when
-- it first read so, and on how many consecutive passes. The row is removed the
-- moment the blob is named again or let go, so a blob named between two passes
-- starts its count over. A lost row only delays letting a blob go.
CREATE TABLE blob_unnamed_watch (
    blob_hash TEXT PRIMARY KEY NOT NULL,
    first_unnamed_at TEXT NOT NULL,
    passes INTEGER NOT NULL DEFAULT 0
);
