import { Injectable } from '@angular/core';

import { map, tap, catchError, shareReplay, switchMap } from 'rxjs/operators';

import { Observable, of, forkJoin } from 'rxjs';

import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import {
  DataLoaderService,
  ClusterConnectionSummary,
} from '@app/elohim/services/data-loader.service';

import {
  ClusterNode,
  ClusterEdge,
  ClusterGraphData,
  ClusterExpansionResult,
  ClusterConnection,
  ClusterConnectionSummary as GraphClusterConnectionSummary,
  createChapterCluster,
  createModuleCluster,
  createSectionCluster,
  createConceptNode,
  calculateClusterRadius,
  CLUSTER_LEVEL_CONFIG,
} from '../models/cluster-graph.model';
import { ContentNode } from '../models/content-node.model';
import { LearningPath, PathChapter, PathModule, PathSection } from '../models/learning-path.model';

/**
 * LRU Cache for cluster data.
 */
class LRUCache<K, V> {
  private readonly cache = new Map<K, V>();
  private readonly maxSize: number;

  constructor(maxSize = 50) {
    this.maxSize = maxSize;
  }

  get(key: K): V | undefined {
    const value = this.cache.get(key);
    if (value !== undefined) {
      this.cache.delete(key);
      this.cache.set(key, value);
    }
    return value;
  }

  set(key: K, value: V): void {
    if (this.cache.has(key)) {
      this.cache.delete(key);
    } else if (this.cache.size >= this.maxSize) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey !== undefined) {
        this.cache.delete(firstKey);
      }
    }
    this.cache.set(key, value);
  }

  has(key: K): boolean {
    return this.cache.has(key);
  }

  values(): IterableIterator<V> {
    return this.cache.values();
  }

  clear(): void {
    this.cache.clear();
  }
}

/**
 * HierarchicalGraphService - Transforms learning path structure into cluster graph format.
 *
 * Responsibilities:
 * - Convert LearningPath hierarchy (chapters → modules → sections → concepts)
 *   into ClusterNode tree for D3 visualization
 * - Manage cluster expansion/collapse state
 * - Cache expanded cluster data with LRU eviction
 * - Compute aggregated metrics (completion %, external connections)
 * - Map conceptIds to cluster IDs for relationship aggregation
 */
@Injectable({ providedIn: 'root' })
export class HierarchicalGraphService {
  /** Currently expanded clusters */
  private readonly expandedClusters = new Set<string>();

  /** Cache for expanded cluster children */
  private readonly childrenCache = new LRUCache<string, ClusterNode[]>(50);

  /** Cache for cluster connections */
  private readonly connectionsCache = new LRUCache<string, ClusterConnection[]>(100);

  /** Map from conceptId to containing cluster ID (for relationship aggregation) */
  private readonly conceptToClusterMap = new Map<string, string>();

  /** Current graph data (cached observable) */
  private currentGraph$: Observable<ClusterGraphData> | null = null;
  private currentPathId: string | null = null;

  /** Snapshot of current graph for synchronous access */
  private currentGraphSnapshot: ClusterGraphData | null = null;

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly affinityService: AffinityTrackingService
  ) {}

  /**
   * Initialize cluster graph from a learning path.
   *
   * Loads the path hierarchy from Holochain and transforms it into
   * a ClusterGraphData structure with chapter-level clusters visible.
   *
   * @param pathId - Learning path ID (e.g., 'elohim-protocol')
   * @returns Observable of ClusterGraphData with root and chapter clusters
   */
  initializeFromPath(pathId: string): Observable<ClusterGraphData> {
    // Return cached graph if same path
    if (this.currentPathId === pathId && this.currentGraph$) {
      return this.currentGraph$;
    }

    this.currentPathId = pathId;
    this.expandedClusters.clear();
    this.conceptToClusterMap.clear();

    this.currentGraph$ = this.dataLoader.getPathHierarchy(pathId).pipe(
      map(path => this.transformPathToGraph(path)),
      tap(graph => {
        // Store snapshot for synchronous access
        this.currentGraphSnapshot = graph;
        // Build initial concept-to-cluster mapping for all concepts
        this.buildConceptToClusterMap(graph);
      }),
      shareReplay(1),
      catchError(err => {
        console.error('[HierarchicalGraphService] Failed to load path hierarchy:', err);
        return of(this.createEmptyGraph());
      })
    );

    return this.currentGraph$;
  }

  /**
   * Expand a cluster to show its children.
   *
   * - Chapter → shows modules
   * - Module → shows sections
   * - Section → shows concepts (loaded from Holochain)
   *
   * @param clusterId - ID of cluster to expand
   * @returns Observable of expansion result with new nodes and edges
   */
  expandCluster(clusterId: string): Observable<ClusterExpansionResult> {
    // Check cache first
    const cachedChildren = this.childrenCache.get(clusterId);
    if (cachedChildren) {
      this.expandedClusters.add(clusterId);
      return of({
        clusterId,
        children: cachedChildren,
        edges: this.createChildEdges(clusterId, cachedChildren),
        connections: [],
      });
    }

    // Need to load children based on cluster type
    if (!this.currentGraph$) {
      return of({ clusterId, children: [], edges: [], connections: [] });
    }

    return this.currentGraph$.pipe(
      switchMap(graph => {
        const cluster = graph.clusters.get(clusterId);
        if (!cluster) {
          return of({ clusterId, children: [], edges: [], connections: [] });
        }

        if (cluster.clusterType === 'section') {
          // Load concepts from Holochain
          return this.loadSectionConcepts(cluster);
        } else {
          // Children are already in the hierarchy (modules or sections)
          const children = this.getChildClusters(graph, clusterId);
          this.childrenCache.set(clusterId, children);
          this.expandedClusters.add(clusterId);

          return of({
            clusterId,
            children,
            edges: this.createChildEdges(clusterId, children),
            connections: [],
          });
        }
      })
    );
  }

  /**
   * Collapse a cluster to hide its children.
   */
  collapseCluster(clusterId: string): void {
    this.expandedClusters.delete(clusterId);
    // Also collapse any expanded descendants
    this.collapseDescendants(clusterId);
  }

  /**
   * Check if a cluster is currently expanded.
   */
  isExpanded(clusterId: string): boolean {
    return this.expandedClusters.has(clusterId);
  }

  /**
   * Get all currently visible nodes (expanded clusters and their visible children).
   */
  getVisibleNodes(graph: ClusterGraphData): ClusterNode[] {
    const visible: ClusterNode[] = [graph.root];

    const addVisibleChildren = (parentId: string) => {
      if (!this.expandedClusters.has(parentId)) return;

      const children = this.childrenCache.get(parentId) || this.getChildClusters(graph, parentId);

      for (const child of children) {
        visible.push(child);
        if (this.expandedClusters.has(child.id)) {
          addVisibleChildren(child.id);
        }
      }
    };

    // Start with root's children (chapters)
    addVisibleChildren(graph.root.id);
    // Also include chapters even if root isn't "expanded" - they're always visible
    const chapters = Array.from(graph.clusters.values()).filter(c => c.clusterType === 'chapter');
    for (const chapter of chapters) {
      if (!visible.includes(chapter)) {
        visible.push(chapter);
      }
      if (this.expandedClusters.has(chapter.id)) {
        addVisibleChildren(chapter.id);
      }
    }

    return visible;
  }

  /**
   * Get visible edges between visible nodes.
   * Includes both containment edges (parent → child) and progression edges (sibling → sibling).
   */
  getVisibleEdges(visibleNodeIds: Set<string>): ClusterEdge[] {
    const edges: ClusterEdge[] = [];

    // Group visible nodes by parent to find siblings
    const nodesByParent = new Map<string, ClusterNode[]>();

    for (const nodeId of visibleNodeIds) {
      // Get the node from cache or graph
      const node = this.findNode(nodeId);
      if (node?.parentClusterId) {
        const siblings = nodesByParent.get(node.parentClusterId) || [];
        siblings.push(node);
        nodesByParent.set(node.parentClusterId, siblings);
      }
    }

    // Create progression edges between siblings (sorted by order)
    for (const [_parentId, siblings] of nodesByParent) {
      // Sort by order
      const sorted = siblings.sort((a, b) => (a.order ?? 0) - (b.order ?? 0));

      // Create progression edges between consecutive siblings
      for (let i = 0; i < sorted.length - 1; i++) {
        const current = sorted[i];
        const next = sorted[i + 1];

        if (visibleNodeIds.has(current.id) && visibleNodeIds.has(next.id)) {
          edges.push({
            source: current.id,
            target: next.id,
            type: 'NEXT',
            isAggregated: false,
          });
        }
      }
    }

    return edges;
  }

  /**
   * Find a node by ID from graph or cache.
   */
  private findNode(nodeId: string): ClusterNode | undefined {
    // Check children cache first
    for (const children of this.childrenCache.values()) {
      const found = children.find(c => c.id === nodeId);
      if (found) return found;
    }

    // Check current graph
    if (this.currentGraphSnapshot) {
      return this.currentGraphSnapshot.clusters.get(nodeId);
    }

    return undefined;
  }

  /**
   * Get cluster connections for a cluster (aggregated relationships).
   *
   * @param clusterId - Cluster to get connections for
   * @returns Observable of connections to other clusters
   */
  getClusterConnections(clusterId: string): Observable<ClusterConnection[]> {
    // Check cache
    const cached = this.connectionsCache.get(clusterId);
    if (cached) {
      return of(cached);
    }

    if (!this.currentGraph$) {
      return of([]);
    }

    return this.currentGraph$.pipe(
      switchMap(graph => {
        const cluster = graph.clusters.get(clusterId);
        if (!cluster) {
          return of([]);
        }

        // Collect all concept IDs in this cluster (recursively)
        const conceptIds = this.collectConceptIds(cluster, graph);
        if (conceptIds.length === 0) {
          return of([]);
        }

        // Query connections from Holochain
        return this.dataLoader.getClusterConnections(conceptIds, this.conceptToClusterMap).pipe(
          map(summary => {
            const connections: ClusterConnection[] = [];

            // Convert outgoing connections
            for (const [targetId, conn] of summary.outgoingByCluster) {
              if (targetId !== clusterId) {
                // Skip self-connections
                connections.push({
                  sourceClusterId: clusterId,
                  targetClusterId: targetId,
                  connectionCount: conn.connectionCount,
                  relationshipTypes: conn.relationshipTypes,
                });
              }
            }

            this.connectionsCache.set(clusterId, connections);
            return connections;
          })
        );
      })
    );
  }

  /**
   * Clear all caches and reset state.
   */
  reset(): void {
    this.expandedClusters.clear();
    this.childrenCache.clear();
    this.currentGraphSnapshot = null;
    this.connectionsCache.clear();
    this.conceptToClusterMap.clear();
    this.currentGraph$ = null;
    this.currentPathId = null;
  }

  // =========================================================================
  // Private Methods
  // =========================================================================

  /**
   * Transform LearningPath into ClusterGraphData.
   */
  private transformPathToGraph(path: LearningPath): ClusterGraphData {
    const clusters = new Map<string, ClusterNode>();

    // Create root node for the path
    const root: ClusterNode = {
      id: path.id,
      title: path.title,
      description: path.description,
      contentType: 'path',
      isCluster: true,
      clusterType: 'path',
      clusterLevel: 0,
      parentClusterId: null,
      childClusterIds: [],
      conceptIds: [],
      isExpanded: true, // Root is always expanded
      isLoading: false,
      totalConceptCount: 0,
      completedConceptCount: 0,
      externalConnectionCount: 0,
      state: 'unseen',
      affinityScore: 0,
    };

    clusters.set(root.id, root);

    // Process chapters
    if (path.chapters) {
      for (const chapter of path.chapters) {
        const chapterCluster = this.processChapter(chapter, path.id, clusters);
        root.childClusterIds.push(chapterCluster.id);
        root.totalConceptCount += chapterCluster.totalConceptCount;
      }
    }

    // Calculate root affinity
    root.affinityScore = this.calculateClusterAffinity(root, clusters);
    root.state = this.determineClusterState(root);

    return {
      root,
      clusters,
      edges: [],
      connections: [],
    };
  }

  /**
   * Process a chapter and its modules/sections into clusters.
   */
  private processChapter(
    chapter: PathChapter,
    pathId: string,
    clusters: Map<string, ClusterNode>
  ): ClusterNode {
    // Count total concepts in chapter
    let totalConcepts = 0;
    const moduleIds: string[] = [];

    if (chapter.modules) {
      for (const module of chapter.modules) {
        for (const section of module.sections || []) {
          totalConcepts += section.conceptIds?.length || 0;
        }
      }
    }

    const chapterCluster = createChapterCluster(
      chapter,
      pathId,
      chapter.modules?.length || 0,
      totalConcepts
    );

    // Process modules
    if (chapter.modules) {
      for (const module of chapter.modules) {
        const moduleCluster = this.processModule(module, chapter.id, clusters);
        moduleIds.push(moduleCluster.id);
      }
    }

    chapterCluster.childClusterIds = moduleIds;
    chapterCluster.affinityScore = this.calculateClusterAffinity(chapterCluster, clusters);
    chapterCluster.state = this.determineClusterState(chapterCluster);
    chapterCluster.clusterRadius = calculateClusterRadius(chapterCluster);

    clusters.set(chapterCluster.id, chapterCluster);
    return chapterCluster;
  }

  /**
   * Process a module and its sections into clusters.
   */
  private processModule(
    module: PathModule,
    chapterId: string,
    clusters: Map<string, ClusterNode>
  ): ClusterNode {
    let totalConcepts = 0;
    const sectionIds: string[] = [];

    for (const section of module.sections || []) {
      totalConcepts += section.conceptIds?.length || 0;
    }

    const moduleCluster = createModuleCluster(
      module,
      chapterId,
      module.sections?.length || 0,
      totalConcepts
    );

    // Process sections
    for (const section of module.sections || []) {
      const sectionCluster = this.processSection(section, module.id, clusters);
      sectionIds.push(sectionCluster.id);
    }

    moduleCluster.childClusterIds = sectionIds;
    moduleCluster.affinityScore = this.calculateClusterAffinity(moduleCluster, clusters);
    moduleCluster.state = this.determineClusterState(moduleCluster);
    moduleCluster.clusterRadius = calculateClusterRadius(moduleCluster);

    clusters.set(moduleCluster.id, moduleCluster);
    return moduleCluster;
  }

  /**
   * Process a section into a cluster (leaf cluster containing conceptIds).
   */
  private processSection(
    section: PathSection,
    moduleId: string,
    clusters: Map<string, ClusterNode>
  ): ClusterNode {
    const sectionCluster = createSectionCluster(section, moduleId);

    sectionCluster.affinityScore = this.calculateClusterAffinity(sectionCluster, clusters);
    sectionCluster.state = this.determineClusterState(sectionCluster);
    sectionCluster.clusterRadius = calculateClusterRadius(sectionCluster);

    clusters.set(sectionCluster.id, sectionCluster);
    return sectionCluster;
  }

  /**
   * Load concepts for a section from Holochain.
   */
  private loadSectionConcepts(section: ClusterNode): Observable<ClusterExpansionResult> {
    if (!section.conceptIds || section.conceptIds.length === 0) {
      return of({ clusterId: section.id, children: [], edges: [], connections: [] });
    }

    return this.dataLoader.getClusterConcepts(section.conceptIds).pipe(
      map(contentMap => {
        const children: ClusterNode[] = [];

        for (const conceptId of section.conceptIds) {
          const content = contentMap.get(conceptId);
          const affinityScore = this.affinityService.getAffinity(conceptId);

          const conceptNode = createConceptNode(
            conceptId,
            content?.title || conceptId,
            content?.contentType || 'concept',
            section.id,
            this.determineNodeState(affinityScore),
            affinityScore
          );

          children.push(conceptNode);
        }

        this.childrenCache.set(section.id, children);
        this.expandedClusters.add(section.id);

        return {
          clusterId: section.id,
          children,
          edges: this.createChildEdges(section.id, children),
          connections: [],
        };
      }),
      catchError(err => {
        console.error('[HierarchicalGraphService] Failed to load section concepts:', err);
        return of({ clusterId: section.id, children: [], edges: [], connections: [] });
      })
    );
  }

  /**
   * Get child clusters for a parent from the graph.
   */
  private getChildClusters(graph: ClusterGraphData, parentId: string): ClusterNode[] {
    const parent = graph.clusters.get(parentId);
    if (!parent) return [];

    return parent.childClusterIds
      .map(id => graph.clusters.get(id))
      .filter((c): c is ClusterNode => c !== undefined);
  }

  /**
   * Create edges from parent to children.
   */
  private createChildEdges(parentId: string, children: ClusterNode[]): ClusterEdge[] {
    return children.map(child => ({
      source: parentId,
      target: child.id,
      type: 'CONTAINS',
      isAggregated: false,
    }));
  }

  /**
   * Collapse all descendants of a cluster.
   */
  private collapseDescendants(clusterId: string): void {
    const children = this.childrenCache.get(clusterId);
    if (!children) return;

    for (const child of children) {
      this.expandedClusters.delete(child.id);
      this.collapseDescendants(child.id);
    }
  }

  /**
   * Build the conceptId → clusterId mapping for relationship aggregation.
   */
  private buildConceptToClusterMap(graph: ClusterGraphData): void {
    for (const [clusterId, cluster] of graph.clusters) {
      // Map concepts to their containing section
      if (cluster.clusterType === 'section') {
        for (const conceptId of cluster.conceptIds) {
          this.conceptToClusterMap.set(conceptId, clusterId);
        }
      }
    }
  }

  /**
   * Collect all concept IDs in a cluster (recursively).
   */
  private collectConceptIds(cluster: ClusterNode, graph: ClusterGraphData): string[] {
    if (cluster.clusterType === 'section') {
      return cluster.conceptIds;
    }

    const conceptIds: string[] = [];
    for (const childId of cluster.childClusterIds) {
      const child = graph.clusters.get(childId);
      if (child) {
        conceptIds.push(...this.collectConceptIds(child, graph));
      }
    }
    return conceptIds;
  }

  /**
   * Calculate aggregate affinity score for a cluster.
   */
  private calculateClusterAffinity(
    cluster: ClusterNode,
    clusters: Map<string, ClusterNode>
  ): number {
    if (cluster.clusterType === 'section') {
      // Average affinity of concepts
      if (cluster.conceptIds.length === 0) return 0;

      let total = 0;
      for (const conceptId of cluster.conceptIds) {
        total += this.affinityService.getAffinity(conceptId);
      }
      return total / cluster.conceptIds.length;
    }

    // Average of child clusters
    if (cluster.childClusterIds.length === 0) return 0;

    let total = 0;
    for (const childId of cluster.childClusterIds) {
      const child = clusters.get(childId);
      if (child) {
        total += child.affinityScore;
      }
    }
    return total / cluster.childClusterIds.length;
  }

  /**
   * Determine cluster state based on affinity and completion.
   */
  private determineClusterState(cluster: ClusterNode): ClusterNode['state'] {
    const affinity = cluster.affinityScore;

    if (affinity >= 0.8) return 'proficient';
    if (affinity >= 0.4) return 'in-progress';
    if (affinity > 0) return 'in-progress';

    // Check if this is a recommended starting point
    if (cluster.clusterType === 'chapter' && cluster.order === 1) {
      return 'recommended';
    }

    return 'unseen';
  }

  /**
   * Determine node state based on affinity.
   */
  private determineNodeState(affinity: number): ClusterNode['state'] {
    if (affinity >= 0.8) return 'proficient';
    if (affinity >= 0.4) return 'in-progress';
    if (affinity > 0) return 'in-progress';
    return 'unseen';
  }

  /**
   * Create empty graph for error cases.
   */
  private createEmptyGraph(): ClusterGraphData {
    return {
      root: {
        id: 'empty',
        title: 'No Content',
        contentType: 'path',
        isCluster: true,
        clusterType: 'path',
        clusterLevel: 0,
        parentClusterId: null,
        childClusterIds: [],
        conceptIds: [],
        isExpanded: false,
        isLoading: false,
        totalConceptCount: 0,
        completedConceptCount: 0,
        externalConnectionCount: 0,
        state: 'unseen',
        affinityScore: 0,
      },
      clusters: new Map(),
      edges: [],
      connections: [],
    };
  }
}
