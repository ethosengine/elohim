import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { DocumentGraphService } from '../../services/document-graph.service';
import { ScenarioNode, FeatureNode } from '../../models';

@Component({
  selector: 'app-scenario-detail',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="scenario-detail" *ngIf="scenario">
      <div class="scenario-header">
        <div class="breadcrumb">
          <a routerLink="/lamad">Home</a> /
          <a [routerLink]="['/lamad/content', feature.id]" *ngIf="feature">{{feature.title}}</a> /
          <span>Scenario</span>
        </div>
        <h1 class="scenario-title">{{scenario.title}}</h1>
        <div class="scenario-meta">
          <span class="meta-badge">{{scenario.scenarioType}}</span>
          <span class="meta-badge">{{scenario.steps.length}} steps</span>
        </div>
      </div>

      <div class="steps-section">
        <h2>Steps</h2>
        <div class="steps-list">
          <div *ngFor="let step of scenario.steps; let i = index" class="step-item">
            <span class="step-keyword">{{step.keyword}}</span>
            <span class="step-text">{{step.text}}</span>
          </div>
        </div>
      </div>

      <div class="examples-section" *ngIf="scenario.examples && scenario.examples.length > 0">
        <h2>Examples</h2>
        <div *ngFor="let example of scenario.examples" class="example-table">
          <table>
            <thead>
              <tr>
                <th *ngFor="let header of example.headers">{{header}}</th>
              </tr>
            </thead>
            <tbody>
              <tr *ngFor="let row of example.rows">
                <td *ngFor="let cell of row">{{cell}}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .scenario-detail { padding: 2rem; }
    .scenario-header { margin-bottom: 2rem; }
    .breadcrumb { font-size: 0.875rem; color: #64748b; margin-bottom: 1rem; }
    .breadcrumb a { color: #6366f1; text-decoration: none; }
    .scenario-title { font-size: 2rem; margin: 0 0 1rem; color: #eab308; }
    .scenario-meta { display: flex; gap: 0.75rem; }
    .meta-badge { padding: 0.5rem 1rem; background: rgba(234, 179, 8, 0.15); border: 1px solid rgba(234, 179, 8, 0.3); border-radius: 0.5rem; font-size: 0.875rem; }
    h2 { color: #e0e6ed; margin: 2rem 0 1rem; }
    .steps-list { background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(99, 102, 241, 0.2); border-radius: 1rem; padding: 1.5rem; }
    .step-item { padding: 0.75rem; margin-bottom: 0.5rem; border-left: 3px solid rgba(234, 179, 8, 0.5); }
    .step-keyword { font-weight: 600; color: #eab308; margin-right: 0.5rem; }
    .step-text { color: #cbd5e1; }
    table { width: 100%; border-collapse: collapse; background: rgba(15, 23, 42, 0.6); border-radius: 0.5rem; overflow: hidden; }
    th, td { padding: 0.75rem; text-align: left; border: 1px solid rgba(99, 102, 241, 0.2); }
    th { background: rgba(99, 102, 241, 0.15); color: #a5b4fc; font-weight: 600; }
    td { color: #cbd5e1; }
  `]
})
export class ScenarioDetailComponent implements OnInit {
  scenario: ScenarioNode | null = null;
  feature: FeatureNode | null = null;

  constructor(
    private readonly route: ActivatedRoute,
    private readonly documentGraphService: DocumentGraphService
  ) {}

  ngOnInit(): void {
    this.route.params.subscribe(params => {
      const node = this.documentGraphService.getNode(params['id']);
      if (node && node.type === 'scenario') {
        this.scenario = node as ScenarioNode;
        const featureNode = this.documentGraphService.getNode(this.scenario.featureId);
        if (featureNode?.type === 'feature') {
          this.feature = featureNode as FeatureNode;
        }
      }
    });
  }
}
