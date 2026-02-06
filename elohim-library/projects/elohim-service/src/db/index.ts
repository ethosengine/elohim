/**
 * Kuzu Database Module
 *
 * Provides embedded graph database functionality for the Elohim Protocol.
 * This module is the foundation for:
 * - CLI CRUD operations on paths and content
 * - Future Angular WASM integration
 * - Migration path to Holochain
 */

export { initializeSchema, getSchemaStats, dropAllTables, SCHEMA_DDL, RELATIONSHIP_TYPE_MAP } from './kuzu-schema';
export { KuzuClient } from './kuzu-client';
// Note: ContentNode, LearningPath interfaces are defined in kuzu-client but also exist in models
// The kuzu-client versions are specific to the DB layer, models versions are the canonical ones
