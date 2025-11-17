import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { DocumentGraphService } from '../../services/document-graph.service';
import { FeatureNode, ScenarioNode, EpicNode } from '../../models';

@Component({
  selector: 'app-feature-viewer',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="feature-viewer" *ngIf="feature">
      <div class="feature-header">
        <div class="breadcrumb">
          <a routerLink="/docs">Home</a> / <span>Feature</span>
        </div>
        <h1 class="feature-title">{{feature.title}}</h1>
        <div class="feature-meta">
          <span class="meta-badge">{{feature.category}}</span>
          <span class="meta-badge">{{feature.scenarioIds.length}} scenarios</span>
        </div>
      </div>

      <div class="gherkin-content">
        <pre><code>{{feature.gherkinContent}}</code></pre>
      </div>

      <div class="scenarios-section" *ngIf="scenarios.length > 0">
        <h2>Scenarios</h2>
        <div class="scenario-list">
          <a *ngFor="let scenario of scenarios"
             [routerLink]="['/docs/scenario', scenario.id]"
             class="scenario-card">
            <h3>{{scenario.title}}</h3>
            <div class="step-count">{{scenario.steps.length}} steps</div>
          </a>
        </div>
      </div>

      <div class="related-epics" *ngIf="relatedEpics.length > 0">
        <h2>Related Epics</h2>
        <div class="epic-list">
          <a *ngFor="let epic of relatedEpics"
             [routerLink]="['/docs/epic', epic.id]"
             class="epic-link">
            📖 {{epic.title}}
          </a>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .feature-viewer { padding: 2rem; }
    .feature-header { margin-bottom: 2rem; }
    .breadcrumb { font-size: 0.875rem; color: #64748b; margin-bottom: 1rem; }
    .breadcrumb a { color: #6366f1; text-decoration: none; }
    .feature-title { font-size: 2rem; margin: 0 0 1rem; color: #22c55e; }
    .feature-meta { display: flex; gap: 0.75rem; }
    .meta-badge { padding: 0.5rem 1rem; background: rgba(34, 197, 94, 0.15); border: 1px solid rgba(34, 197, 94, 0.3); border-radius: 0.5rem; font-size: 0.875rem; }
    .gherkin-content { background: rgba(15, 23, 42, 0.8); border: 1px solid rgba(99, 102, 241, 0.2); border-radius: 1rem; padding: 1.5rem; margin: 2rem 0; }
    .gherkin-content pre { margin: 0; overflow-x: auto; }
    .gherkin-content code { color: #94a3b8; font-family: 'Courier New', monospace; line-height: 1.6; }
    .scenarios-section, .related-epics { margin-top: 3rem; }
    h2 { color: #e0e6ed; margin-bottom: 1rem; }
    .scenario-list, .epic-list { display: flex; flex-direction: column; gap: 1rem; }
    .scenario-card { background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(34, 197, 94, 0.2); border-radius: 0.75rem; padding: 1rem; text-decoration: none; color: inherit; transition: all 0.2s; }
    .scenario-card:hover { border-color: rgba(34, 197, 94, 0.5); transform: translateX(4px); }
    .scenario-card h3 { margin: 0 0 0.5rem; font-size: 1.125rem; color: #22c55e; }
    .step-count { font-size: 0.875rem; color: #64748b; }
    .epic-link { color: #8b5cf6; text-decoration: none; padding: 0.5rem; display: block; }
    .epic-link:hover { background: rgba(139, 92, 246, 0.1); border-radius: 0.375rem; }
  `]
})
export class FeatureViewerComponent implements OnInit {
  feature: FeatureNode | null = null;
  scenarios: ScenarioNode[] = [];
  relatedEpics: EpicNode[] = [];

  constructor(
    private readonly route: ActivatedRoute,
    private readonly documentGraphService: DocumentGraphService
  ) {}

  ngOnInit(): void {
    this.route.params.subscribe(params => {
      const node = this.documentGraphService.getNode(params['id']);
      if (node && node.type === 'feature') {
        this.feature = node as FeatureNode;
        this.loadRelated();
      }
    });
  }

  private loadRelated(): void {
    if (!this.feature) return;
    this.scenarios = this.feature.scenarioIds
      .map(id => this.documentGraphService.getNode(id))
      .filter((n): n is ScenarioNode => n?.type === 'scenario');
    this.relatedEpics = this.feature.epicIds
      .map(id => this.documentGraphService.getNode(id))
      .filter((n): n is EpicNode => n?.type === 'epic');
  }
}
