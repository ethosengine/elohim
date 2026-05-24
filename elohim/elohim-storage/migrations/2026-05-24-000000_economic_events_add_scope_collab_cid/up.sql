-- M1 Task 14 deepening: record the Collab-Qahal scope under which an EconomicEvent
-- was authored. Downstream queries use this to correlate Settlement events with their
-- source Collab.
ALTER TABLE economic_events ADD COLUMN scope_collab_cid TEXT NULL;
CREATE INDEX IF NOT EXISTS idx_economic_events_scope_collab_cid
    ON economic_events(scope_collab_cid) WHERE scope_collab_cid IS NOT NULL;
