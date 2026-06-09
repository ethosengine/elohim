-- Source of truth: DHT (EconomicEvent notarized entry). This column is a write-through projection.
ALTER TABLE economic_events ADD COLUMN substrate_signal TEXT NULL;
