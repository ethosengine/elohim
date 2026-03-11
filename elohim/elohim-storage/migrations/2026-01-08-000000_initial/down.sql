-- Rollback initial migration
-- WARNING: This will delete all data!

DROP TABLE IF EXISTS path_attestations;
DROP TABLE IF EXISTS steps;
DROP TABLE IF EXISTS chapters;
DROP TABLE IF EXISTS path_tags;
DROP TABLE IF EXISTS paths;
DROP TABLE IF EXISTS content_tags;
DROP TABLE IF EXISTS content;
DROP TABLE IF EXISTS apps;
DROP TABLE IF EXISTS schema_version;
