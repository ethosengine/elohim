-- T16 — placement-gap action convention.
--
-- Emitted by the custody reconciliation controller when a custody-blob
-- commitment goes unhonored beyond grace. The action joins
-- project-blob / serve-blob / custody-blob (T03d) as the operational REA
-- vocabulary. It is an observation event (lives in economic_events), not
-- a commitment.
--
-- Convention:
--   action='placement-gap'
--   provider=<custodian-cid>           — the peer expected to host
--   receiver=<content-steward-cid>     — the peer who holds the commitment
--   resource_inventoried_as=<blob_hash>
--   output_of=<custody-blob commitment action_hash>  — links the gap to its commitment
--
-- The composite index from T03d (idx_economic_events_action_resource) already
-- covers this query pattern; this migration adds an output_of index for the
-- "all gaps for commitment X" query.

CREATE INDEX IF NOT EXISTS idx_economic_events_output_of
    ON economic_events(output_of);
