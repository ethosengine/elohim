-- EPR Phase 2B — A.7: resolver-backed Ed25519 verify stamp columns
-- Source of truth: EPR atoms (self-notarized via content-derived CID + Ed25519).
-- These two columns are stamped on ingest when signature verification succeeds
-- against the signer's pubkey timeline. NULL means "not yet verified" (atoms
-- ingested before A.7 landed, or atoms whose signer has no timeline entry yet).
--
-- verified_at         — UTC timestamp at which resolver-backed verify ran
-- verified_signer_fingerprint — blake3-128-prefix of the 32-byte ed25519 pubkey
--                               that successfully signed this atom
--                               (first 16 bytes = 32 hex chars)
--
-- The (signer_cid, issued_at) index enables A.8's revocation sweep to efficiently
-- locate atoms signed by a given agent during a specific time window.

ALTER TABLE epr_atoms ADD COLUMN verified_at TEXT;
ALTER TABLE epr_atoms ADD COLUMN verified_signer_fingerprint TEXT;
CREATE INDEX idx_epr_atoms_signer_cid_issued_at ON epr_atoms(signer_cid, issued_at);
