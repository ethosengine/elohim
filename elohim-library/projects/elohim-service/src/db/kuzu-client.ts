/**
 * Kuzu Database Client for Node.js CLI
 *
 * Provides a simplified interface for database operations,
 * designed to be compatible with the DataClient interface
 * that will be shared with the Angular app.
 */

// eslint-disable-next-line @typescript-eslint/no-var-requires
const kuzu = require('kuzu');

import { initializeSchema, getSchemaStats, RELATIONSHIP_TYPE_MAP } from './kuzu-schema';
import { ContentRelationship } from '../models/content-node.model';
import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';

/**
 * Generate a Holochain-style hash ID
 *
 * Holochain uses Blake2b-256 hashes encoded as Base64 for ActionHash/EntryHash.
 * We use SHA-256 (widely available) with URL-safe Base64 encoding.
 *
 * Format: uhC[base64] mimics Holochain's "hC" prefix for content hashes
 *
 * @param input - String to hash (typically: type + content or parent + order)
 * @returns 43-character hash string (like Holochain's ActionHash)
 */
export function generateHashId(input: string): string {
  const hash = crypto.createHash('sha256').update(input).digest('base64url');
  // Prefix with 'uhC' to mimic Holochain's hCk... pattern
  // Truncate to 43 chars total (similar to Holochain ActionHash length)
  return 'uhC' + hash.substring(0, 40);
}

/**
 * Generate a deterministic hash ID for a step
 * Uses path ID + order + resourceId for uniqueness and reproducibility
 */
export function generateStepHashId(pathId: string, order: number, resourceId: string): string {
  return generateHashId(`step:${pathId}:${order}:${resourceId}`);
}

/**
 * Generate a deterministic hash ID for a chapter
 */
export function generateChapterHashId(pathId: string, order: number, title: string): string {
  return generateHashId(`chapter:${pathId}:${order}:${title}`);
}

export interface ContentNode {
  id: string;
  contentType: string;
  title: string;
  description?: string;
  content?: string;
  contentFormat?: string;
  tags?: string[];
  authorId?: string;
  reach?: string;
  trustScore?: number;
  metadata?: Record<string, unknown>;
  sourcePath?: string;
  relatedNodeIds?: string[];
  createdAt?: string;
  updatedAt?: string;
}

export interface LearningPath {
  id: string;
  version?: string;
  title: string;
  description?: string;
  purpose?: string;
  createdBy?: string;
  difficulty?: string;
  estimatedDuration?: string;
  visibility?: string;
  pathType?: string;
  tags?: string[];
  steps?: PathStep[];
  chapters?: PathChapter[];
  createdAt?: string;
  updatedAt?: string;
}

export interface PathStep {
  order: number;
  stepType?: string;
  resourceId: string;
  stepTitle?: string;
  stepNarrative?: string;
  optional?: boolean;
  attestationRequired?: string;
  attestationGranted?: string;
  estimatedTime?: string;
  learningObjectives?: string[];
  completionCriteria?: string[];
}

export interface PathChapter {
  id: string;
  title: string;
  description?: string;
  order: number;
  steps: PathStep[];
  estimatedDuration?: string;
  attestationGranted?: string;
}


/**
 * Escape a string for use in Cypher queries
 */
function escapeString(value: string): string {
  if (value === null || value === undefined) return '""';
  return `"${String(value).replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

/**
 * Escape an array for use in Cypher queries
 */
function escapeArray(arr: string[] | undefined): string {
  if (!arr || arr.length === 0) return '[]';
  return `[${arr.map(escapeString).join(', ')}]`;
}

/**
 * Kuzu database client for Node.js
 */
export class KuzuClient {
  private db: any = null;
  private conn: any = null;
  private dbPath: string;

  constructor(dbPath: string) {
    this.dbPath = dbPath;
  }

  /**
   * Initialize the database connection
   */
  async initialize(): Promise<void> {
    // Ensure directory exists
    const dir = path.dirname(this.dbPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    this.db = new kuzu.Database(this.dbPath);
    this.conn = new kuzu.Connection(this.db);
    await initializeSchema(this.conn);
  }

  /**
   * Close the database connection
   */
  close(): void {
    if (this.conn) {
      this.conn = null;
    }
    if (this.db) {
      this.db.close();
      this.db = null;
    }
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.conn !== null;
  }

  /**
   * Get connection (throws if not initialized)
   */
  private getConnection(): any {
    if (!this.conn) {
      throw new Error('Database not initialized. Call initialize() first.');
    }
    return this.conn;
  }

  // ==========================================================================
  // Content Node Operations
  // ==========================================================================

  /**
   * Create a content node
   */
  async createContentNode(node: ContentNode): Promise<ContentNode> {
    const conn = this.getConnection();
    const now = new Date().toISOString();

    const query = `
      CREATE (n:ContentNode {
        id: ${escapeString(node.id)},
        contentType: ${escapeString(node.contentType || 'content')},
        title: ${escapeString(node.title)},
        description: ${escapeString(node.description || '')},
        content: ${escapeString(node.content || '')},
        contentFormat: ${escapeString(node.contentFormat || 'markdown')},
        tags: ${escapeArray(node.tags)},
        authorId: ${escapeString(node.authorId || '')},
        reach: ${escapeString(node.reach || 'commons')},
        trustScore: ${node.trustScore || 0.0},
        metadata: ${escapeString(JSON.stringify(node.metadata || {}))},
        sourcePath: ${escapeString(node.sourcePath || '')},
        createdAt: timestamp(${escapeString(node.createdAt || now)}),
        updatedAt: timestamp(${escapeString(node.updatedAt || now)})
      })
    `;

    await conn.query(query);
    return { ...node, createdAt: node.createdAt || now, updatedAt: node.updatedAt || now };
  }

  /**
   * Get a content node by ID
   */
  async getContentNode(id: string): Promise<ContentNode | null> {
    const conn = this.getConnection();
    const result = await conn.query(
      `MATCH (n:ContentNode) WHERE n.id = ${escapeString(id)} RETURN n`
    );
    const rows = await result.getAll();
    if (rows.length === 0) return null;

    return this.rowToContentNode(rows[0].n);
  }

  /**
   * Search content nodes
   */
  async searchContentNodes(options: {
    contentTypes?: string[];
    tags?: string[];
    limit?: number;
  }): Promise<ContentNode[]> {
    const conn = this.getConnection();
    let query = 'MATCH (n:ContentNode)';
    const conditions: string[] = [];

    if (options.contentTypes?.length) {
      const types = options.contentTypes.map(escapeString).join(', ');
      conditions.push(`n.contentType IN [${types}]`);
    }

    if (conditions.length > 0) {
      query += ` WHERE ${conditions.join(' AND ')}`;
    }

    query += ` RETURN n`;
    if (options.limit) {
      query += ` LIMIT ${options.limit}`;
    }

    const result = await conn.query(query);
    const rows = await result.getAll();
    return rows.map((row: any) => this.rowToContentNode(row.n));
  }

  /**
   * Delete a content node
   */
  async deleteContentNode(id: string): Promise<void> {
    const conn = this.getConnection();
    await conn.query(
      `MATCH (n:ContentNode) WHERE n.id = ${escapeString(id)} DELETE n`
    );
  }

  // ==========================================================================
  // Learning Path Operations
  // ==========================================================================

  /**
   * Create a learning path
   */
  async createPath(pathData: Omit<LearningPath, 'createdAt' | 'updatedAt'> & { createdAt?: string; updatedAt?: string }): Promise<LearningPath> {
    const conn = this.getConnection();
    const now = new Date().toISOString();

    const query = `
      CREATE (p:LearningPath {
        id: ${escapeString(pathData.id)},
        version: ${escapeString(pathData.version || '1.0.0')},
        title: ${escapeString(pathData.title)},
        description: ${escapeString(pathData.description || '')},
        purpose: ${escapeString(pathData.purpose || '')},
        createdBy: ${escapeString(pathData.createdBy || 'cli')},
        difficulty: ${escapeString(pathData.difficulty || 'intermediate')},
        estimatedDuration: ${escapeString(pathData.estimatedDuration || '')},
        visibility: ${escapeString(pathData.visibility || 'public')},
        pathType: ${escapeString(pathData.pathType || 'journey')},
        tags: ${escapeArray(pathData.tags)},
        createdAt: timestamp(${escapeString(pathData.createdAt || now)}),
        updatedAt: timestamp(${escapeString(pathData.updatedAt || now)})
      })
    `;

    await conn.query(query);

    // Create steps if provided
    if (pathData.steps?.length) {
      for (const step of pathData.steps) {
        await this.addPathStep(pathData.id, step);
      }
    }

    // Create chapters if provided
    if (pathData.chapters?.length) {
      for (const chapter of pathData.chapters) {
        await this.addPathChapter(pathData.id, chapter);
      }
    }

    return { ...pathData, createdAt: pathData.createdAt || now, updatedAt: pathData.updatedAt || now };
  }

  /**
   * Get a learning path by ID
   */
  async getPath(id: string): Promise<LearningPath | null> {
    const conn = this.getConnection();

    // Get path node
    const pathResult = await conn.query(
      `MATCH (p:LearningPath) WHERE p.id = ${escapeString(id)} RETURN p`
    );
    const pathRows = await pathResult.getAll();
    if (pathRows.length === 0) return null;

    const pathRow = pathRows[0].p;

    // Get steps
    const stepsResult = await conn.query(
      `MATCH (p:LearningPath)-[:PATH_HAS_STEP]->(s:PathStep)
       WHERE p.id = ${escapeString(id)}
       RETURN s ORDER BY s.orderIndex`
    );
    const stepsRows = await stepsResult.getAll();

    // Get chapters
    const chaptersResult = await conn.query(
      `MATCH (p:LearningPath)-[:PATH_HAS_CHAPTER]->(c:PathChapter)
       WHERE p.id = ${escapeString(id)}
       RETURN c ORDER BY c.orderIndex`
    );
    const chaptersRows = await chaptersResult.getAll();

    const learningPath = this.rowToPath(pathRow);
    learningPath.steps = stepsRows.map((row: any) => this.rowToStep(row.s));

    if (chaptersRows.length > 0) {
      learningPath.chapters = [];
      for (const chapterRow of chaptersRows) {
        const chapter = this.rowToChapter(chapterRow.c);
        // Get chapter steps
        const chapterStepsResult = await conn.query(
          `MATCH (c:PathChapter)-[:CHAPTER_HAS_STEP]->(s:PathStep)
           WHERE c.id = ${escapeString(chapter.id)}
           RETURN s ORDER BY s.orderIndex`
        );
        const chapterStepsRows = await chapterStepsResult.getAll();
        chapter.steps = chapterStepsRows.map((row: any) => this.rowToStep(row.s));
        learningPath.chapters.push(chapter);
      }
    }

    return learningPath;
  }

  /**
   * List all paths
   */
  async listPaths(): Promise<LearningPath[]> {
    const conn = this.getConnection();
    const result = await conn.query('MATCH (p:LearningPath) RETURN p');
    const rows = await result.getAll();
    return rows.map((row: any) => this.rowToPath(row.p));
  }

  /**
   * Add a step to a path
   */
  async addPathStep(pathId: string, step: PathStep, chapterId?: string): Promise<void> {
    const conn = this.getConnection();
    // Use Holochain-style hash ID for uniqueness
    const stepId = generateStepHashId(pathId, step.order, step.resourceId);

    // Create step node
    await conn.query(`
      CREATE (s:PathStep {
        id: ${escapeString(stepId)},
        pathId: ${escapeString(pathId)},
        orderIndex: ${step.order},
        stepType: ${escapeString(step.stepType || 'content')},
        resourceId: ${escapeString(step.resourceId)},
        stepTitle: ${escapeString(step.stepTitle || '')},
        stepNarrative: ${escapeString(step.stepNarrative || '')},
        isOptional: ${step.optional || false},
        attestationRequired: ${escapeString(step.attestationRequired || '')},
        attestationGranted: ${escapeString(step.attestationGranted || '')},
        estimatedTime: ${escapeString(step.estimatedTime || '')}
      })
    `);

    // Link to path or chapter
    if (chapterId) {
      await conn.query(`
        MATCH (c:PathChapter), (s:PathStep)
        WHERE c.id = ${escapeString(chapterId)} AND s.id = ${escapeString(stepId)}
        CREATE (c)-[:CHAPTER_HAS_STEP]->(s)
      `);
    } else {
      await conn.query(`
        MATCH (p:LearningPath), (s:PathStep)
        WHERE p.id = ${escapeString(pathId)} AND s.id = ${escapeString(stepId)}
        CREATE (p)-[:PATH_HAS_STEP]->(s)
      `);
    }

    // Link step to content if resourceId exists (optional - content may be imported later)
    if (step.resourceId) {
      try {
        await conn.query(`
          MATCH (s:PathStep), (c:ContentNode)
          WHERE s.id = ${escapeString(stepId)} AND c.id = ${escapeString(step.resourceId)}
          CREATE (s)-[:STEP_USES_CONTENT]->(c)
        `);
      } catch (err) {
        // Expected: ContentNode may not exist yet if paths are imported before content
        console.warn(`Step ${stepId}: ContentNode '${step.resourceId}' not found (will link when content is imported)`);
      }
    }
  }

  /**
   * Add a chapter to a path
   */
  async addPathChapter(pathId: string, chapter: PathChapter): Promise<void> {
    const conn = this.getConnection();
    // Use Holochain-style hash ID for uniqueness
    const chapterId = chapter.id || generateChapterHashId(pathId, chapter.order, chapter.title);

    // Create chapter node
    await conn.query(`
      CREATE (c:PathChapter {
        id: ${escapeString(chapterId)},
        pathId: ${escapeString(pathId)},
        orderIndex: ${chapter.order},
        title: ${escapeString(chapter.title)},
        description: ${escapeString(chapter.description || '')},
        estimatedDuration: ${escapeString(chapter.estimatedDuration || '')},
        attestationGranted: ${escapeString(chapter.attestationGranted || '')}
      })
    `);

    // Link to path
    await conn.query(`
      MATCH (p:LearningPath), (c:PathChapter)
      WHERE p.id = ${escapeString(pathId)} AND c.id = ${escapeString(chapterId)}
      CREATE (p)-[:PATH_HAS_CHAPTER]->(c)
    `);

    // Add chapter steps
    if (chapter.steps?.length) {
      for (const step of chapter.steps) {
        await this.addPathStep(pathId, step, chapterId);
      }
    }
  }

  /**
   * Delete a path
   */
  async deletePath(id: string): Promise<void> {
    const conn = this.getConnection();
    // Delete steps, chapters, and path
    await conn.query(`
      MATCH (p:LearningPath)
      WHERE p.id = ${escapeString(id)}
      OPTIONAL MATCH (p)-[:PATH_HAS_STEP]->(s:PathStep)
      OPTIONAL MATCH (p)-[:PATH_HAS_CHAPTER]->(c:PathChapter)
      OPTIONAL MATCH (c)-[:CHAPTER_HAS_STEP]->(cs:PathStep)
      DETACH DELETE s, cs, c, p
    `);
  }

  // ==========================================================================
  // Relationship Operations
  // ==========================================================================

  /**
   * Create a relationship between content nodes
   */
  async createRelationship(rel: ContentRelationship): Promise<void> {
    const conn = this.getConnection();
    const relType = RELATIONSHIP_TYPE_MAP[rel.relationshipType] || 'RELATES_TO';

    const query = `
      MATCH (a:ContentNode), (b:ContentNode)
      WHERE a.id = ${escapeString(rel.sourceNodeId)} AND b.id = ${escapeString(rel.targetNodeId)}
      CREATE (a)-[:${relType}]->(b)
    `;

    await conn.query(query);
  }

  /**
   * Bulk insert relationships
   */
  async bulkInsertRelationships(relationships: ContentRelationship[]): Promise<number> {
    let inserted = 0;
    for (const rel of relationships) {
      try {
        await this.createRelationship(rel);
        inserted++;
      } catch {
        // Node might not exist, skip
      }
    }
    return inserted;
  }

  // ==========================================================================
  // Cypher Query Execution
  // ==========================================================================

  /**
   * Execute a raw Cypher query
   */
  async query<T = unknown>(cypher: string): Promise<T[]> {
    const conn = this.getConnection();
    const result = await conn.query(cypher);
    return result.getAll() as T[];
  }

  // ==========================================================================
  // Bulk Operations for Import
  // ==========================================================================

  /**
   * Bulk insert content nodes
   */
  async bulkInsertContentNodes(nodes: ContentNode[]): Promise<number> {
    let inserted = 0;
    for (const node of nodes) {
      try {
        await this.createContentNode(node);
        inserted++;
      } catch (err) {
        console.error(`Failed to insert node ${node.id}:`, (err as Error).message);
      }
    }
    return inserted;
  }

  /**
   * Get schema statistics
   */
  async getStats(): Promise<Record<string, number>> {
    return getSchemaStats(this.getConnection());
  }

  // ==========================================================================
  // Helper Methods
  // ==========================================================================

  private rowToContentNode(row: any): ContentNode {
    return {
      id: row.id,
      contentType: row.contentType,
      title: row.title,
      description: row.description,
      content: row.content,
      contentFormat: row.contentFormat,
      tags: row.tags,
      authorId: row.authorId,
      reach: row.reach,
      trustScore: row.trustScore,
      metadata: row.metadata ? JSON.parse(row.metadata) : undefined,
      sourcePath: row.sourcePath,
      createdAt: row.createdAt?.toString(),
      updatedAt: row.updatedAt?.toString()
    };
  }

  private rowToPath(row: any): LearningPath {
    return {
      id: row.id,
      version: row.version,
      title: row.title,
      description: row.description,
      purpose: row.purpose,
      createdBy: row.createdBy,
      difficulty: row.difficulty,
      estimatedDuration: row.estimatedDuration,
      visibility: row.visibility,
      pathType: row.pathType,
      tags: row.tags,
      steps: [],
      createdAt: row.createdAt?.toString(),
      updatedAt: row.updatedAt?.toString()
    };
  }

  private rowToStep(row: any): PathStep {
    return {
      order: row.orderIndex,
      stepType: row.stepType,
      resourceId: row.resourceId,
      stepTitle: row.stepTitle,
      stepNarrative: row.stepNarrative,
      optional: row.isOptional,
      attestationRequired: row.attestationRequired,
      attestationGranted: row.attestationGranted,
      estimatedTime: row.estimatedTime
    };
  }

  private rowToChapter(row: any): PathChapter {
    return {
      id: row.id,
      title: row.title,
      description: row.description,
      order: row.orderIndex,
      steps: [],
      estimatedDuration: row.estimatedDuration,
      attestationGranted: row.attestationGranted
    };
  }
}
