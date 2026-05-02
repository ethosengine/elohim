-- T03d — REA action conventions for blob projection contracts.
--
-- Blob distribution facts (replicas, projector acks, custody) are NOT stored
-- in dedicated tables. They live in the existing REA tables (rea_commitments,
-- economic_events) using these action conventions:
--   project-blob (commitment) — doorway-steward commits to project a blob
--   serve-blob   (event)      — doorway-steward fulfills the projection
--   custody-blob (commitment) — peer steward commits N bytes of custody
--
-- Note: extends the existing single-column action indexes
-- (idx_rea_commitment_action, idx_event_action) with resource-column
-- coverage for the (action, blob_hash) query pattern. Not redundant —
-- SQLite cannot satisfy the composite filter from the single-column index.
--
-- Index choice:
--   - economic_events has `resource_inventoried_as` (the blob_hash for serve-blob).
--   - rea_commitments has `resource_classified_as` (the blob_hash for
--     project-blob and custody-blob); it does NOT have resource_inventoried_as.
--   See the Components section of the topology design spec for the convention.

CREATE INDEX idx_rea_commitments_action_resource
    ON rea_commitments(action, resource_classified_as);

CREATE INDEX idx_economic_events_action_resource
    ON economic_events(action, resource_inventoried_as);
