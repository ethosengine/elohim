import { Injectable } from '@angular/core';
import { Router, ActivatedRoute, NavigationEnd } from '@angular/router';
import { BehaviorSubject, Observable } from 'rxjs';
import { filter, map } from 'rxjs/operators';
import { DocumentGraphService } from './document-graph.service';
import { DocumentNode, EpicNode, FeatureNode, ScenarioNode } from '../models';

/**
 * Represents a single segment in the composite identifier path
 * Pattern: type:id:type:id:type:id...
 */
export interface PathSegment {
  /** Node type (epic, feature, scenario, etc.) */
  type: string;
  /** Node ID */
  id: string;
  /** The actual node (null for collection views) */
  node: DocumentNode | null;
  /** Composite URL segment: "type:id" */
  urlSegment: string;
}

/**
 * View mode based on composite identifier pattern
 */
export enum ViewMode {
  /** Home page (no segments) */
  HOME = 'home',
  /** Node view: shows content + collections (even segments: type:id pairs) */
  NODE = 'node',
  /** Collection view: shows list of items (odd segments: ends with type) */
  COLLECTION = 'collection'
}

/**
 * Context for the current navigation state
 */
export interface NavigationContext {
  /** Full path from root to current position */
  pathSegments: PathSegment[];

  /** View mode (home, node, or collection) */
  viewMode: ViewMode;

  /** Current node being viewed (null for collection views) */
  currentNode: DocumentNode | null;

  /** For collection views: the type of items to display */
  collectionType: string | null;

  /** Children/items to display (either node's children or collection items) */
  children: DocumentNode[];

  /** Parent node (null if at root) */
  parent: DocumentNode | null;

  /** Query parameters for additional context */
  queryParams: {
    /** Target subject for orientation */
    target?: string;
    /** Attestation journey tracking */
    attestation?: string;
    /** Step in suggested path */
    step?: number;
    /** Source node (for breadcrumb) */
    from?: string;
    /** Graph traversal depth */
    depth?: number;
  };
}

/**
 * Service managing hierarchical navigation through the graph
 */
@Injectable({
  providedIn: 'root'
})
export class NavigationService {
  private readonly contextSubject = new BehaviorSubject<NavigationContext | null>(null);
  public readonly context$ = this.contextSubject.asObservable();

  constructor(
    private readonly router: Router,
    private readonly graphService: DocumentGraphService
  ) {
    this.initializeNavigationTracking();
  }

  /**
   * Navigate to a node using composite identifier
   * @param nodeType - Type of node (epic, feature, scenario)
   * @param nodeId - ID of the node
   * @param options - Optional parent path and query params
   */
  navigateTo(
    nodeType: string,
    nodeId: string,
    options?: {
      parentPath?: string; // Composite path like "epic:social-medium:feature:affinity"
      queryParams?: Record<string, any>;
    }
  ): void {
    const composite = options?.parentPath
      ? `${options.parentPath}:${nodeType}:${nodeId}`
      : `${nodeType}:${nodeId}`;

    const urlPath = `/lamad/${composite}`;

    this.router.navigate([urlPath], {
      queryParams: options?.queryParams || {}
    });
  }

  /**
   * Navigate to a collection view
   * @param collectionType - Type of items in collection (feature, scenario, etc.)
   * @param parentPath - Parent composite path
   */
  navigateToCollection(
    collectionType: string,
    parentPath?: string,
    queryParams?: Record<string, any>
  ): void {
    const composite = parentPath ? `${parentPath}:${collectionType}` : collectionType;
    const urlPath = `/lamad/${composite}`;

    this.router.navigate([urlPath], {
      queryParams: queryParams || {}
    });
  }

  /**
   * Navigate to home (epics list)
   */
  navigateToHome(): void {
    this.router.navigate(['/lamad']);
  }

  /**
   * Navigate up one level in the hierarchy
   */
  navigateUp(): void {
    const context = this.contextSubject.value;
    if (!context) {
      this.navigateToHome();
      return;
    }

    // If in collection view, go back to parent node
    if (context.viewMode === ViewMode.COLLECTION) {
      if (context.pathSegments.length === 0) {
        this.navigateToHome();
      } else {
        const compositePath = context.pathSegments.map(s => s.urlSegment).join(':');
        this.router.navigate([`/lamad/${compositePath}`], {
          queryParams: context.queryParams
        });
      }
      return;
    }

    // If in node view, go up one node level
    if (context.pathSegments.length === 0) {
      this.navigateToHome();
      return;
    }

    if (context.pathSegments.length === 1) {
      this.navigateToHome();
      return;
    }

    const parentSegments = context.pathSegments.slice(0, -1);
    const compositePath = parentSegments.map(s => s.urlSegment).join(':');

    this.router.navigate([`/lamad/${compositePath}`], {
      queryParams: context.queryParams
    });
  }

  /**
   * Get the current navigation context
   */
  getCurrentContext(): NavigationContext | null {
    return this.contextSubject.value;
  }

  /**
   * Parse composite identifier URL into navigation context
   * Pattern: epic:id:feature:id:scenario:id or epic:id:feature (collection view)
   */
  parsePathSegments(urlPath: string, queryParams: any = {}): NavigationContext | null {
    const graph = this.graphService.getGraph();
    if (!graph) return null;

    // If no path, we're at home - show epics
    if (!urlPath || urlPath.length === 0) {
      return {
        pathSegments: [],
        viewMode: ViewMode.HOME,
        currentNode: null,
        collectionType: null,
        children: Array.from(graph.nodesByType.epics.values()),
        parent: null,
        queryParams
      };
    }

    // Parse composite identifier: "epic:social-medium:feature:affinity:scenario"
    const parts = urlPath.split(':');

    // Determine view mode
    const isCollectionView = parts.length % 2 === 1; // Odd number = collection view
    const viewMode = isCollectionView ? ViewMode.COLLECTION : ViewMode.NODE;

    // Build path segments (type:id pairs)
    const pathSegments: PathSegment[] = [];
    let currentNode: DocumentNode | null = null;
    let parent: DocumentNode | null = null;

    // Process type:id pairs
    for (let i = 0; i < parts.length - (isCollectionView ? 1 : 0); i += 2) {
      const type = parts[i];
      const id = parts[i + 1];

      if (!id) {
        console.warn(`Invalid composite identifier: missing ID for type ${type}`);
        return null;
      }

      const node = this.findNodeById(id);
      if (!node) {
        console.warn(`Node not found: ${type}:${id}`);
        return null;
      }

      // Validate node type matches
      if (node.type !== type) {
        console.warn(`Type mismatch: expected ${type}, got ${node.type} for ID ${id}`);
        return null;
      }

      pathSegments.push({
        type,
        id,
        node,
        urlSegment: `${type}:${id}`
      });

      parent = currentNode;
      currentNode = node;
    }

    // Determine what to display
    let children: DocumentNode[] = [];
    let collectionType: string | null = null;

    if (isCollectionView) {
      // Collection view: show all items of the specified type under current node
      collectionType = parts[parts.length - 1];
      children = this.getChildrenOfType(currentNode, collectionType);
    } else {
      // Node view: show content + available collections
      children = this.getChildren(currentNode!);
    }

    return {
      pathSegments,
      viewMode,
      currentNode,
      collectionType,
      children,
      parent,
      queryParams
    };
  }

  /**
   * Get children of a node based on its type and relationships
   * Returns all types of children (features, scenarios, etc.)
   */
  private getChildren(node: DocumentNode): DocumentNode[] {
    const graph = this.graphService.getGraph();
    if (!graph) return [];

    switch (node.type) {
      case 'epic': {
        const epic = node as EpicNode;
        // Epic children are its features
        return epic.featureIds
          .map(id => graph.nodes.get(id))
          .filter((n): n is DocumentNode => n !== undefined);
      }

      case 'feature': {
        const feature = node as FeatureNode;
        // Feature children are its scenarios
        return feature.scenarioIds
          .map(id => graph.nodes.get(id))
          .filter((n): n is DocumentNode => n !== undefined);
      }

      case 'scenario':
        // Scenarios typically don't have children (leaf nodes)
        // But could have related content in the future
        return [];

      default:
        return [];
    }
  }

  /**
   * Get children of a specific type under a node (for collection views)
   * @param node - Parent node (null for root-level collections like all epics)
   * @param childType - Type of children to get (epic, feature, scenario)
   */
  private getChildrenOfType(node: DocumentNode | null, childType: string): DocumentNode[] {
    const graph = this.graphService.getGraph();
    if (!graph) return [];

    // Root-level collection (e.g., /lamad/epic)
    if (!node) {
      switch (childType) {
        case 'epic':
          return Array.from(graph.nodesByType.epics.values());
        case 'feature':
          return Array.from(graph.nodesByType.features.values());
        case 'scenario':
          return Array.from(graph.nodesByType.scenarios.values());
        default:
          return [];
      }
    }

    // Node-specific collection
    switch (node.type) {
      case 'epic': {
        if (childType === 'feature') {
          const epic = node as EpicNode;
          return epic.featureIds
            .map(id => graph.nodes.get(id))
            .filter((n): n is DocumentNode => n !== undefined);
        }
        return [];
      }

      case 'feature': {
        if (childType === 'scenario') {
          const feature = node as FeatureNode;
          return feature.scenarioIds
            .map(id => graph.nodes.get(id))
            .filter((n): n is DocumentNode => n !== undefined);
        }
        return [];
      }

      default:
        return [];
    }
  }

  /**
   * Find a node by ID
   */
  private findNodeById(id: string): DocumentNode | undefined {
    return this.graphService.getNode(id);
  }

  /**
   * Convert a node to a composite URL segment (type:id)
   */
  nodeToUrlSegment(node: DocumentNode): string {
    return `${node.type}:${node.id}`;
  }

  /**
   * Build breadcrumb trail from path segments
   * Shows: Home > Epic: Social Medium > Feature: Affinity > Scenario: Emma
   */
  getBreadcrumbs(context: NavigationContext): Array<{ label: string; path: string; typeLabel?: string }> {
    const breadcrumbs: Array<{ label: string; path: string; typeLabel?: string }> = [
      { label: 'Home', path: '' }
    ];

    let compositePath = '';
    context.pathSegments.forEach((segment, index) => {
      compositePath += (index === 0 ? '' : ':') + segment.urlSegment;

      const typeLabel = this.formatTypeLabel(segment.type);
      breadcrumbs.push({
        label: segment.node?.title || segment.id,
        path: compositePath,
        typeLabel
      });
    });

    // Add collection view to breadcrumb if applicable
    if (context.viewMode === ViewMode.COLLECTION && context.collectionType) {
      const collectionLabel = this.formatTypeLabel(context.collectionType) + 's';
      compositePath += ':' + context.collectionType;
      breadcrumbs.push({
        label: collectionLabel,
        path: compositePath,
        typeLabel: undefined
      });
    }

    return breadcrumbs;
  }

  /**
   * Format type label for display
   */
  private formatTypeLabel(type: string): string {
    const labels: Record<string, string> = {
      'epic': 'Epic',
      'feature': 'Feature',
      'scenario': 'Scenario'
    };
    return labels[type] || type;
  }

  /**
   * Initialize navigation tracking to update context on route changes
   */
  private initializeNavigationTracking(): void {
    this.router.events
      .pipe(
        filter((event): event is NavigationEnd => event instanceof NavigationEnd)
      )
      .subscribe(() => {
        this.updateContextFromCurrentRoute();
      });
  }

  /**
   * Update context based on current route
   * This is called internally when navigation occurs
   */
  private updateContextFromCurrentRoute(): void {
    const url = this.router.url;

    // Parse URL to extract composite path
    const urlTree = this.router.parseUrl(url);
    const primarySegments = urlTree.root.children['primary']?.segments || [];

    // Skip 'lamad' prefix and special routes (map, search, content)
    const pathParts = primarySegments
      .map(s => s.path)
      .filter(s => s !== 'lamad');

    // Check for special routes
    if (pathParts.length > 0 && ['map', 'search', 'content'].includes(pathParts[0])) {
      // Don't update navigation context for special routes
      return;
    }

    // Extract composite path (everything after /lamad/)
    // URL: /lamad/epic:social-medium:feature:affinity
    // compositePath: "epic:social-medium:feature:affinity"
    const compositePath = pathParts.join('/'); // Segments may be split by Angular router

    const queryParams = urlTree.queryParams;

    const context = this.parsePathSegments(compositePath, queryParams);
    this.contextSubject.next(context);
  }

  /**
   * Get suggested next nodes based on current context
   * (Future: use affinity and orientation metrics)
   */
  getSuggestedNext(context: NavigationContext): DocumentNode[] {
    // For now, just return children sorted by title
    // Future: sort by orientation and affinity
    return context.children.slice().sort((a, b) => a.title.localeCompare(b.title));
  }
}
