/**
 * KuzuDataService - Embedded graph database client for Angular
 *
 * Uses @kuzu/kuzu-wasm to run Kuzu in the browser via WebAssembly.
 * This is the Holochain-ready replacement for JSON file loading.
 *
 * Data flow:
 * 1. On app init, loads the Cypher seed file from assets
 * 2. Initializes Kuzu WASM database in IndexedDB
 * 3. Executes seed to populate data
 * 4. Provides Observable-based API matching DataLoaderService
 */

import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, from, of, BehaviorSubject } from 'rxjs';
import { map, switchMap, shareReplay, catchError, tap } from 'rxjs/operators';
// Models from lamad pillar (content-specific)
import { LearningPath, PathIndex } from '../../lamad/models/learning-path.model';
import { ContentNode, ContentGraph, ContentGraphMetadata } from '../../lamad/models/content-node.model';

// Local type definitions for kuzu-wasm high-level API
// The module exposes Database() and Connection() async factory functions

interface KuzuQueryResult {
  table: {
    toString(): string;
    toArray(): any[];
    length: number;
  };
  numTuples?: number;
}

interface KuzuConnection {
  execute(cypher: string): Promise<KuzuQueryResult>;
  close(): void;
}

interface KuzuDatabase {
  close(): void;
}

// Emscripten filesystem interface for COPY FROM support
interface EmscriptenFS {
  writeFile(path: string, data: Uint8Array | string): void;
  readFile(path: string, opts?: { encoding?: string }): Uint8Array | string;
  unlink(path: string): void;
}

// The high-level API uses async factory functions
interface KuzuModule {
  Database(): Promise<KuzuDatabase>;
  Connection(db: KuzuDatabase): Promise<KuzuConnection>;
  FS?: EmscriptenFS; // Emscripten virtual filesystem
}

@Injectable({ providedIn: 'root' })
export class KuzuDataService {
  private kuzu: KuzuModule | null = null;
  private db: KuzuDatabase | null = null;
  private conn: KuzuConnection | null = null;
  private readonly ready$ = new BehaviorSubject<boolean>(false);
  private initPromise: Promise<void> | null = null;

  // Caches
  private readonly pathCache = new Map<string, Observable<LearningPath>>();
  private readonly contentCache = new Map<string, Observable<ContentNode>>();
  private graphCache$: Observable<ContentGraph> | null = null;

  constructor(private readonly http: HttpClient) {}

  /**
   * Load Kuzu module from assets folder.
   * Bypasses Vite's bundler which has issues with WASM modules.
   * Uses kuzu-browser.js which provides the high-level Database/Connection API.
   */
  private async loadKuzuModule(): Promise<KuzuModule> {
    // Load kuzu-browser.js which wraps the WASM module with high-level API
    // This provides Database() and Connection() async factory functions
    const moduleUrl = '/assets/wasm/kuzu-browser.js';

    // Dynamic import from absolute URL
    const kuzuModule = await import(/* @vite-ignore */ moduleUrl);

    // The default export is a factory function that returns the initialized kuzu instance
    const kuzuFactory = kuzuModule.default || kuzuModule;

    // Call the factory with locateFile to find WASM in our assets folder
    const kuzu = await kuzuFactory({
      locateFile: (path: string) => '/assets/wasm/' + path
    }) as KuzuModule;

    return kuzu;
  }

  /**
   * Initialize the Kuzu WASM database.
   * Call this once at app startup.
   */
  async initialize(): Promise<void> {
    if (this.initPromise) {
      return this.initPromise;
    }

    this.initPromise = this.doInitialize();
    return this.initPromise;
  }

  private async doInitialize(): Promise<void> {
    try {
      // Check for Cross-Origin Isolation (required for SharedArrayBuffer)
      if (typeof crossOriginIsolated !== 'undefined' && !crossOriginIsolated) {
        console.warn('[KuzuData] Cross-Origin Isolation not enabled. Kuzu WASM requires COOP/COEP headers.');
        console.warn('[KuzuData] Add these headers to your server:');
        console.warn('  Cross-Origin-Opener-Policy: same-origin');
        console.warn('  Cross-Origin-Embedder-Policy: require-corp');
        throw new Error('Cross-Origin Isolation required for Kuzu WASM');
      }

      // Load kuzu from assets via dynamic import
      // The module exposes Database() and Connection() async factory functions
      const kuzuModule = await this.loadKuzuModule();
      this.kuzu = kuzuModule;

      // Create in-memory database and connection using async factory functions
      this.db = await this.kuzu.Database();
      this.conn = await this.kuzu.Connection(this.db);

      // Create schema and load seed data
      await this.createSchema();
      await this.loadSeedData();

      this.ready$.next(true);
    } catch (err) {
      console.error('[KuzuData] Initialization failed:', err);
      throw err;
    }
  }

  private async createSchema(): Promise<void> {
    if (!this.conn) throw new Error('Connection not initialized');

    const schema = [
      `CREATE NODE TABLE IF NOT EXISTS ContentNode (
        id STRING PRIMARY KEY,
        contentType STRING,
        title STRING,
        description STRING,
        content STRING,
        contentFormat STRING,
        tags STRING[],
        relatedNodeIds STRING[],
        authorId STRING,
        reach STRING,
        trustScore DOUBLE DEFAULT 0.0,
        metadata STRING,
        sourcePath STRING,
        createdAt TIMESTAMP,
        updatedAt TIMESTAMP
      )`,
      // ContentChunk stores large content split into smaller pieces
      // to work around Kuzu WASM string literal size limitations
      `CREATE NODE TABLE IF NOT EXISTS ContentChunk (
        id STRING PRIMARY KEY,
        parentId STRING,
        chunkIndex INT32,
        totalChunks INT32,
        content STRING
      )`,
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
        thumbnailUrl STRING,
        thumbnailAlt STRING,
        createdAt TIMESTAMP,
        updatedAt TIMESTAMP
      )`,
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
      `CREATE REL TABLE IF NOT EXISTS CONTAINS (
        FROM ContentNode TO ContentNode,
        level INT32 DEFAULT 0
      )`,
      `CREATE REL TABLE IF NOT EXISTS RELATES_TO (
        FROM ContentNode TO ContentNode,
        score DOUBLE DEFAULT 0.5
      )`,
      `CREATE REL TABLE IF NOT EXISTS PATH_HAS_STEP (
        FROM LearningPath TO PathStep
      )`,
      `CREATE REL TABLE IF NOT EXISTS STEP_USES_CONTENT (
        FROM PathStep TO ContentNode
      )`,
      `CREATE REL TABLE IF NOT EXISTS HAS_CHUNK (
        FROM ContentNode TO ContentChunk
      )`
    ];

    for (const ddl of schema) {
      try {
        await this.conn!.execute(ddl);
      } catch {
        // Table might already exist - ignore
      }
    }
  }

  private async loadSeedData(): Promise<void> {
    if (!this.conn) throw new Error('Connection not initialized');

    try {
      // Load main seed file from assets
      const seedUrl = '/assets/lamad-data/lamad-seed.cypher';
      const response = await fetch(seedUrl);

      if (!response.ok) {
        console.warn('[KuzuData] No seed file found, starting with empty database');
        return;
      }

      const seedContent = await response.text();
      const statements = seedContent
        .split('\n')
        .filter(line => line.trim() && !line.trim().startsWith('//'))
        .join('\n')
        .split(';')
        .filter(stmt => stmt.trim());

      let failCount = 0;
      for (const stmt of statements) {
        if (stmt.trim()) {
          try {
            await this.conn!.execute(stmt.trim() + ';');
          } catch (err) {
            failCount++;
            // Log first few failures for debugging
            if (failCount <= 3) {
              console.warn('[KuzuData] Seed statement failed:', stmt.substring(0, 100), (err as Error).message);
            }
          }
        }
      }

      // Load content chunks file (for large content that was split)
      await this.loadContentChunks();
    } catch (err) {
      console.warn('[KuzuData] Could not load seed data:', err);
    }
  }

  /**
   * Load content chunks for large content that was split to work around
   * Kuzu WASM string literal size limitations.
   */
  private async loadContentChunks(): Promise<void> {
    if (!this.conn) return;

    try {
      const chunksUrl = '/assets/lamad-data/content-chunks.cypher';
      const response = await fetch(chunksUrl);

      if (!response.ok) {
        // No chunks file is normal for smaller content sets
        return;
      }

      const chunksContent = await response.text();
      const statements = chunksContent
        .split('\n')
        .filter(line => line.trim() && !line.trim().startsWith('//'))
        .join('\n')
        .split(';')
        .filter(stmt => stmt.trim());

      let chunkFail = 0;
      for (const stmt of statements) {
        if (stmt.trim()) {
          try {
            await this.conn!.execute(stmt.trim() + ';');
          } catch (err) {
            chunkFail++;
            if (chunkFail <= 3) {
              console.warn('[KuzuData] Chunk statement failed:', stmt.substring(0, 80), (err as Error).message);
            }
          }
        }
      }
    } catch (err) {
      console.warn('[KuzuData] Could not load content chunks:', err);
    }
  }

  /**
   * Wait for database to be ready.
   */
  whenReady(): Observable<boolean> {
    return this.ready$.asObservable();
  }

  /**
   * Check if database is ready.
   */
  isReady(): boolean {
    return this.ready$.value;
  }

  /**
   * Execute a Cypher query.
   */
  async query<T = any>(cypher: string): Promise<T[]> {
    if (!this.conn) {
      await this.initialize();
    }

    const result = await this.conn!.execute(cypher);

    // Extract data from result - try table.toArray() first (primary approach)
    try {
      if (result.table && typeof result.table.toArray === 'function') {
        return result.table.toArray() as T[];
      }

      // Fallback: result.table.toString() with JSON parse
      if (result.table && typeof result.table.toString === 'function') {
        const parsed = JSON.parse(result.table.toString());
        return Array.isArray(parsed) ? parsed : [parsed];
      }

      // Result itself might be the data
      if (Array.isArray(result)) {
        return result as T[];
      }

      return [];
    } catch {
      return [];
    }
  }

  /**
   * Load a LearningPath by ID.
   */
  getPath(pathId: string): Observable<LearningPath> {
    if (!this.pathCache.has(pathId)) {
      const request = from(this.fetchPath(pathId)).pipe(
        shareReplay(1),
        catchError(() => {
          throw new Error(`Path not found: ${pathId}`);
        })
      );
      this.pathCache.set(pathId, request);
    }
    return this.pathCache.get(pathId)!;
  }

  private async fetchPath(pathId: string): Promise<LearningPath> {
    await this.initialize();

    // Get path metadata - return individual properties
    const pathResult = await this.query<any>(
      `MATCH (p:LearningPath) WHERE p.id = "${pathId}"
       RETURN p.id, p.version, p.title, p.description, p.purpose, p.createdBy,
              p.difficulty, p.estimatedDuration, p.visibility, p.pathType, p.tags,
              p.thumbnailUrl, p.thumbnailAlt, p.createdAt, p.updatedAt`
    );

    if (!pathResult.length) {
      throw new Error(`Path not found: ${pathId}`);
    }

    const row = pathResult[0];

    // Get path steps
    const stepsResult = await this.query<any>(
      `MATCH (s:PathStep) WHERE s.pathId = "${pathId}"
       RETURN s.orderIndex, s.stepType, s.resourceId, s.stepTitle, s.stepNarrative,
              s.isOptional, s.attestationRequired, s.attestationGranted, s.estimatedTime
       ORDER BY s.orderIndex`
    );

    const steps = stepsResult.map((s: any) => ({
      order: s['s.orderIndex'],
      stepType: s['s.stepType'] || 'content',
      resourceId: s['s.resourceId'],
      stepTitle: this.unescapeCypher(s['s.stepTitle'] || ''),
      stepNarrative: this.unescapeCypher(s['s.stepNarrative'] || ''),
      optional: s['s.isOptional'],
      attestationRequired: s['s.attestationRequired'],
      attestationGranted: s['s.attestationGranted'],
      estimatedTime: s['s.estimatedTime'],
      learningObjectives: [],
      completionCriteria: []
    }));

    return {
      id: row['p.id'],
      version: row['p.version'],
      title: this.unescapeCypher(row['p.title'] || ''),
      description: this.unescapeCypher(row['p.description'] || ''),
      purpose: this.unescapeCypher(row['p.purpose'] || ''),
      createdBy: row['p.createdBy'],
      contributors: [], // TODO: Load contributors
      difficulty: row['p.difficulty'],
      estimatedDuration: row['p.estimatedDuration'],
      visibility: row['p.visibility'] || 'public',
      pathType: row['p.pathType'],
      tags: row['p.tags'] || [],
      thumbnailUrl: row['p.thumbnailUrl'] || undefined,
      thumbnailAlt: row['p.thumbnailAlt'] || undefined,
      steps,
      chapters: [], // TODO: Load chapters
      createdAt: row['p.createdAt'],
      updatedAt: row['p.updatedAt']
    };
  }

  /**
   * Load a ContentNode by ID.
   */
  getContent(resourceId: string): Observable<ContentNode> {
    if (!this.contentCache.has(resourceId)) {
      const request = from(this.fetchContent(resourceId)).pipe(
        shareReplay(1),
        catchError(() => {
          throw new Error(`Content not found: ${resourceId}`);
        })
      );
      this.contentCache.set(resourceId, request);
    }
    return this.contentCache.get(resourceId)!;
  }

  private async fetchContent(resourceId: string): Promise<ContentNode> {
    await this.initialize();

    const result = await this.query<any>(
      `MATCH (c:ContentNode) WHERE c.id = "${resourceId}"
       RETURN c.id, c.contentType, c.title, c.description, c.content, c.contentFormat,
              c.tags, c.relatedNodeIds, c.authorId, c.reach, c.trustScore, c.metadata, c.sourcePath,
              c.createdAt, c.updatedAt`
    );

    if (!result.length) {
      throw new Error(`Content not found: ${resourceId}`);
    }

    const row = result[0];
    let content = row['c.content'] || '';

    // Check if this content has chunks (for large content that was split)
    // If content is empty or marked as chunked, try to reassemble from chunks
    if (!content || content === '[CHUNKED]') {
      const reassembled = await this.reassembleChunks(resourceId);
      if (reassembled) {
        content = reassembled;
      }
    } else {
      // Unescape non-chunked content too (it also has escaped newlines, quotes, etc.)
      content = this.unescapeCypher(content);
    }

    return {
      id: row['c.id'],
      contentType: row['c.contentType'],
      title: this.unescapeCypher(row['c.title'] || ''),
      description: this.unescapeCypher(row['c.description'] || ''),
      content,
      contentFormat: row['c.contentFormat'] || 'markdown',
      tags: row['c.tags'] || [],
      relatedNodeIds: row['c.relatedNodeIds'] || [],
      authorId: row['c.authorId'],
      reach: row['c.reach'] || 'commons',
      trustScore: row['c.trustScore'],
      metadata: row['c.metadata'] ? JSON.parse(row['c.metadata']) : {},
      sourcePath: row['c.sourcePath'],
      createdAt: row['c.createdAt'],
      updatedAt: row['c.updatedAt']
    };
  }

  /**
   * Reassemble content from ContentChunk nodes for large content.
   * Returns null if no chunks found.
   */
  private async reassembleChunks(resourceId: string): Promise<string | null> {
    try {
      const chunkResult = await this.query<any>(
        `MATCH (ch:ContentChunk) WHERE ch.parentId = "${resourceId}"
         RETURN ch.chunkIndex, ch.content, ch.totalChunks
         ORDER BY ch.chunkIndex`
      );

      if (!chunkResult.length) {
        return null;
      }

      // Sort by index and concatenate
      const sortedChunks = chunkResult.sort(
        (a: any, b: any) => a['ch.chunkIndex'] - b['ch.chunkIndex']
      );

      // Concatenate chunks and unescape Cypher string escapes
      const rawContent = sortedChunks.map((c: any) => c['ch.content'] || '').join('');
      const content = this.unescapeCypher(rawContent);
      return content;
    } catch (err) {
      console.warn(`[KuzuData] Error reassembling chunks for "${resourceId}":`, err);
      return null;
    }
  }

  /**
   * Unescape content by converting placeholders back to original characters.
   * Uses placeholders instead of escape sequences because Kuzu WASM
   * mangles traditional escape sequences (strips backslash from \n).
   */
  private unescapeCypher(str: string): string {
    if (!str) return '';
    return str
      .replace(/\{\{NEWLINE\}\}/g, '\n')     // Placeholder back to newline
      .replace(/\{\{QUOTE\}\}/g, "'")        // Placeholder back to single quote
      .replace(/\{\{DQUOTE\}\}/g, '"')       // Placeholder back to double quote
      .replace(/\{\{BACKSLASH\}\}/g, '\\');  // Placeholder back to backslash
  }

  /**
   * Load the path index.
   */
  getPathIndex(): Observable<PathIndex> {
    return from(this.fetchPathIndex()).pipe(
      catchError(() => of({ paths: [], totalCount: 0, lastUpdated: new Date().toISOString() }))
    );
  }

  private async fetchPathIndex(): Promise<PathIndex> {
    await this.initialize();

    const result = await this.query<any>(
      `MATCH (p:LearningPath)
       RETURN p.id, p.title, p.description, p.difficulty, p.estimatedDuration, p.tags, p.pathType,
              p.thumbnailUrl, p.thumbnailAlt`
    );

    const paths = result.map((row: any) => ({
      id: row['p.id'],
      title: row['p.title'],
      description: row['p.description'],
      difficulty: row['p.difficulty'],
      estimatedDuration: row['p.estimatedDuration'],
      stepCount: 0, // Would need a count query
      tags: row['p.tags'] || [],
      pathType: row['p.pathType'],
      thumbnailUrl: row['p.thumbnailUrl'] || undefined,
      thumbnailAlt: row['p.thumbnailAlt'] || undefined
    }));

    return {
      paths,
      totalCount: paths.length,
      lastUpdated: new Date().toISOString()
    };
  }

  /**
   * Load the content index.
   */
  getContentIndex(): Observable<{ nodes: ContentNode[]; lastUpdated: string }> {
    return from(this.fetchContentIndex()).pipe(
      catchError(() => of({ nodes: [], lastUpdated: new Date().toISOString() }))
    );
  }

  private async fetchContentIndex(): Promise<{ nodes: ContentNode[]; lastUpdated: string }> {
    await this.initialize();

    const result = await this.query<any>(
      `MATCH (c:ContentNode) RETURN c.id, c.contentType, c.title, c.description, c.tags`
    );

    const nodes = result.map((row: any) => ({
      id: row['c.id'],
      contentType: row['c.contentType'],
      title: row['c.title'],
      description: row['c.description'],
      tags: row['c.tags'] || [],
      content: '',
      contentFormat: 'markdown' as const,
      relatedNodeIds: [],
      reach: 'commons' as const,
      metadata: {}
    }));

    return {
      nodes,
      lastUpdated: new Date().toISOString()
    };
  }

  /**
   * Search content nodes by query.
   */
  searchContent(query: string, limit = 20): Observable<ContentNode[]> {
    return from(this.doSearch(query, limit));
  }

  private async doSearch(query: string, limit: number): Promise<ContentNode[]> {
    await this.initialize();

    const escapedQuery = query.replace(/"/g, '\\"').toLowerCase();

    const result = await this.query<any>(`
      MATCH (c:ContentNode)
      WHERE toLower(c.title) CONTAINS "${escapedQuery}"
         OR toLower(c.description) CONTAINS "${escapedQuery}"
      RETURN c.id, c.contentType, c.title, c.description, c.content, c.contentFormat,
             c.tags, c.reach
      LIMIT ${limit}
    `);

    return result.map((row: any) => ({
      id: row['c.id'],
      contentType: row['c.contentType'],
      title: row['c.title'],
      description: row['c.description'],
      content: row['c.content'],
      contentFormat: row['c.contentFormat'] || 'markdown',
      tags: row['c.tags'] || [],
      relatedNodeIds: [],
      reach: row['c.reach'] || 'commons',
      metadata: {}
    }));
  }

  /**
   * Get content nodes by type.
   */
  getContentByType(contentType: string): Observable<ContentNode[]> {
    return from(this.fetchByType(contentType));
  }

  private async fetchByType(contentType: string): Promise<ContentNode[]> {
    await this.initialize();

    const result = await this.query<any>(`
      MATCH (c:ContentNode)
      WHERE c.contentType = "${contentType}"
      RETURN c.id, c.contentType, c.title, c.description, c.content, c.contentFormat,
             c.tags, c.reach
    `);

    return result.map((row: any) => ({
      id: row['c.id'],
      contentType: row['c.contentType'],
      title: row['c.title'],
      description: row['c.description'],
      content: row['c.content'],
      contentFormat: row['c.contentFormat'] || 'markdown',
      tags: row['c.tags'] || [],
      relatedNodeIds: [],
      reach: row['c.reach'] || 'commons',
      metadata: {}
    }));
  }

  /**
   * Clear caches.
   */
  clearCache(): void {
    this.pathCache.clear();
    this.contentCache.clear();
    this.graphCache$ = null;
  }

  /**
   * Close the database.
   */
  async close(): Promise<void> {
    if (this.db) {
      await this.db.close();
      this.db = null;
      this.ready$.next(false);
    }
  }
}
