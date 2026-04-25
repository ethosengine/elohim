-- Revert A.7 verify-stamp columns from epr_atoms.
-- SQLite 3.35+ supports DROP COLUMN (released 2021-03-12).

DROP INDEX IF EXISTS idx_epr_atoms_signer_cid_issued_at;
ALTER TABLE epr_atoms DROP COLUMN verified_signer_fingerprint;
ALTER TABLE epr_atoms DROP COLUMN verified_at;
