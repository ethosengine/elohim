import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { Router, RouterModule } from '@angular/router';
import { Subject, combineLatest } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { LearningPathService, PathNode } from '../../services/learning-path.service';
import { NavigationService } from '../../services/navigation.service';
import { ContentNode } from '../../models/content-node.model';
import { AffinityStats } from '../../models/user-affinity.model';
import { AffinityCircleComponent } from '../affinity-circle/affinity-circle.component';

@Component({
  selector: 'app-lamad-home',
  standalone: true,
  imports: [CommonModule, RouterModule, AffinityCircleComponent],
  templateUrl: './lamad-home.component.html',
  styleUrls: ['./lamad-home.component.css']
})
export class LamadHomeComponent implements OnInit, OnDestroy {
  // Learning path
  pathNodes: PathNodeWithAffinity[] = [];

  // Sidebar nodes (Epics only)
  sidebarNodes: PathNodeWithAffinity[] = [];

  // All nodes for "At a Glance"
  allContentNodes: ContentNodeWithAffinity[] = [];

  // Currently selected content
  selectedNode: ContentNode | null = null;
  selectedAffinity = 0;

  // Stats
  affinityStats: AffinityStats | null = null;

  // UI state
  isGraphExpanded = false; // Default collapsed for better initial view
  isLoading = true;
  isSidebarOpen = true;
  isSearchOpen = false;

  // Drill-down state
  viewState: 'root' | 'drilldown' = 'root';
  activeDrillDown: {
    parent: ContentNode,
    nodes: ContentNodeWithAffinity[],
    type: string
  } | null = null;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly graphService: DocumentGraphService,
    private readonly affinityService: AffinityTrackingService,
    private readonly pathService: LearningPathService,
    private readonly navigationService: NavigationService,
    private readonly router: Router,
    private readonly sanitizer: DomSanitizer
  ) {}

  ngOnInit(): void {
    // Set initial sidebar state based on screen width
    this.checkScreenSize();

    // Load graph and build path
    combineLatest([
      this.graphService.graph$,
      this.pathService.path$,
      this.affinityService.affinity$
    ])
      .pipe(takeUntil(this.destroy$))
      .subscribe(([graph, path, affinity]) => {
        if (graph && path.length > 0) {
          // Enrich path nodes with affinity data
          this.pathNodes = path.map(pn => this.enrichPathNode(pn));

          // Enrich all nodes for the hexagon view and drill-down lookups
          const allNodes = Array.from(graph.nodes.values());
          this.allContentNodes = allNodes.map(node => this.enrichContentNode(node));

          // Initial sidebar state (Epics) if at root
          if (this.viewState === 'root') {
             this.sidebarNodes = this.pathNodes.filter(pn => pn.node.contentType === 'epic');
          }

          // Calculate stats
          this.affinityStats = this.affinityService.getStats(allNodes);

          this.isLoading = false;
        }
      });

    // Listen for affinity changes to update UI
    this.affinityService.changes$
      .pipe(takeUntil(this.destroy$))
      .subscribe(change => {
        if (change && this.selectedNode && change.nodeId === this.selectedNode.id) {
          this.selectedAffinity = change.newValue;
        }
        // Refresh path nodes affinity
        this.pathNodes = this.pathNodes.map(pn => this.enrichPathNode({
          node: pn.node,
          order: pn.order,
          depth: pn.depth,
          category: pn.category
        }));

        // Refresh sidebar nodes based on current view
        if (this.viewState === 'root') {
          this.sidebarNodes = this.pathNodes.filter(pn => pn.node.contentType === 'epic');
        } else if (this.viewState === 'drilldown' && this.activeDrillDown) {
           // Re-enrich drilled down nodes to update affinity colors
           this.activeDrillDown.nodes = this.activeDrillDown.nodes.map(node =>
             this.enrichContentNode(node)
           );
           this.updateSidebarForDrillDown();
        }

        // Refresh all content nodes
        if (this.allContentNodes.length > 0) {
           this.allContentNodes = this.allContentNodes.map(node => this.enrichContentNode(node));
        }
      });

    // Listen for navigation changes (URL updates)
    this.navigationService.context$
      .pipe(takeUntil(this.destroy$))
      .subscribe(context => {
        if (context?.currentNode) {
          // URL has a node selected
          // Update local state without navigating again
          this.selectedNode = context.currentNode;
          this.selectedAffinity = this.affinityService.getAffinity(this.selectedNode.id);
          this.affinityService.trackView(this.selectedNode.id);

          window.scrollTo(0, 0);
          this.closeSidebar();
        } else if (context?.viewMode === 'home') {
          // At root /lamad
          this.goHome(false); // false = don't navigate router
        }
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  private checkScreenSize(): void {
    if (window.innerWidth < 768) {
      this.isSidebarOpen = false;
      this.isGraphExpanded = false;
    }
  }

  /**
   * Navigate to Home / Overview
   * @param navigate Whether to update the URL (default true)
   */
  goHome(navigate = true): void {
    this.selectedNode = null;
    // Don't force expand on mobile to avoid jarring layout shifts
    if (window.innerWidth >= 768) {
        this.isGraphExpanded = true;
    }
    
    this.viewState = 'root';
    this.activeDrillDown = null;

    // Reset sidebar to epics
    this.sidebarNodes = this.pathNodes.filter(pn => pn.node.contentType === 'epic');

    if (navigate) {
      this.navigationService.navigateToHome();
    }
    
    window.scrollTo(0, 0);
  }
  
  /**
   * Handle click on Overview sidebar item
   */
  onOverviewClick(): void {
    // If we are already at home (no selected node, root view), clicking overview should toggle graph
    if (!this.selectedNode && this.viewState === 'root') {
        this.toggleGraph();
    } else {
        // Otherwise navigate home
        this.goHome();
    }
  }

  /**
   * Go up one level from drilldown
   */
  goUp(): void {
    if (this.viewState === 'drilldown') {
      this.goHome();
    } else if (this.selectedNode) {
      // Navigate up in the URL hierarchy
      this.navigationService.navigateUp();
    }
  }

  /**
   * Drill down into a specific content type for a parent node
   */
  drillDown(parent: ContentNode, type: string): void {
    const relatedIds = parent.relatedNodeIds || [];
    const relatedNodes = this.allContentNodes.filter(n =>
      relatedIds.includes(n.id) && n.contentType === type
    );

    this.activeDrillDown = {
      parent,
      nodes: relatedNodes,
      type
    };
    this.viewState = 'drilldown';
    this.selectedNode = null; // Ensure we show the list view

    this.updateSidebarForDrillDown();
    window.scrollTo(0, 0);
  }

  private updateSidebarForDrillDown(): void {
    if (!this.activeDrillDown) return;

    // Map content nodes to PathNodes for sidebar compatibility
    this.sidebarNodes = this.activeDrillDown.nodes.map((node, index) => ({
      node,
      order: index,
      depth: 1,
      category: node.metadata?.['category'] ?? 'general',
      affinity: node.affinity,
      affinityLevel: node.affinityLevel
    }));
  }

  /**
   * Get related nodes grouped by type for card display
   */
  getRelatedGrouping(node: ContentNode): { type: string, count: number }[] {
    if (!node.relatedNodeIds || node.relatedNodeIds.length === 0) return [];

    const counts = new Map<string, number>();

    node.relatedNodeIds.forEach(id => {
      const related = this.allContentNodes.find(n => n.id === id);
      if (related) {
        const type = related.contentType;
        counts.set(type, (counts.get(type) ?? 0) + 1);
      }
    });

    const groups: { type: string, count: number }[] = [];
    counts.forEach((count, type) => {
      groups.push({ type, count });
    });

    // Sort for consistency
    return groups.sort((a, b) => a.type.localeCompare(b.type));
  }

  /**
   * Toggle sidebar visibility
   */
  toggleSidebar(): void {
    this.isSidebarOpen = !this.isSidebarOpen;
  }

  /**
   * Close sidebar (useful for mobile selection)
   */
  closeSidebar(): void {
    if (window.innerWidth < 768) {
      this.isSidebarOpen = false;
    }
  }

  /**
   * Toggle search
   */
  toggleSearch(): void {
    this.isSearchOpen = !this.isSearchOpen;
  }

  /**
   * Enrich path node with affinity data
   */
  private enrichPathNode(pathNode: PathNode): PathNodeWithAffinity {
    const affinity = this.affinityService.getAffinity(pathNode.node.id);
    return {
      ...pathNode,
      affinity,
      affinityLevel: this.getAffinityLevel(affinity)
    };
  }

  /**
   * Enrich generic content node with affinity data
   */
  private enrichContentNode(node: ContentNode): ContentNodeWithAffinity {
    const affinity = this.affinityService.getAffinity(node.id);
    return {
      ...node,
      affinity,
      affinityLevel: this.getAffinityLevel(affinity)
    };
  }

  /**
   * Select a node to display in the content viewer
   * This now uses NavigationService to update URL
   */
  selectNode(node: ContentNode): void {
    // Use navigation service to update URL
    // The local state update will happen via subscription to context$

    let parentPath: string | undefined;

    // Check if it's a child of active drilldown parent
    if (this.activeDrillDown?.parent) {
        // Construct parent path from parent node
        // parentType:parentId
        parentPath = `${this.activeDrillDown.parent.contentType}:${this.activeDrillDown.parent.id}`;
    }

    this.navigationService.navigateTo(node.contentType, node.id, {
        parentPath
    });
  }

  /**
   * Toggle graph visualization expansion
   */
  toggleGraph(): void {
    this.isGraphExpanded = !this.isGraphExpanded;
  }

  /**
   * Navigate to next node in path
   */
  goToNext(): void {
    if (!this.selectedNode) return;
    const next = this.pathService.getNextNode(this.selectedNode.id);
    if (next) {
      this.selectNode(next.node);
    }
  }

  /**
   * Navigate to previous node in path
   */
  goToPrevious(): void {
    if (!this.selectedNode) return;
    const prev = this.pathService.getPreviousNode(this.selectedNode.id);
    if (prev) {
      this.selectNode(prev.node);
    }
  }

  /**
   * Check if there's a next node
   */
  hasNext(): boolean {
    if (!this.selectedNode) return false;
    return this.pathService.getNextNode(this.selectedNode.id) !== null;
  }

  /**
   * Check if there's a previous node
   */
  hasPrevious(): boolean {
    if (!this.selectedNode) return false;
    return this.pathService.getPreviousNode(this.selectedNode.id) !== null;
  }

  /**
   * Manually adjust affinity
   */
  adjustAffinity(delta: number): void {
    if (!this.selectedNode) return;
    this.affinityService.incrementAffinity(this.selectedNode.id, delta);
  }

  /**
   * Get affinity level classification
   */
  getAffinityLevel(affinity: number): 'unseen' | 'low' | 'medium' | 'high' {
    if (affinity === 0) return 'unseen';
    if (affinity <= 0.33) return 'low';
    if (affinity <= 0.66) return 'medium';
    return 'high';
  }

  /**
   * Get affinity percentage
   */
  getAffinityPercentage(affinity: number): number {
    return Math.round(affinity * 100);
  }

  /**
   * Get path progress percentage
   */
  getPathProgress(): number {
    if (!this.affinityStats) return 0;
    const affinityMap = new Map<string, number>();
    this.pathNodes.forEach(pn => {
      affinityMap.set(pn.node.id, pn.affinity);
    });
    return this.pathService.getPathProgress(affinityMap);
  }

  /**
   * Get content type icon
   */
  getContentTypeIcon(contentType: string): string {
    const icons: Record<string, string> = {
      epic: '📖',
      feature: '⚙️',
      scenario: '✓',
      organization: '🏢',
      user_type: '👤',
      book: '📚',
      video: '🎥',
      article: '📰',
      audio: '🎧',
      document: '📄',
      page: '🌐'
    };
    return icons[contentType] || '📄';
  }

  /**
   * Get category display name
   */
  getCategoryDisplay(category: string): string {
    const displays: Record<string, string> = {
      vision: 'Vision',
      core: 'Core Concepts',
      advanced: 'Advanced',
      systemic: 'Systemic View',
      implementation: 'Implementation',
      technical: 'Technical',
      // Epic categories
      governance: 'Governance',
      autonomous_entity: 'Autonomous Entity',
      public_observer: 'Public Observer',
      social_medium: 'Social Medium',
      value_scanner: 'Value Scanner'
    };
    return displays[category] || category;
  }

  /**
   * Get current position in the learning path (1-indexed)
   */
  getCurrentPosition(): number {
    if (!this.selectedNode) return 0;
    return this.pathNodes.findIndex(pn => pn.node.id === this.selectedNode?.id) + 1;
  }

  /**
   * Render markdown content (simple version)
   */
  renderMarkdown(content: string): SafeHtml {
    if (!content) return '';

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
    html = html.replace(/\\\[(.*?)\\\]\((.*?)\)/g, '<a href="$2" target="_blank">$1</a>');

    // Code blocks
    html = html.replace(/```(.*?)```/gs, '<pre><code>$1</code></pre>');

    // Inline code
    html = html.replace(/`(.*?)`/g, '<code>$1</code>');

    // Line breaks
    html = html.replace(/\n/g, '<br>');

    return this.sanitizer.sanitize(1, html) ?? '';
  }

  /**
   * Render Gherkin content
   */
  renderGherkin(content: string): SafeHtml {
    if (!content) return '';

    const lines = content.split('\n');
    const keywords = ['Feature:', 'Background:', 'Scenario:', 'Scenario Outline:', 'Given', 'When', 'Then', 'And', 'But', 'Examples:'];

    const html = lines
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

    return this.sanitizer.sanitize(1, html) ?? '';
  }

  /**
   * Escapes HTML special characters to prevent XSS attacks
   */
  private escapeHtml(text: string): string {
    const map: Record<string, string> = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#039;'
    };
    return text.replace(/[&<>"']/g, (m) => map[m]);
  }
}

interface PathNodeWithAffinity extends PathNode {
  affinity: number;
  affinityLevel: 'unseen' | 'low' | 'medium' | 'high';
}

interface ContentNodeWithAffinity extends ContentNode {
  affinity: number;
  affinityLevel: 'unseen' | 'low' | 'medium' | 'high';
}
