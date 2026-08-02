-- Reverting drops the election ordering projection. The rows keep their
-- `declared_head_action_hash`; they simply lose the ability to be moved forward
-- by a DHT-elected canonical answer (the pre-2026-08-02 behaviour).
ALTER TABLE content DROP COLUMN canonical_earned;
ALTER TABLE content DROP COLUMN canonical_declared_at;
