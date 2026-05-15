-- custodian_shares — per-custodian Shamir share store.
--
-- This table is a LOCAL OPERATIONAL projection (Category C).
-- Source of truth: the stewardship-rotation ceremony that installs shares
-- (T22 extern `create_shamir_custody_setup`). The share bytes here are what
-- the responder reads when it receives an authorized ShamirShareRequest.
--
-- ── At-rest threat model (deferred to post-M4 stewardship-rotation work) ──
-- Share bytes are stored unencrypted in this SQLite file. The threat model
-- accepts that a local attacker with read access to the database file could
-- extract individual shares. Because reconstruction requires `m` shares from
-- `m` distinct custodians, a single-custodian compromise does NOT reconstruct
-- the secret — the scheme's threshold property holds.
--
-- Post-M4 stewardship-rotation work should:
--   1. Add a `key_id` column referencing an HSM-backed or OS-keychain key.
--   2. Encrypt `share_data` with an AEAD key derived from the OS keychain.
--   3. The decryption key never lands in SQLite; only the ciphertext does.
-- Until then, database-file confidentiality relies on filesystem-level ACLs
-- and full-disk encryption on the hosting device.
CREATE TABLE custodian_shares (
    id                              INTEGER PRIMARY KEY AUTOINCREMENT,
    governance_action_cid           TEXT    NOT NULL,
    custodian_cid                   TEXT    NOT NULL,
    share_index                     INTEGER NOT NULL,   -- 1-based index in the (m,n) scheme
    share_data                      BLOB    NOT NULL,   -- raw share bytes (see at-rest threat model above)
    share_blob_hash                 TEXT    NOT NULL,   -- hex-encoded SHA-256 of share_data; integrity check on read
    created_at                      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    superseded_at                   TEXT                -- ISO 8601; non-null when this share is superseded
                                                        -- by a stewardship-rotation replacement
);

-- Fast lookup: "does this custodian hold a share for this governance action?"
-- The WHERE clause in get_share_for_governance_action filters superseded_at IS NULL.
CREATE INDEX idx_custodian_shares_gov_action_custodian
    ON custodian_shares(governance_action_cid, custodian_cid);

-- Fast listing: "all active shares held by this custodian"
-- Used by the stewardship-rotation cleanup sweep (T22+).
CREATE INDEX idx_custodian_shares_custodian
    ON custodian_shares(custodian_cid);
