import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { EpicNode, FeatureNode, ScenarioNode } from '../../models';
import { AffinityStats } from '../../models/user-affinity.model';
import { DocumentNode } from '../../models/document-node.model';

@Component({
  selector: 'app-lamad-home',
  standalone: true,
  imports: [CommonModule, RouterLink],
  templateUrl: './lamad-home.component.html',
  styleUrls: ['./lamad-home.component.css']
})
export class LamadHomeComponent implements OnInit, OnDestroy {
  epics: EpicNode[] = [];
  features: FeatureNode[] = [];
  scenarios: ScenarioNode[] = [];
  stats: any = {};
  affinityStats: AffinityStats | null = null;

  private destroy$ = new Subject<void>();

  constructor(
    private readonly documentGraphService: DocumentGraphService,
    private readonly affinityService: AffinityTrackingService
  ) {}

  ngOnInit(): void {
    this.documentGraphService.graph$
      .pipe(takeUntil(this.destroy$))
      .subscribe((graph) => {
        if (graph) {
          this.epics = Array.from(graph.nodesByType.epics.values());
          this.features = Array.from(graph.nodesByType.features.values());
          this.scenarios = Array.from(graph.nodesByType.scenarios.values());
          this.stats = graph.metadata.stats;

          // Calculate affinity stats
          const allNodes = Array.from(graph.nodes.values()) as DocumentNode[];
          this.affinityStats = this.affinityService.getStats(allNodes);
        }
      });

    // Update when affinity changes
    this.affinityService.changes$
      .pipe(takeUntil(this.destroy$))
      .subscribe(() => {
        const graph = this.documentGraphService.getGraph();
        if (graph) {
          const allNodes = Array.from(graph.nodes.values()) as DocumentNode[];
          this.affinityStats = this.affinityService.getStats(allNodes);
        }
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  getEpicCategory(epic: EpicNode): string {
    return epic.category ?? 'general';
  }

  getFeatureCategory(feature: FeatureNode): string {
    return feature.category ?? 'general';
  }

  getCategoryIcon(category: string): string {
    const icons: Record<string, string> = {
      observer: '👁️',
      'value-scanner': '🔍',
      'autonomous-entity': '🤖',
      social: '🌐',
      deployment: '🚀',
      general: '📄'
    };
    return icons[category] || '📄';
  }
}
