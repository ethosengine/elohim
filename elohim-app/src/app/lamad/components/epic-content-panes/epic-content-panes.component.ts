import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router } from '@angular/router';
import { Subject, takeUntil } from 'rxjs';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { ContentNode } from '../../models/content-node.model';
import { AffinityCircleComponent } from '../affinity-circle/affinity-circle.component';

interface PaneTab {
  id: string;
  label: string;
  icon: string;
  count?: number;
}

@Component({
  selector: 'app-epic-content-panes',
  standalone: true,
  imports: [CommonModule, AffinityCircleComponent],
  templateUrl: './epic-content-panes.component.html',
  styleUrls: ['./epic-content-panes.component.scss']
})
export class EpicContentPanesComponent implements OnInit, OnDestroy {
  private readonly destroy$ = new Subject<void>();

  epic: ContentNode | null = null;
  features: ContentNode[] = [];
  scenarios: ContentNode[] = [];
  relatedEpics: ContentNode[] = [];

  activeTab: string = 'overview';
  affinity: number = 0;

  tabs: PaneTab[] = [
    { id: 'overview', label: 'Overview', icon: '📄' },
    { id: 'features', label: 'Features', icon: '⚙️', count: 0 },
    { id: 'scenarios', label: 'Scenarios', icon: '🧪', count: 0 },
    { id: 'related', label: 'Related Epics', icon: '🔗', count: 0 }
  ];

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly documentGraphService: DocumentGraphService,
    private readonly affinityTrackingService: AffinityTrackingService
  ) {}

  ngOnInit(): void {
    this.route.params
      .pipe(takeUntil(this.destroy$))
      .subscribe(params => {
        const epicId = params['id'];
        if (epicId) {
          this.loadEpicContent(epicId);
        }
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  private loadEpicContent(epicId: string): void {
    this.documentGraphService.graph$
      .pipe(takeUntil(this.destroy$))
      .subscribe(graph => {
        if (!graph) return;

        // Load the epic
        this.epic = graph.nodes.get(epicId) || null;

        if (!this.epic) {
          console.error('Epic not found:', epicId);
          return;
        }

        // Get all directly related nodes
        const directRelations = this.documentGraphService.getRelatedNodes(epicId);

        // Filter for features
        this.features = directRelations.filter(node => node.contentType === 'feature');

        // Filter for related epics
        this.relatedEpics = directRelations.filter(node => 
            node.contentType === 'epic' && node.id !== epicId
        );

        // Load scenarios from features (indirect relation from epic perspective, usually)
        // Or direct if the graph links them directly.
        // Let's gather scenarios from the features we found.
        const scenarioSet = new Set<string>();
        const scenarios: ContentNode[] = [];

        this.features.forEach(feature => {
             const featureRelations = this.documentGraphService.getRelatedNodes(feature.id);
             featureRelations.forEach(rel => {
                 if ((rel.contentType === 'scenario' || rel.contentType === 'scenario_outline') && !scenarioSet.has(rel.id)) {
                     scenarioSet.add(rel.id);
                     scenarios.push(rel);
                 }
             });
        });

        this.scenarios = scenarios;

        // Update tab counts
        this.updateTabCounts();

        // Track affinity
        this.loadAffinity(epicId);
        this.trackInitialView(epicId);
      });
  }

  private updateTabCounts(): void {
    const featureTab = this.tabs.find(t => t.id === 'features');
    const scenarioTab = this.tabs.find(t => t.id === 'scenarios');
    const relatedTab = this.tabs.find(t => t.id === 'related');

    if (featureTab) featureTab.count = this.features.length;
    if (scenarioTab) scenarioTab.count = this.scenarios.length;
    if (relatedTab) relatedTab.count = this.relatedEpics.length;
  }

  private loadAffinity(nodeId: string): void {
    this.affinityTrackingService.affinity$
      .pipe(takeUntil(this.destroy$))
      .subscribe(affinityData => {
        this.affinity = affinityData.affinity[nodeId] || 0;
      });
  }

  private trackInitialView(nodeId: string): void {
    const currentAffinity = this.affinityTrackingService.getAffinity(nodeId);
    if (currentAffinity === 0) {
      // First time viewing, auto-increment
      this.affinityTrackingService.incrementAffinity(nodeId, 0.2);
    }
  }

  selectTab(tabId: string): void {
    this.activeTab = tabId;
  }

  getAffinityPercentage(): number {
    return Math.round(this.affinity * 100);
  }

  getAffinityLevel(): string {
    if (this.affinity >= 0.8) return 'high';
    if (this.affinity >= 0.5) return 'medium';
    if (this.affinity >= 0.2) return 'low';
    return 'none';
  }

  increaseAffinity(): void {
    if (this.epic) {
      this.affinityTrackingService.incrementAffinity(this.epic.id, 0.2);
    }
  }

  decreaseAffinity(): void {
    if (this.epic) {
      this.affinityTrackingService.incrementAffinity(this.epic.id, -0.2);
    }
  }

  markMastered(): void {
    if (this.epic) {
      this.affinityTrackingService.setAffinity(this.epic.id, 1.0);
    }
  }

  viewFeature(feature: ContentNode): void {
    this.router.navigate(['/lamad/content', feature.id]);
  }

  viewScenario(scenario: ContentNode): void {
    this.router.navigate(['/lamad/content', scenario.id]);
  }

  viewRelatedEpic(epic: ContentNode): void {
    this.router.navigate(['/lamad/content', epic.id]);
  }

  getFeatureStatusClass(feature: ContentNode): string {
    const status = feature.metadata?.['testStatus']?.status;
    if (!status) return 'status-unknown';
    return `status-${status}`;
  }

  getScenarioStatusClass(scenario: ContentNode): string {
    const status = scenario.metadata?.['testStatus']?.status;
    if (!status) return 'status-unknown';
    return `status-${status}`;
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
    html = html.replace(/\[(.*?)\]\((.*?)\)/g, '<a href="$2">$1</a>');

    // Code blocks
    html = html.replace(/```(.*?)```/gs, '<pre><code>$1</code></pre>');

    // Inline code
    html = html.replace(/`(.*?)`/g, '<code>$1</code>');

    // Line breaks
    html = html.replace(/\n/g, '<br>');

    return html;
  }
}
