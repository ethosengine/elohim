import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';
import { ContentNode } from '../models/content-node.model';
import { DocumentGraphService } from './document-graph.service';
import { DocumentNodeAdapter } from '../adapters/document-node.adapter';

export interface PathNode {
  node: ContentNode;
  order: number;
  depth: number;
  category: string;
}

/**
 * Service for managing the suggested learning path through content.
 * Defines the default order for exploring the Elohim Protocol.
 */
@Injectable({
  providedIn: 'root'
})
export class LearningPathService {
  private readonly pathSubject = new BehaviorSubject<PathNode[]>([]);
  public readonly path$ = this.pathSubject.asObservable();

  /**
   * Default learning path order - the suggested sequence for understanding Elohim
   * This is the "pre-authored path" mentioned in claude.md
   */
  private readonly DEFAULT_PATH_ORDER = [
    // Core vision and concepts (epics)
    { id: 'manifesto.md', category: 'vision', depth: 0 },
    { id: 'elohim-observer-protocol.md', category: 'core', depth: 0 },
    { id: 'elohim-public-observer-epic.md', category: 'core', depth: 0 },
    { id: 'elohim-value-scanner-protocol.md', category: 'core', depth: 0 },
    { id: 'elohim-social-medium.md', category: 'core', depth: 0 },
    { id: 'elohim-autonomous-entity-story.md', category: 'advanced', depth: 0 },
    { id: 'elohim-global-orchestra.md', category: 'systemic', depth: 0 },
    { id: 'elohim_hardware_spec.md', category: 'implementation', depth: 0 },

    // Technical features
    { id: 'core/platform-architecture.feature', category: 'technical', depth: 1 },
    { id: 'value-scanner/care-economy.feature', category: 'technical', depth: 1 },
    { id: 'deployment/staging-validation.feature', category: 'technical', depth: 1 },
  ];

  constructor(private readonly graphService: DocumentGraphService) {
    this.initializePath();
  }

  /**
   * Initialize the learning path from the graph
   */
  private initializePath(): void {
    this.graphService.graph$.subscribe(graph => {
      if (!graph) return;

      const pathNodes: PathNode[] = [];

      this.DEFAULT_PATH_ORDER.forEach((pathItem, index) => {
        // Find node by matching the ID pattern
        const node = Array.from(graph.nodes.values()).find(n =>
          n.id.includes(pathItem.id) || n.id === pathItem.id
        );

        if (node) {
          const contentNode = DocumentNodeAdapter.fromDocumentNode(node);
          pathNodes.push({
            node: contentNode,
            order: index,
            depth: pathItem.depth,
            category: pathItem.category
          });
        }
      });

      this.pathSubject.next(pathNodes);
    });
  }

  /**
   * Get the full learning path
   */
  getPath(): PathNode[] {
    return this.pathSubject.value;
  }

  /**
   * Get the next node in the path after the given node ID
   */
  getNextNode(currentNodeId: string): PathNode | null {
    const path = this.pathSubject.value;
    const currentIndex = path.findIndex(pn => pn.node.id === currentNodeId);

    if (currentIndex >= 0 && currentIndex < path.length - 1) {
      return path[currentIndex + 1];
    }

    return null;
  }

  /**
   * Get the previous node in the path before the given node ID
   */
  getPreviousNode(currentNodeId: string): PathNode | null {
    const path = this.pathSubject.value;
    const currentIndex = path.findIndex(pn => pn.node.id === currentNodeId);

    if (currentIndex > 0) {
      return path[currentIndex - 1];
    }

    return null;
  }

  /**
   * Get the position of a node in the path (0-indexed)
   */
  getNodePosition(nodeId: string): number {
    const path = this.pathSubject.value;
    return path.findIndex(pn => pn.node.id === nodeId);
  }

  /**
   * Check if a node is in the suggested path
   */
  isInPath(nodeId: string): boolean {
    return this.getNodePosition(nodeId) >= 0;
  }

  /**
   * Get path progress (percentage completed based on affinity)
   */
  getPathProgress(affinityMap: Map<string, number>): number {
    const path = this.pathSubject.value;
    if (path.length === 0) return 0;

    const engagedCount = path.filter(pn =>
      (affinityMap.get(pn.node.id) ?? 0) > 0
    ).length;

    return (engagedCount / path.length) * 100;
  }
}
