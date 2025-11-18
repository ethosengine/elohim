import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { Router, RouterModule } from '@angular/router';
import { Subject, combineLatest } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { LearningPathService, PathNode } from '../../services/learning-path.service';
import { ContentNode } from '../../models/content-node.model';
import { AffinityStats } from '../../models/user-affinity.model';

@Component({
  selector: 'app-lamad-home',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './lamad-home.component.html',
  styleUrls: ['./lamad-home.component.css']
})
export class LamadHomeComponent implements OnInit, OnDestroy {
  // Learning path
  pathNodes: PathNodeWithAffinity[] = [];

  // Currently selected content
  selectedNode: ContentNode | null = null;
  selectedAffinity = 0;

  // Stats
  affinityStats: AffinityStats | null = null;

  // UI state
  isGraphExpanded = true;
  isLoading = true;

  private destroy$ = new Subject<void>();

  constructor(
    private readonly graphService: DocumentGraphService,
    private readonly affinityService: AffinityTrackingService,
    private readonly pathService: LearningPathService,
    private readonly router: Router,
    private readonly sanitizer: DomSanitizer
  ) {}

  ngOnInit(): void {
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

          // Calculate stats
          const allNodes = Array.from(graph.nodes.values());
          this.affinityStats = this.affinityService.getStats(allNodes);

          // Select first node if nothing selected
          if (!this.selectedNode && this.pathNodes.length > 0) {
            this.selectNode(this.pathNodes[0].node);
          }

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
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
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
   * Select a node to display in the content viewer
   */
  selectNode(node: ContentNode): void {
    this.selectedNode = node;
    this.selectedAffinity = this.affinityService.getAffinity(node.id);

    // Auto-track view
    this.affinityService.trackView(node.id);
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
    html = html.replace(/\[(.*?)\]\((.*?)\)/g, '<a href="$2" target="_blank">$1</a>');

    // Code blocks
    html = html.replace(/```(.*?)```/gs, '<pre><code>$1</code></pre>');

    // Inline code
    html = html.replace(/`(.*?)`/g, '<code>$1</code>');

    // Line breaks
    html = html.replace(/\n/g, '<br>');

    return this.sanitizer.sanitize(1, html) || '';
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

    return this.sanitizer.sanitize(1, html) || '';
  }

  /**
   * Escape HTML
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

interface PathNodeWithAffinity extends PathNode {
  affinity: number;
  affinityLevel: 'unseen' | 'low' | 'medium' | 'high';
}
