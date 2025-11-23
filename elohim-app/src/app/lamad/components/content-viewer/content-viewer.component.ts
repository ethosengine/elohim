import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { NavigationService, NavigationContext } from '../../services/navigation.service';
import { ContentNode } from '../../models/content-node.model';

@Component({
  selector: 'app-content-viewer',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './content-viewer.component.html',
  styleUrls: ['./content-viewer.component.css'],
})
export class ContentViewerComponent implements OnInit, OnDestroy {
  node: ContentNode | null = null;
  affinity = 0;
  relatedNodes: ContentNode[] = [];
  isLoading = true;
  error: string | null = null;

  // Hierarchical navigation context
  navigationContext: NavigationContext | null = null;
  breadcrumbs: Array<{ label: string; path: string; typeLabel?: string }> = [];
  children: ContentNode[] = [];

  private readonly destroy$ = new Subject<void>();
  private nodeId: string | null = null;

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly graphService: DocumentGraphService,
    private readonly affinityService: AffinityTrackingService,
    private readonly navigationService: NavigationService
  ) {}

  ngOnInit(): void {
    // Handle old-style direct content access: /content/:id
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe((params) => {
      const directId = params['id'];
      if (directId) {
        this.nodeId = directId;
        this.loadContent(directId);
        // Don't use hierarchical context for direct access
      }
    });

    // Handle new hierarchical navigation context
    this.navigationService.context$
      .pipe(takeUntil(this.destroy$))
      .subscribe((context) => {
        if (context?.currentNode) {
          this.navigationContext = context;
          this.breadcrumbs = this.navigationService.getBreadcrumbs(context);

          // Convert children to ContentNodes
          this.children = context.children; // Already ContentNodes

          // Load the current node
          this.nodeId = context.currentNode.id;
          this.loadContent(context.currentNode.id);
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
  }

  /**
   * Load content node by ID
   */
  private loadContent(nodeId: string): void {
    this.isLoading = true;
    this.error = null;

    this.graphService.graph$.pipe(takeUntil(this.destroy$)).subscribe({
      next: (graph) => {
        if (!graph) return;

        const contentNode = graph.nodes.get(nodeId);
        if (!contentNode) {
          this.error = 'Content not found';
          this.isLoading = false;
          return;
        }

        // Set ContentNode
        this.node = contentNode;

        // Get current affinity
        this.affinity = this.affinityService.getAffinity(nodeId);

        // Auto-track view (increment if first time)
        this.affinityService.trackView(nodeId);

        // Load related nodes
        this.loadRelatedNodes(graph, this.node.relatedNodeIds);

        this.isLoading = false;
      },
      error: (err) => {
        this.error = 'Failed to load content';
        this.isLoading = false;
        console.error('Error loading content:', err);
      },
    });
  }

  /**
   * Load related content nodes
   */
  private loadRelatedNodes(graph: any, relatedIds: string[]): void {
    this.relatedNodes = relatedIds
      .map((id: string) => graph.nodes.get(id))
      .filter((node: ContentNode | undefined): node is ContentNode => node !== undefined);
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
   * Navigate to a child node in the hierarchy using composite identifiers
   */
  navigateToChild(child: ContentNode): void {
    if (this.navigationContext) {
      // Build composite parent path: "epic:id:feature:id"
      const parentPath = this.navigationContext.pathSegments
        .map(s => s.urlSegment)
        .join(':');

      // Navigate to child with composite identifier
      this.navigationService.navigateTo(
        child.contentType,
        child.id,
        {
          parentPath: parentPath || undefined,
          queryParams: this.navigationContext.queryParams
        }
      );
    } else {
      // Fallback to direct navigation with composite identifier
      this.router.navigate(['/lamad', `${child.contentType}:${child.id}`]);
    }
  }

  /**
   * Navigate up one level in the hierarchy
   */
  navigateUp(): void {
    this.navigationService.navigateUp();
  }

  /**
   * Navigate to a breadcrumb using composite path
   */
  navigateToBreadcrumb(compositePath: string): void {
    if (!compositePath || compositePath.length === 0) {
      this.navigationService.navigateToHome();
    } else {
      this.router.navigate([`/lamad/${compositePath}`], {
        queryParams: this.navigationContext?.queryParams || {}
      });
    }
  }

  /**
   * Navigate back to mission map
   */
  backToMap(): void {
    this.router.navigate(['/lamad/map']);
  }

  /**
   * Check if we're in hierarchical navigation mode
   */
  isHierarchicalMode(): boolean {
    return this.navigationContext !== null;
  }

  /**
   * Check if there are children to display
   */
  hasChildren(): boolean {
    return this.children.length > 0;
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
   * Render markdown content
   */
  renderMarkdown(content: string): string {
    // Simple markdown rendering
    let html = content;

    // Headers
    html = html.replace(/^### (.*$)/gim, '<h3>$1</h3>');
    html = html.replace(/^## (.*$)/gim, '<h2>$1</h2>');
    html = html.replace(/^# (.*$)/gim, '<h1>$1</h1>');

    // Bold
    html = html.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>');

    // Italic
    html = html.replace(/\*(.*?)\*/g, '<em>$1</em>');

    // Links
    html = html.replace(/\\\[(.*?)\]\((.*?)\)/g, '<a href="$2">$1</a>');

    // Code blocks
    html = html.replace(/```(.*?)```/gs, '<pre><code>$1</code></pre>');

    // Inline code
    html = html.replace(/`(.*?)`/g, '<code>$1</code>');

    // Line breaks
    html = html.replace(/\n/g, '<br>');

    return html;
  }

  /**
   * Render Gherkin content with syntax highlighting
   */
  renderGherkin(content: string): string {
    const lines = content.split('\n');
    const keywords = ['Feature:', 'Background:', 'Scenario:', 'Scenario Outline:', 'Given', 'When', 'Then', 'And', 'But', 'Examples:'];

    return lines
      .map((line) => {
        const trimmed = line.trim();
        let className = '';

        if (trimmed.startsWith('@')) {
          className = 'gherkin-tag';
        } else if (keywords.some(keyword => trimmed.startsWith(keyword))) {
          className = 'gherkin-keyword';
        } else if (trimmed.startsWith('|')) {
          className = 'gherkin-table';
        } else if (trimmed.startsWith('#')) {
          className = 'gherkin-comment';
        }

        return `<div class="${className}">${this.escapeHtml(line)}</div>`;
      })
      .join('');
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

  /**
   * Escape HTML entities
   */
  private escapeHtml(text: string): string {
    const map: Record<string, string> = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#039;',
    };
    return text.replace(/[&<>"']/g, (m) => map[m]);
  }
}
