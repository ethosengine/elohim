-- Reverting drops the election tiebreak; rows keep tier and clock ordering.
ALTER TABLE content DROP COLUMN canonical_link_hash;
