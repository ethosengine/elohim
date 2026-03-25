-- Drop path-specific tables (children first for foreign key ordering).
-- Paths are now ContentNodes with contentType 'path'. The parallel
-- path type system is eliminated.

DROP TABLE IF EXISTS path_extensions;
DROP TABLE IF EXISTS path_attestations;
DROP TABLE IF EXISTS steps;
DROP TABLE IF EXISTS chapters;
DROP TABLE IF EXISTS path_tags;
DROP TABLE IF EXISTS paths;
