-- Reverting drops the liveness verdict. Rows keep `dht_anchor_hash`; they
-- simply lose the ability to say "I cannot prove this any more", and the heal
-- loop returns to selecting NULL anchors only.
ALTER TABLE content DROP COLUMN dht_anchor_checked_at;
ALTER TABLE content DROP COLUMN dht_anchor_state;
