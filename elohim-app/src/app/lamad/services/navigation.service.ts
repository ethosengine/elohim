import { Injectable } from '@angular/core';
import { Router, ActivatedRoute, NavigationEnd } from '@angular/router';
import { BehaviorSubject, Observable } from 'rxjs';
import { filter, map } from 'rxjs/operators';
import { DocumentGraphService } from './document-graph.service';
import { DocumentNode, EpicNode, FeatureNode, ScenarioNode } from '../models';

/**
 * Represents a single segment in the navigation path
 */
export interface PathSegment {
  /** Node ID */
  id: string;
  /** The actual node */
  node: DocumentNode;
  /** URL segment (may be different from ID for readability) */
  urlSegment: string;
}

/**
 * Context for the current navigation state
 */
export interface NavigationContext {
  /** Full path from root to current node */
  pathSegments: PathSegment[];

  /** Current node being viewed */
  currentNode: DocumentNode | null;

  /** Children of current node (for panes display) */
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
   * Navigate to a node with optional path context
   */
  navigateTo(
    nodeId: string,
    options?: {
      parentPath?: string[];
      queryParams?: Record<string, any>;
    }
  ): void {
    const pathSegments = options?.parentPath ? [...options.parentPath, nodeId] : [nodeId];
    const urlPath = `/lamad/${pathSegments.join('/')}`;

    this.router.navigate([urlPath], {
      queryParams: options?.queryParams || {}
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
    if (!context || context.pathSegments.length <= 1) {
      this.navigateToHome();
      return;
    }

    const parentSegments = context.pathSegments.slice(0, -1);
    const urlPath = `/lamad/${parentSegments.map(s => s.urlSegment).join('/')}`;

    this.router.navigate([urlPath], {
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
   * Parse URL path into navigation context
   */
  parsePathSegments(segments: string[], queryParams: any = {}): NavigationContext | null {
    const graph = this.graphService.getGraph();
    if (!graph) return null;

    // If no segments, we're at home - show epics
    if (segments.length === 0) {
      return {
        pathSegments: [],
        currentNode: null,
        children: Array.from(graph.nodesByType.epics.values()),
        parent: null,
        queryParams
      };
    }

    // Parse each segment into PathSegment objects
    const pathSegments: PathSegment[] = [];
    for (const urlSegment of segments) {
      const node = this.findNodeByUrlSegment(urlSegment);
      if (!node) {
        console.warn(`Node not found for URL segment: ${urlSegment}`);
        return null;
      }

      pathSegments.push({
        id: node.id,
        node,
        urlSegment
      });
    }

    // Get current node (last segment)
    const currentSegment = pathSegments[pathSegments.length - 1];
    const currentNode = currentSegment.node;

    // Get parent node (second to last segment)
    const parent = pathSegments.length > 1
      ? pathSegments[pathSegments.length - 2].node
      : null;

    // Get children of current node
    const children = this.getChildren(currentNode);

    return {
      pathSegments,
      currentNode,
      children,
      parent,
      queryParams
    };
  }

  /**
   * Get children of a node based on its type and relationships
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
   * Find a node by URL segment (ID or slug)
   */
  private findNodeByUrlSegment(urlSegment: string): DocumentNode | undefined {
    // For now, URL segment is the node ID
    // In the future, could support slugs or friendly URLs
    return this.graphService.getNode(urlSegment);
  }

  /**
   * Convert a node to a URL segment
   */
  nodeToUrlSegment(node: DocumentNode): string {
    // For now, use the node ID directly
    // Future: could create URL-friendly slugs from titles
    return node.id;
  }

  /**
   * Build breadcrumb trail from path segments
   */
  getBreadcrumbs(context: NavigationContext): Array<{ label: string; path: string[] }> {
    const breadcrumbs: Array<{ label: string; path: string[] }> = [
      { label: 'Home', path: [] }
    ];

    context.pathSegments.forEach((segment, index) => {
      breadcrumbs.push({
        label: segment.node.title,
        path: context.pathSegments.slice(0, index + 1).map(s => s.urlSegment)
      });
    });

    return breadcrumbs;
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

    // Parse URL to extract path segments
    const urlTree = this.router.parseUrl(url);
    const primarySegments = urlTree.root.children['primary']?.segments || [];

    // Skip 'lamad' prefix and special routes
    const pathSegments = primarySegments
      .map(s => s.path)
      .filter(s => s !== 'lamad' && s !== 'map' && s !== 'search' && s !== 'content');

    const queryParams = urlTree.queryParams;

    const context = this.parsePathSegments(pathSegments, queryParams);
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
