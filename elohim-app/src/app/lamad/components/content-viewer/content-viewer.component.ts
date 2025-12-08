import {
  Component,
  OnInit,
  OnDestroy,
  ViewChild,
  ViewContainerRef,
  ComponentRef,
  inject
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { Subject, Subscription, forkJoin, of } from 'rxjs';
import { takeUntil, catchError } from 'rxjs/operators';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { ContentService } from '../../services/content.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { SeoService } from '../../../services/seo.service';
import { ContentNode } from '../../models/content-node.model';
import {
  RendererRegistryService,
  ContentRenderer,
  RendererCompletionEvent
} from '../../renderers/renderer-registry.service';

import { TrustBadgeService } from '@app/elohim/services/trust-badge.service';
import { TrustBadge } from '@app/elohim/models/trust-badge.model';

// Content I/O for download functionality
import { ContentDownloadComponent } from '../../content-io/components/content-download/content-download.component';
import { ContentEditorService } from '../../content-io/services/content-editor.service';

@Component({
  selector: 'app-content-viewer',
  standalone: true,
  imports: [CommonModule, RouterModule, ContentDownloadComponent],
  templateUrl: './content-viewer.component.html',
  styleUrls: ['./content-viewer.component.css'],
})
export class ContentViewerComponent implements OnInit, OnDestroy {
  node: ContentNode | null = null;
  affinity = 0;
  relatedNodes: ContentNode[] = [];
  isLoading = true;
  error: string | null = null;

  // Tab state
  activeTab: 'content' | 'trust' | 'governance' | 'network' = 'content';

  // Trust data
  trustBadge: TrustBadge | null = null;
  isLoadingTrust = false;

  // "Appears in paths" back-links (Wikipedia-style)
  containingPaths: Array<{ pathId: string; pathTitle: string; stepIndex: number }> = [];
  loadingPaths = false;

  // Edit capability
  canEditContent = false;

  // Dynamic renderer hosting
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;
  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSubscription: Subscription | null = null;

  /** Whether we have a registered renderer for the current content format */
  hasRegisteredRenderer = false;

  private readonly destroy$ = new Subject<void>();
  private nodeId: string | null = null;
  private readonly seoService = inject(SeoService);

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly affinityService: AffinityTrackingService,
    private readonly rendererRegistry: RendererRegistryService,
    private readonly contentService: ContentService,
    private readonly dataLoader: DataLoaderService,
    private readonly trustBadgeService: TrustBadgeService,
    private readonly editorService: ContentEditorService
  ) {}

  ngOnInit(): void {
    // Handle direct content access: /lamad/resource/:resourceId or /lamad/content/:id
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe((params) => {
      const resourceId = params['resourceId'] ?? params['id'];
      if (resourceId) {
        this.nodeId = resourceId;
        this.loadContent(resourceId);
      }
    });

    // Listen for affinity changes
    this.affinityService.changes$
      .pipe(takeUntil(this.destroy$))
      .subscribe((change) => {
        if (change && change.nodeId === this.nodeId) {
          this.affinity = change.newValue;
        }
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
  }

  /**
   * Clean up the current renderer instance
   */
  private destroyRenderer(): void {
    if (this.rendererSubscription) {
      this.rendererSubscription.unsubscribe();
      this.rendererSubscription = null;
    }
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }

  /**
   * Dynamically instantiate the appropriate renderer for the current node.
   * Called after the node is loaded and the view is ready.
   */
  private loadRenderer(): void {
    if (!this.node || !this.rendererHost) {
      return;
    }

    // Clean up previous renderer
    this.destroyRenderer();
    this.rendererHost.clear();

    // Get the renderer component for this content format
    const rendererComponent = this.rendererRegistry.getRenderer(this.node);

    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      return;
    }

    this.hasRegisteredRenderer = true;

    // Create the renderer component
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);

    // Set the node input using setInput to trigger ngOnChanges
    this.rendererRef.setInput('node', this.node);

    // Subscribe to completion events if the renderer supports them
    const instance = this.rendererRef.instance as any;
    if (instance.complete) {
      this.rendererSubscription = instance.complete.subscribe(
        (event: RendererCompletionEvent) => this.onRendererComplete(event)
      );
    }
  }

  /**
   * Handle completion events from interactive renderers (quiz, simulation, etc.)
   * Updates affinity based on the completion result.
   */
  private onRendererComplete(event: RendererCompletionEvent): void {
    if (!this.nodeId) return;

    // Map completion result to affinity delta
    // Passing increases affinity more than failing
    const affinityDelta = event.passed
      ? 0.3 + (event.score / 100) * 0.2  // 0.3 to 0.5 for passing
      : 0.1;                              // Small bump for attempting

    this.affinityService.incrementAffinity(this.nodeId, affinityDelta);
  }

  /**
   * Load content node by ID
   */
  private loadContent(nodeId: string): void {
    this.isLoading = true;
    this.error = null;

    this.dataLoader.getContent(nodeId).pipe(takeUntil(this.destroy$)).subscribe({
      next: (contentNode) => {
        if (!contentNode) {
          this.error = 'Content not found';
          this.isLoading = false;
          return;
        }

        // Set ContentNode
        this.node = contentNode;

        // Check if content is editable
        this.canEditContent = this.editorService.canEdit(contentNode);

        // Update SEO metadata for this content
        this.seoService.updateForContent({
          id: contentNode.id,
          title: contentNode.title,
          summary: contentNode.description,
          contentType: contentNode.contentType,
          thumbnailUrl: contentNode.metadata?.['thumbnailUrl'],
          authors: contentNode.metadata?.['authors'],
          createdAt: contentNode.createdAt,
          updatedAt: contentNode.updatedAt
        });

        // Get current affinity
        this.affinity = this.affinityService.getAffinity(nodeId);

        // Auto-track view (increment if first time)
        this.affinityService.trackView(nodeId);

        // Load related nodes
        this.loadRelatedNodes(contentNode.relatedNodeIds);

        // Load containing paths (Wikipedia-style "appears in" back-links)
        this.loadContainingPaths(nodeId);

        // Load trust badge data for Attestations tab
        this.loadTrustBadge(nodeId);

        this.isLoading = false;

        // Load the appropriate renderer for this content format
        // Use setTimeout to ensure ViewChild is available after view updates
        setTimeout(() => this.loadRenderer(), 0);
      },
      error: () => {
        this.error = 'Failed to load content';
        this.isLoading = false;
      },
    });
  }

  /**
   * Load paths that contain this content (Wikipedia-style back-links)
   */
  private loadContainingPaths(nodeId: string): void {
    this.loadingPaths = true;
    this.containingPaths = [];

    this.contentService.getContainingPathsSummary(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (paths) => {
          this.containingPaths = paths;
          this.loadingPaths = false;
        },
        error: () => {
          this.loadingPaths = false;
        }
      });
  }

  /**
   * Load Trust Badge data for the Attestations tab
   */
  private loadTrustBadge(nodeId: string): void {
    this.isLoadingTrust = true;
    this.trustBadge = null;

    this.trustBadgeService.getBadge(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (badge) => {
          this.trustBadge = badge;
          this.isLoadingTrust = false;
        },
        error: () => {
          this.isLoadingTrust = false;
        }
      });
  }

  /**
   * Switch active tab
   */
  setActiveTab(tab: 'content' | 'trust' | 'governance' | 'network'): void {
    this.activeTab = tab;
  }

  /**
   * Handle badge action click
   */
  handleAction(action: any): void {
    if (action.route) {
      this.router.navigate([action.route]);
    }
    // Actions without routes are no-ops (e.g., placeholder actions)
  }

  /**
   * Navigate to a path that contains this content
   */
  navigateToPath(pathId: string, stepIndex: number): void {
    this.router.navigate(['/lamad/path', pathId, 'step', stepIndex]);
  }

  /**
   * Load related content nodes
   */
  private loadRelatedNodes(relatedIds: string[]): void {
    if (!relatedIds || relatedIds.length === 0) {
      this.relatedNodes = [];
      return;
    }

    // Load related nodes in parallel (limit to 5)
    const loadObservables = relatedIds.slice(0, 5).map(id =>
      this.dataLoader.getContent(id).pipe(catchError(() => of(null)))
    );

    forkJoin(loadObservables).pipe(takeUntil(this.destroy$)).subscribe({
      next: (nodes) => {
        this.relatedNodes = nodes.filter((n): n is ContentNode => n !== null);
      },
      error: () => {
        this.relatedNodes = [];
      }
    });
  }

  /**
   * Manually adjust affinity
   */
  adjustAffinity(delta: number): void {
    if (!this.nodeId) return;
    this.affinityService.incrementAffinity(this.nodeId, delta);
  }

  /**
   * Set affinity to a specific value
   */
  setAffinity(value: number): void {
    if (!this.nodeId) return;
    this.affinityService.setAffinity(this.nodeId, value);
  }

  /**
   * Navigate to related content
   */
  viewRelatedContent(node: ContentNode): void {
    this.router.navigate(['/lamad/content', node.id]);
  }

  /**
   * Navigate back to lamad home
   */
  backToHome(): void {
    this.router.navigate(['/lamad']);
  }

  /**
   * Get affinity level
   */
  getAffinityLevel(): string {
    if (this.affinity === 0) return 'unseen';
    if (this.affinity <= 0.33) return 'low';
    if (this.affinity <= 0.66) return 'medium';
    return 'high';
  }

  /**
   * Get affinity percentage
   */
  getAffinityPercentage(): number {
    return Math.round(this.affinity * 100);
  }

  /**
   * Get content type display
   */
  getContentTypeDisplay(): string {
    if (!this.node) return '';
    const displays: Record<string, string> = {
      epic: 'Epic',
      feature: 'Feature',
      scenario: 'Scenario',
    };
    return displays[this.node.contentType] || this.node.contentType;
  }

  /**
   * Get content type icon
   */
  getContentTypeIcon(): string {
    if (!this.node) return '';
    const icons: Record<string, string> = {
      epic: '📖',
      feature: '⚙️',
      scenario: '✓',
    };
    return icons[this.node.contentType] || '📄';
  }

  /**
   * Get content as string (handles string | object union type)
   */
  getStringContent(content: string | object): string {
    if (typeof content === 'string') {
      return content;
    }
    return JSON.stringify(content, null, 2);
  }

  /**
   * Get affinity percentage for related node
   */
  getRelatedNodeAffinity(nodeId: string): number {
    return Math.round(this.affinityService.getAffinity(nodeId) * 100);
  }

  /**
   * Get metadata category
   */
  getMetadataCategory(): string | null {
    if (!this.node?.metadata?.['category']) return null;
    return this.node.metadata['category'];
  }

  /**
   * Get metadata authors as joined string
   */
  getMetadataAuthors(): string | null {
    if (!this.node?.metadata?.['authors']) return null;
    const authors = this.node.metadata['authors'];
    if (Array.isArray(authors) && authors.length > 0) {
      return authors.join(', ');
    }
    return null;
  }

  /**
   * Get metadata version
   */
  getMetadataVersion(): string | null {
    if (!this.node?.metadata?.['version']) return null;
    return this.node.metadata['version'];
  }
}
