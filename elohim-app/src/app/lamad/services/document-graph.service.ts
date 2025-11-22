import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { BehaviorSubject, Observable, from, of } from 'rxjs';
import { map, catchError, tap, switchMap, mergeMap, toArray } from 'rxjs/operators';
import {
  ContentGraph,
  ContentNode,
  ContentRelationship,
  RelationshipType
} from '../models/content-node.model';
import { GherkinParser, MarkdownParser } from '../parsers';

/**
 * Service responsible for building and maintaining the documentation graph
 * Loads files, parses them, and builds relationships
 */
@Injectable({
  providedIn: 'root'
})
export class DocumentGraphService {
  private readonly graphSubject = new BehaviorSubject<ContentGraph | null>(null);
  public readonly graph$ = this.graphSubject.asObservable();

  private readonly DOCS_PATH = 'assets/docs';
  private readonly MANIFEST_PATH = 'assets/docs/manifest.json';

  constructor(private readonly http: HttpClient) {}

  /**
   * Initialize and build the documentation graph
   */
  buildGraph(): Observable<ContentGraph> {
    return this.loadContentFiles().pipe(
      map(nodes => {
        const graph = this.createEmptyGraph();

        // Add all nodes to graph
        nodes.forEach(node => {
          this.addNodeToGraph(graph, node);
        });

        // Build relationships
        this.buildRelationships(graph);

        // Update metadata
        this.updateGraphMetadata(graph);

        // Cache the graph
        this.graphSubject.next(graph);

        return graph;
      }),
      tap(graph => {
        console.log('Content graph built:', {
          nodes: graph.nodes.size,
          relationships: graph.relationships.size,
          types: Array.from(graph.nodesByType.keys())
        });
      })
    );
  }

  /**
   * Get the current graph
   */
  getGraph(): ContentGraph | null {
    return this.graphSubject.value;
  }

  /**
   * Get a node by ID
   */
  getNode(id: string): ContentNode | undefined {
    return this.graphSubject.value?.nodes.get(id);
  }

  /**
   * Get all nodes of a specific type
   */
  getNodesByType(type: string): ContentNode[] {
    const graph = this.graphSubject.value;
    if (!graph || !graph.nodesByType.has(type)) return [];

    const ids = graph.nodesByType.get(type)!;
    return Array.from(ids)
      .map(id => graph.nodes.get(id))
      .filter((node): node is ContentNode => node !== undefined);
  }

  /**
   * Get related nodes for a given node ID
   */
  getRelatedNodes(nodeId: string): ContentNode[] {
    const graph = this.graphSubject.value;
    if (!graph) return [];

    const relatedIds = graph.adjacency.get(nodeId);
    if (!relatedIds) return [];

    return Array.from(relatedIds)
      .map(id => graph.nodes.get(id))
      .filter((node): node is ContentNode => node !== undefined);
  }

  /**
   * Search nodes by text
   */
  searchNodes(query: string): ContentNode[] {
    const graph = this.graphSubject.value;
    if (!graph) return [];

    const lowerQuery = query.toLowerCase();
    const results: ContentNode[] = [];

    graph.nodes.forEach(node => {
      const matchScore = this.calculateMatchScore(node, lowerQuery);
      if (matchScore > 0) {
        results.push(node);
      }
    });

    return results.sort((a, b) => {
      const scoreA = this.calculateMatchScore(a, lowerQuery);
      const scoreB = this.calculateMatchScore(b, lowerQuery);
      return scoreB - scoreA;
    });
  }

  /**
   * Load all content files from manifest
   */
  private loadContentFiles(): Observable<ContentNode[]> {
    return this.http.get<{ files: { path: string; type: string }[] }>(this.MANIFEST_PATH).pipe(
      switchMap(manifest => {
        if (!manifest || !manifest.files) {
            console.warn('Manifest empty or invalid');
            return of([]);
        }

        return from(manifest.files).pipe(
          mergeMap(file =>
            this.http.get(`${this.DOCS_PATH}/${file.path}`, { responseType: 'text' }).pipe(
              map(content => {
                const fullPath = file.path;
                const category = this.inferCategoryFromPath(fullPath);

                if (file.type === 'feature' || fullPath.endsWith('.feature')) {
                  const result = GherkinParser.parseFeature(content, fullPath, category);
                  // Return array of [FeatureNode, ...ScenarioNodes]
                  return [result.feature, ...result.scenarios];
                } else {
                  // Default to markdown parser
                  const node = MarkdownParser.parseContent(content, fullPath);
                  // Ensure type from manifest overrides or complements parser inference if needed
                  if (file.type && file.type !== 'epic' && node.contentType === 'epic') {
                      node.contentType = file.type;
                  }
                  return [node];
                }
              }),
              catchError(err => {
                console.error(`Failed to load file: ${file.path}`, err);
                return of([] as ContentNode[]);
              })
            ),
            5 // Concurrency limit
          ),
          toArray(),
          map(results => results.flat())
        );
      })
    );
  }

  private inferCategoryFromPath(path: string): string {
    const parts = path.split('/');
    if (parts.length > 1) {
        // e.g. "autonomous_entity/community_investor/..." -> "autonomous-entity"
        return parts[0].replace(/_/g, '-');
    }
    return 'general';
  }


  /**
   * Create an empty graph structure
   */
  private createEmptyGraph(): ContentGraph {
    return {
      nodes: new Map(),
      relationships: new Map(),
      nodesByType: new Map(),
      nodesByTag: new Map(),
      nodesByCategory: new Map(),
      adjacency: new Map(),
      reverseAdjacency: new Map(),
      metadata: {
        nodeCount: 0,
        relationshipCount: 0,
        lastUpdated: new Date(),
        version: '1.0'
      }
    };
  }

  /**
   * Add a node to the graph with all indices
   */
  private addNodeToGraph(graph: ContentGraph, node: ContentNode): void {
    // Add to main nodes map
    graph.nodes.set(node.id, node);

    // Add to type index
    if (!graph.nodesByType.has(node.contentType)) {
      graph.nodesByType.set(node.contentType, new Set());
    }
    graph.nodesByType.get(node.contentType)!.add(node.id);

    // Add to tag index
    node.tags.forEach(tag => {
      if (!graph.nodesByTag.has(tag)) {
        graph.nodesByTag.set(tag, new Set());
      }
      graph.nodesByTag.get(tag)!.add(node.id);
    });

    // Add to category index
    const category = node.metadata?.['category'];
    if (category) {
      if (!graph.nodesByCategory.has(category)) {
        graph.nodesByCategory.set(category, new Set());
      }
      graph.nodesByCategory.get(category)!.add(node.id);
    }

    // Initialize adjacency lists
    if (!graph.adjacency.has(node.id)) {
      graph.adjacency.set(node.id, new Set());
    }
    if (!graph.reverseAdjacency.has(node.id)) {
      graph.reverseAdjacency.set(node.id, new Set());
    }
  }

  /**
   * Build relationships between nodes
   */
  private buildRelationships(graph: ContentGraph): void {
    graph.nodes.forEach(node => {
        // 1. Explicit relatedNodeIds
        if (node.relatedNodeIds && node.relatedNodeIds.length > 0) {
            node.relatedNodeIds.forEach(targetId => this.linkNodes(graph, node.id, targetId, RelationshipType.RELATES_TO));
        }
        
        // 2. Metadata-based relationships
        if (node.metadata) {
            const meta = node.metadata;
            
            // related_users (e.g. ['worker', 'customer']) -> User Types
            if (Array.isArray(meta['related_users'])) {
                meta['related_users'].forEach((userType: string) => this.linkNodes(graph, node.id, userType, RelationshipType.RELATES_TO));
            }
            
            // related_epics -> Epics
            if (Array.isArray(meta['related_epics'])) {
                meta['related_epics'].forEach((epicId: string) => this.linkNodes(graph, node.id, epicId, RelationshipType.RELATES_TO));
            }
            
            // related_layers -> Epics/Layers
            if (Array.isArray(meta['related_layers'])) {
                meta['related_layers'].forEach((layerId: string) => this.linkNodes(graph, node.id, layerId, RelationshipType.RELATES_TO));
            }
            
            // primary_epic -> Epic (Belongs To)
            if (meta['primary_epic']) {
                this.linkNodes(graph, node.id, meta['primary_epic'], RelationshipType.BELONGS_TO);
            }
            
            // governance_scope -> Layers
            if (Array.isArray(meta['governance_scope'])) {
                 meta['governance_scope'].forEach((scopeId: string) => this.linkNodes(graph, node.id, scopeId, RelationshipType.RELATES_TO));
            }
            
             // epic -> Epic (Belongs To) - specific for user types
            if (meta['epic']) {
                this.linkNodes(graph, node.id, meta['epic'], RelationshipType.BELONGS_TO);
            }
        }
    });
  }

  /**
   * Helper to safely link nodes if target exists
   */
  private linkNodes(graph: ContentGraph, sourceId: string, targetIdRaw: string, type: RelationshipType): void {
     if (!targetIdRaw) return;
     const targetId = targetIdRaw.trim();
     
     // Try exact match
     if (graph.nodes.has(targetId)) {
         this.createRelationship(graph, sourceId, targetId, type);
         return;
     }
     
     // Try "epic_" prefix if target looks like an epic name but wasn't found
     // (Since we normalize epic IDs to just the name now via generateNodeId, this might not be needed if names match)
     // But let's try a fallback search just in case
     // e.g. 'governance' -> 'governance' (exact)
     // e.g. 'community' -> 'community' (exact)
     
     // Try finding by title (case-insensitive) if ID match fails
     // This is expensive (O(N)), but graph is small (<1000 nodes)
     // Optimized: Iterate once? No, just do it.
     for (const [id, node] of graph.nodes.entries()) {
         if (node.title.toLowerCase() === targetId.toLowerCase()) {
             this.createRelationship(graph, sourceId, id, type);
             return;
         }
     }
  }
  
  private createRelationship(graph: ContentGraph, sourceId: string, targetId: string, type: RelationshipType): void {
      const id = `${sourceId}_${type}_${targetId}`;
      if (!graph.relationships.has(id)) {
          this.addRelationship(graph, {
            id,
            sourceNodeId: sourceId,
            targetNodeId: targetId,
            relationshipType: type
          });
          
          // Add reverse relationship for navigation? 
          // RELATES_TO is usually bidirectional in UI navigation
          if (type === RelationshipType.RELATES_TO) {
              const reverseId = `${targetId}_${type}_${sourceId}`;
              if (!graph.relationships.has(reverseId)) {
                  this.addRelationship(graph, {
                    id: reverseId,
                    sourceNodeId: targetId,
                    targetNodeId: sourceId,
                    relationshipType: type
                  });
              }
          }
      }
  }

  /**
   * Add a single relationship to graph
   */
  private addRelationship(graph: ContentGraph, relationship: ContentRelationship): void {
    graph.relationships.set(relationship.id, relationship);

    // Update adjacency lists
    if (!graph.adjacency.has(relationship.sourceNodeId)) {
      graph.adjacency.set(relationship.sourceNodeId, new Set());
    }
    graph.adjacency.get(relationship.sourceNodeId)!.add(relationship.targetNodeId);

    if (!graph.reverseAdjacency.has(relationship.targetNodeId)) {
      graph.reverseAdjacency.set(relationship.targetNodeId, new Set());
    }
    graph.reverseAdjacency.get(relationship.targetNodeId)!.add(relationship.sourceNodeId);
  }

  /**
   * Update graph metadata
   */
  private updateGraphMetadata(graph: ContentGraph): void {
    graph.metadata.nodeCount = graph.nodes.size;
    graph.metadata.relationshipCount = graph.relationships.size;
    graph.metadata.lastUpdated = new Date();
  }

  /**
   * Calculate match score for search
   */
  private calculateMatchScore(node: ContentNode, query: string): number {
    let score = 0;

    if (node.title.toLowerCase().includes(query)) score += 100;
    if (node.description && node.description.toLowerCase().includes(query)) score += 50;
    if (node.content.toLowerCase().includes(query)) score += 10;
    if (node.tags.some(tag => tag.toLowerCase().includes(query))) score += 75;

    return score;
  }
}
