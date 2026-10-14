DROP INDEX idx_acquisition_pins_status_readmit_after;
ALTER TABLE acquisition_pins DROP COLUMN readmit_after;
ALTER TABLE acquisition_pins DROP COLUMN read_since_retired;
ALTER TABLE acquisition_pins DROP COLUMN round;
ALTER TABLE acquisition_pins DROP COLUMN retired_at;
ALTER TABLE acquisition_pins DROP COLUMN last_outcome;
ALTER TABLE acquisition_pins DROP COLUMN held_demand_count;
ALTER TABLE acquisition_pins DROP COLUMN consecutive_retirements;
