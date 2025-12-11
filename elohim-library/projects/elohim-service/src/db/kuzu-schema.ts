/**
 * Kuzu Graph Database Schema for Elohim Protocol
 *
 * This schema is designed to:
 * 1. Support the current JSON data model (ContentNode, LearningPath, etc.)
 * 2. Align with future Holochain entry types
 * 3. Enable rich Cypher graph queries
 *
 * Node tables map to Holochain entry types.
 * Relationship tables map to Holochain link types.
 */

// Using any type since kuzu doesn't have proper TypeScript types
type Connection = any;

/**
 * Schema definition for all node and relationship tables
 */
export const SCHEMA_DDL = {
  nodes: [
    // ContentNode - Core content unit (maps to content entry type)
    `CREATE NODE TABLE IF NOT EXISTS ContentNode (
      id STRING PRIMARY KEY,
      contentType STRING,
      title STRING,
      description STRING,
      content STRING,
      contentFormat STRING,
      tags STRING[],
      authorId STRING,
      reach STRING,
      trustScore DOUBLE DEFAULT 0.0,
      metadata STRING,
      sourcePath STRING,
      createdAt TIMESTAMP,
      updatedAt TIMESTAMP
    )`,

    // LearningPath - Curated journey (maps to path entry type)
    `CREATE NODE TABLE IF NOT EXISTS LearningPath (
      id STRING PRIMARY KEY,
      version STRING,
      title STRING,
      description STRING,
      purpose STRING,
      createdBy STRING,
      difficulty STRING,
      estimatedDuration STRING,
      visibility STRING,
      pathType STRING,
      tags STRING[],
      createdAt TIMESTAMP,
      updatedAt TIMESTAMP
    )`,

    // PathStep - Individual step in a path
    `CREATE NODE TABLE IF NOT EXISTS PathStep (
      id STRING PRIMARY KEY,
      pathId STRING,
      orderIndex INT32,
      stepType STRING,
      resourceId STRING,
      stepTitle STRING,
      stepNarrative STRING,
      isOptional BOOLEAN DEFAULT false,
      attestationRequired STRING,
      attestationGranted STRING,
      estimatedTime STRING
    )`,

    // Agent - Human, AI, or organization
    `CREATE NODE TABLE IF NOT EXISTS Agent (
      id STRING PRIMARY KEY,
      displayName STRING,
      agentType STRING,
      visibility STRING,
      bio STRING,
      profileReach STRING,
      attestations STRING[],
      createdAt TIMESTAMP,
      updatedAt TIMESTAMP
    )`,

    // AgentProgress - User progress on a path (private data)
    `CREATE NODE TABLE IF NOT EXISTS AgentProgress (
      id STRING PRIMARY KEY,
      agentId STRING,
      pathId STRING,
      currentStepIndex INT32,
      completedStepIndices INT32[],
      completedContentIds STRING[],
      startedAt TIMESTAMP,
      lastActivityAt TIMESTAMP,
      completedAt TIMESTAMP
    )`,

    // ContentAttestation - Trust/quality endorsement
    `CREATE NODE TABLE IF NOT EXISTS ContentAttestation (
      id STRING PRIMARY KEY,
      contentId STRING,
      attestationType STRING,
      reachGranted STRING,
      grantedBy STRING,
      grantedAt TIMESTAMP,
      expiresAt TIMESTAMP,
      status STRING,
      evidence STRING
    )`,

    // PathChapter - Organizational unit within a path
    `CREATE NODE TABLE IF NOT EXISTS PathChapter (
      id STRING PRIMARY KEY,
      pathId STRING,
      orderIndex INT32,
      title STRING,
      description STRING,
      estimatedDuration STRING,
      attestationGranted STRING
    )`
  ],

  relationships: [
    // Content relationships
    `CREATE REL TABLE IF NOT EXISTS CONTAINS (
      FROM ContentNode TO ContentNode,
      level INT32 DEFAULT 0
    )`,

    `CREATE REL TABLE IF NOT EXISTS RELATES_TO (
      FROM ContentNode TO ContentNode,
      score DOUBLE DEFAULT 0.5
    )`,

    `CREATE REL TABLE IF NOT EXISTS DEPENDS_ON (
      FROM ContentNode TO ContentNode
    )`,

    `CREATE REL TABLE IF NOT EXISTS REFERENCES (
      FROM ContentNode TO ContentNode
    )`,

    `CREATE REL TABLE IF NOT EXISTS IMPLEMENTS (
      FROM ContentNode TO ContentNode
    )`,

    `CREATE REL TABLE IF NOT EXISTS BELONGS_TO (
      FROM ContentNode TO ContentNode
    )`,

    // Path structure
    `CREATE REL TABLE IF NOT EXISTS PATH_HAS_CHAPTER (
      FROM LearningPath TO PathChapter
    )`,

    `CREATE REL TABLE IF NOT EXISTS CHAPTER_HAS_STEP (
      FROM PathChapter TO PathStep
    )`,

    `CREATE REL TABLE IF NOT EXISTS PATH_HAS_STEP (
      FROM LearningPath TO PathStep
    )`,

    `CREATE REL TABLE IF NOT EXISTS STEP_USES_CONTENT (
      FROM PathStep TO ContentNode
    )`,

    // Agent relationships
    `CREATE REL TABLE IF NOT EXISTS AUTHORED (
      FROM Agent TO ContentNode,
      authoredAt TIMESTAMP
    )`,

    `CREATE REL TABLE IF NOT EXISTS HAS_PROGRESS (
      FROM Agent TO AgentProgress
    )`,

    `CREATE REL TABLE IF NOT EXISTS PROGRESS_ON_PATH (
      FROM AgentProgress TO LearningPath
    )`,

    // Attestation relationships
    `CREATE REL TABLE IF NOT EXISTS ATTESTS (
      FROM ContentAttestation TO ContentNode
    )`,

    `CREATE REL TABLE IF NOT EXISTS ATTESTED_BY (
      FROM ContentAttestation TO Agent
    )`
  ]
};

/**
 * Relationship type mapping for import
 * Maps from JSON relationship.type to Cypher relationship table
 */
export const RELATIONSHIP_TYPE_MAP: Record<string, string> = {
  'CONTAINS': 'CONTAINS',
  'BELONGS_TO': 'BELONGS_TO',
  'RELATES_TO': 'RELATES_TO',
  'DEPENDS_ON': 'DEPENDS_ON',
  'REFERENCES': 'REFERENCES',
  'IMPLEMENTS': 'IMPLEMENTS',
  'DESCRIBES': 'RELATES_TO',  // Map similar types
  'VALIDATES': 'RELATES_TO',
  'FOLLOWS': 'DEPENDS_ON'
};

/**
 * Initialize the Kuzu database schema
 * Creates all node and relationship tables if they don't exist
 */
export async function initializeSchema(conn: Connection): Promise<void> {
  console.log('Initializing Kuzu schema...');

  // Create node tables
  for (const ddl of SCHEMA_DDL.nodes) {
    try {
      await conn.query(ddl);
    } catch (err) {
      // Table might already exist, which is fine
      console.log(`Note: ${(err as Error).message}`);
    }
  }

  // Create relationship tables
  for (const ddl of SCHEMA_DDL.relationships) {
    try {
      await conn.query(ddl);
    } catch (err) {
      console.log(`Note: ${(err as Error).message}`);
    }
  }

  console.log('Schema initialization complete.');
}

/**
 * Drop all tables (use with caution - for testing/reset)
 */
export async function dropAllTables(conn: Connection): Promise<void> {
  const tables = [
    // Drop relationships first (they depend on nodes)
    'ATTESTED_BY', 'ATTESTS', 'PROGRESS_ON_PATH', 'HAS_PROGRESS', 'AUTHORED',
    'STEP_USES_CONTENT', 'PATH_HAS_STEP', 'CHAPTER_HAS_STEP', 'PATH_HAS_CHAPTER',
    'BELONGS_TO', 'IMPLEMENTS', 'REFERENCES', 'DEPENDS_ON', 'RELATES_TO', 'CONTAINS',
    // Then drop nodes
    'PathChapter', 'ContentAttestation', 'AgentProgress', 'Agent',
    'PathStep', 'LearningPath', 'ContentNode'
  ];

  for (const table of tables) {
    try {
      await conn.query(`DROP TABLE IF EXISTS ${table}`);
    } catch (err) {
      console.log(`Could not drop ${table}: ${(err as Error).message}`);
    }
  }
}

/**
 * Get schema statistics
 */
export async function getSchemaStats(conn: Connection): Promise<Record<string, number>> {
  const stats: Record<string, number> = {};

  const nodeTables = ['ContentNode', 'LearningPath', 'PathStep', 'Agent', 'AgentProgress', 'ContentAttestation', 'PathChapter'];

  for (const table of nodeTables) {
    try {
      const result = await conn.query(`MATCH (n:${table}) RETURN count(n) as count`);
      const rows = await result.getAll();
      stats[table] = rows[0]?.count ?? 0;
    } catch {
      stats[table] = 0;
    }
  }

  return stats;
}
