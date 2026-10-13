-- Re-admission backoff state for a retired acquisition pin.
-- Source of truth: local (agent-scoped device pin, Category B — same as the row).
--
-- A retired pin was re-admitted on a flat 6 h cooldown and spent its whole retry
-- budget again each time, forever (alpha matthew 2026-10-13: 1103 retired pins,
-- 1063 for `e2e-<uuid>` heads no peer holds, re-probed four times a day). The
-- hold now doubles per consecutive retirement, which needs the count to survive
-- a restart — an in-memory count would reset the ladder on every roll.
--
--   consecutive_retirements  retirements in a row with no fetch and no peer
--                            inventory naming the head between them. 0 = never
--                            retired, or the ladder was reset.
--   held_demand_count        read-misses recorded against the pin while it sat
--                            in `retired` or `removed` (demand does not undo
--                            either; it is counted). Cleared on re-admission.
--   last_outcome             'exhausted' | 'fetched' | NULL (no round concluded).
--
-- `updated_at` stays the retirement clock (acquisition::pin_readmit_at).
ALTER TABLE acquisition_pins ADD COLUMN consecutive_retirements INTEGER NOT NULL DEFAULT 0;
ALTER TABLE acquisition_pins ADD COLUMN held_demand_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE acquisition_pins ADD COLUMN last_outcome TEXT;

-- A row already `retired` has been retired at least once: start it on the first
-- rung (6 h from its `updated_at`, exactly the hold it is serving today) so its
-- next failed re-probe steps to 12 h instead of repeating 6 h.
UPDATE acquisition_pins
   SET consecutive_retirements = 1, last_outcome = 'exhausted'
 WHERE status = 'retired';
