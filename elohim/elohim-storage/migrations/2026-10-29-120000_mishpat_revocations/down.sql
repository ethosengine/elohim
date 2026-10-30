-- Destroys projected tombstones and this peer's submission records. After a
-- down -> up, a revocation whose target had not landed is lost and a
-- submission whose outcome was unknown may be made once more.
DROP TABLE IF EXISTS mishpat_revocations;
