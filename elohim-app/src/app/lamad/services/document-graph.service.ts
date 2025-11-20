import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { BehaviorSubject, Observable, forkJoin, of } from 'rxjs';
import { map, catchError, tap, switchMap } from 'rxjs/operators';
import {
  DocumentGraph,
  DocumentNode,
  EpicNode,
  FeatureNode,
  ScenarioNode,
  NodeRelationship,
  RelationshipType,
  createBidirectionalRelationship
} from '../models';
import { GherkinParser, MarkdownParser } from '../parsers';

/**
 * Service responsible for building and maintaining the documentation graph
 * Loads files, parses them, and builds relationships
 */
@Injectable({
  providedIn: 'root'
})
export class DocumentGraphService {
  private readonly graphSubject = new BehaviorSubject<DocumentGraph | null>(null);
  public readonly graph$ = this.graphSubject.asObservable();

  private readonly EPIC_PATH = 'assets/docs';
  private readonly FEATURE_PATH = 'assets/features';

  // Manifest files listing available documents
  private readonly EPIC_MANIFEST = 'assets/docs/manifest.json';
  private readonly FEATURE_MANIFEST = 'assets/features/manifest.json';

  constructor(private readonly http: HttpClient) {}

  /**
   * Initialize and build the documentation graph
   */
  buildGraph(): Observable<DocumentGraph> {
    return forkJoin({
      epics: this.loadEpicFiles(),
      features: this.loadFeatureFiles()
    }).pipe(
      map(({ epics, features }) => {
        const graph = this.createEmptyGraph();

        // Add epics to graph
        epics.forEach(epic => {
          this.addNodeToGraph(graph, epic);
        });

        // Add features and scenarios to graph
        features.forEach(({ feature, scenarios }) => {
          this.addNodeToGraph(graph, feature);
          scenarios.forEach(scenario => {
            this.addNodeToGraph(graph, scenario);
          });
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
        console.log('Documentation graph built:', {
          nodes: graph.nodes.size,
          relationships: graph.relationships.size,
          epics: graph.nodesByType.epics.size,
          features: graph.nodesByType.features.size,
          scenarios: graph.nodesByType.scenarios.size
        });
      })
    );
  }

  /**
   * Get the current graph
   */
  getGraph(): DocumentGraph | null {
    return this.graphSubject.value;
  }

  /**
   * Get a node by ID
   */
  getNode(id: string): DocumentNode | undefined {
    return this.graphSubject.value?.nodes.get(id);
  }

  /**
   * Get all nodes of a specific type
   */
  getNodesByType(type: 'epic' | 'feature' | 'scenario'): DocumentNode[] {
    const graph = this.graphSubject.value;
    if (!graph) return [];

    if (type === 'epic') {
      return Array.from(graph.nodesByType.epics.values());
    } else if (type === 'feature') {
      return Array.from(graph.nodesByType.features.values());
    } else {
      return Array.from(graph.nodesByType.scenarios.values());
    }
  }

  /**
   * Get related nodes for a given node ID
   */
  getRelatedNodes(nodeId: string): DocumentNode[] {
    const graph = this.graphSubject.value;
    if (!graph) return [];

    const relatedIds = graph.adjacency.get(nodeId);
    if (!relatedIds) return [];

    return Array.from(relatedIds)
      .map(id => graph.nodes.get(id))
      .filter((node): node is DocumentNode => node !== undefined);
  }

  /**
   * Search nodes by text
   */
  searchNodes(query: string): DocumentNode[] {
    const graph = this.graphSubject.value;
    if (!graph) return [];

    const lowerQuery = query.toLowerCase();
    const results: DocumentNode[] = [];

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
   * Load epic markdown files
   */
  private loadEpicFiles(): Observable<EpicNode[]> {
    return this.http.get<{ files: string[] }>(this.EPIC_MANIFEST).pipe(
      switchMap(manifest => {
        const fileObservables = manifest.files.map(file =>
          this.http.get(`${this.EPIC_PATH}/${file}`, { responseType: 'text' }).pipe(
            map(content => MarkdownParser.parseEpic(content, `docs/${file}`)),
            catchError(err => {
              console.error(`Failed to load epic: ${file}`, err);
              return of(null);
            })
          )
        );
        return forkJoin(fileObservables);
      }),
      map(results => results.filter((epic): epic is EpicNode => epic !== null))
    );
  }

  /**
   * Load feature files
   */
  private loadFeatureFiles(): Observable<Array<{ feature: FeatureNode; scenarios: ScenarioNode[] }>> {
    return this.http.get<{ files: { path: string; category: string }[] }>(this.FEATURE_MANIFEST).pipe(
      switchMap(manifest => {
        const fileObservables = manifest.files.map(({ path, category }) =>
          this.http.get(`${this.FEATURE_PATH}/${path}`, { responseType: 'text' }).pipe(
            map(content => GherkinParser.parseFeature(content, path, category)),
            catchError(err => {
              console.error(`Failed to load feature: ${path}`, err);
              return of(null);
            })
          )
        );
        return forkJoin(fileObservables);
      }),
      map(results =>
        results.filter(
          (result): result is { feature: FeatureNode; scenarios: ScenarioNode[] } => result !== null
        )
      )
    );
  }

  /**
   * Create an empty graph structure
   */
  private createEmptyGraph(): DocumentGraph {
    return {
      nodes: new Map(),
      relationships: new Map(),
      nodesByType: {
        epics: new Map(),
        features: new Map(),
        scenarios: new Map()
      },
      nodesByTag: new Map(),
      nodesByCategory: new Map(),
      adjacency: new Map(),
      reverseAdjacency: new Map(),
      metadata: {
        nodeCount: 0,
        relationshipCount: 0,
        lastBuilt: new Date(),
        sources: {
          epicPath: this.EPIC_PATH,
          featurePath: this.FEATURE_PATH
        },
        stats: {
          epicCount: 0,
          featureCount: 0,
          scenarioCount: 0,
          averageConnectionsPerNode: 0
        }
      }
    };
  }

  /**
   * Add a node to the graph with all indices
   */
  private addNodeToGraph(graph: DocumentGraph, node: DocumentNode): void {
    // Add to main nodes map
    graph.nodes.set(node.id, node);

    // Add to type index
    switch (node.type) {
      case 'epic':
        graph.nodesByType.epics.set(node.id, node as EpicNode);
        break;
      case 'feature':
        graph.nodesByType.features.set(node.id, node as FeatureNode);
        break;
      case 'scenario':
        graph.nodesByType.scenarios.set(node.id, node as ScenarioNode);
        break;
    }

    // Add to tag index
    node.tags.forEach(tag => {
      if (!graph.nodesByTag.has(tag)) {
        graph.nodesByTag.set(tag, new Set());
      }
      graph.nodesByTag.get(tag)!.add(node.id);
    });

    // Add to category index
    const category = (node as any).category;
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
  private buildRelationships(graph: DocumentGraph): void {
    // Epic -> Feature relationships
    graph.nodesByType.epics.forEach(epic => {
      epic.featureIds.forEach(featureId => {
        if (graph.nodes.has(featureId)) {
          this.addRelationships(
            graph,
            createBidirectionalRelationship(
              RelationshipType.DESCRIBES,
              epic.id,
              featureId,
              `Epic describes feature implementation`
            )
          );
        }
      });
    });

    // Feature -> Scenario relationships
    graph.nodesByType.features.forEach(feature => {
      feature.scenarioIds.forEach(scenarioId => {
        if (graph.nodes.has(scenarioId)) {
          this.addRelationship(graph, {
            id: `${feature.id}_${scenarioId}_contains`,
            type: RelationshipType.BELONGS_TO,
            sourceId: scenarioId,
            targetId: feature.id,
            bidirectional: false,
            description: 'Scenario belongs to feature'
          });
        }
      });
    });

    // Scenario -> Epic relationships (via tags)
    graph.nodesByType.scenarios.forEach(scenario => {
      scenario.epicIds.forEach(epicId => {
        if (graph.nodes.has(epicId)) {
          this.addRelationship(graph, {
            id: `${scenario.id}_${epicId}_validates`,
            type: RelationshipType.VALIDATES,
            sourceId: scenario.id,
            targetId: epicId,
            bidirectional: false,
            description: 'Scenario validates epic'
          });
        }
      });
    });

    // Epic -> Epic relationships
    graph.nodesByType.epics.forEach(epic => {
      epic.relatedEpicIds.forEach(relatedEpicId => {
        if (graph.nodes.has(relatedEpicId)) {
          this.addRelationship(graph, {
            id: `${epic.id}_${relatedEpicId}_references`,
            type: RelationshipType.REFERENCES,
            sourceId: epic.id,
            targetId: relatedEpicId,
            bidirectional: false,
            description: 'Epic references related epic'
          });
        }
      });
    });
  }

  /**
   * Add multiple relationships to graph
   */
  private addRelationships(graph: DocumentGraph, relationships: NodeRelationship[]): void {
    relationships.forEach(rel => this.addRelationship(graph, rel));
  }

  /**
   * Add a single relationship to graph
   */
  private addRelationship(graph: DocumentGraph, relationship: NodeRelationship): void {
    graph.relationships.set(relationship.id, relationship);

    // Update adjacency lists
    if (!graph.adjacency.has(relationship.sourceId)) {
      graph.adjacency.set(relationship.sourceId, new Set());
    }
    graph.adjacency.get(relationship.sourceId)!.add(relationship.targetId);

    if (!graph.reverseAdjacency.has(relationship.targetId)) {
      graph.reverseAdjacency.set(relationship.targetId, new Set());
    }
    graph.reverseAdjacency.get(relationship.targetId)!.add(relationship.sourceId);
  }

  /**
   * Update graph metadata
   */
  private updateGraphMetadata(graph: DocumentGraph): void {
    graph.metadata.nodeCount = graph.nodes.size;
    graph.metadata.relationshipCount = graph.relationships.size;
    graph.metadata.lastBuilt = new Date();
    graph.metadata.stats = {
      epicCount: graph.nodesByType.epics.size,
      featureCount: graph.nodesByType.features.size,
      scenarioCount: graph.nodesByType.scenarios.size,
      averageConnectionsPerNode:
        graph.nodes.size > 0
          ? Array.from(graph.adjacency.values()).reduce((sum, set) => sum + set.size, 0) / graph.nodes.size
          : 0
    };
  }

  /**
   * Calculate match score for search
   */
  private calculateMatchScore(node: DocumentNode, query: string): number {
    let score = 0;

    if (node.title.toLowerCase().includes(query)) score += 100;
    if (node.description.toLowerCase().includes(query)) score += 50;
    if (node.content.toLowerCase().includes(query)) score += 10;
    if (node.tags.some(tag => tag.toLowerCase().includes(query))) score += 75;

    return score;
  }
}
