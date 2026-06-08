-- Slice 2b: link a DevicePin to its notarized provide shadow.
-- commitment_cid is the action_hash of the replicates-commons Commitment the
-- provide reconciler authors for this pin (nullable — a pin is born before its
-- shadow exists). The reconciler back-fills it after authoring; the revocation
-- arm (T10) reads it to target a revokes-commitment when the pin is un-pinned.
-- Source of truth for the commitment itself stays the DHT (mishpat_commitments
-- projection); this is a convenience back-reference, logical-key dedup remains
-- the authoritative author-once guard.
-- Spec: 2026-06-08-epr-acquisition-slice2b-provide-loop-design.md.
ALTER TABLE acquisition_pins ADD COLUMN commitment_cid TEXT;
