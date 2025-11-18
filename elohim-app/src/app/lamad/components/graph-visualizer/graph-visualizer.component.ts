import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-graph-visualizer',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="graph-visualizer">
      <h1>Graph Visualization</h1>
      <div class="coming-soon">
        <div class="icon">🕸️</div>
        <h2>Interactive Graph Coming Soon</h2>
        <p>The visual graph explorer is under development. It will feature:</p>
        <ul>
          <li>Interactive force-directed graph layout</li>
          <li>Color-coded nodes by type (Epic, Feature, Scenario)</li>
          <li>Relationship visualization with different edge styles</li>
          <li>Click-to-navigate functionality</li>
          <li>Zoom and pan controls</li>
          <li>Filter by type, tag, and category</li>
        </ul>
        <a routerLink="/lamad" class="back-link">← Back to Documentation</a>
      </div>
    </div>
  `,
  styles: [`
    .graph-visualizer { padding: 2rem; }
    h1 { font-size: 2rem; margin-bottom: 2rem; color: #e0e6ed; }
    .coming-soon { text-align: center; padding: 4rem 2rem; background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(99, 102, 241, 0.2); border-radius: 1rem; }
    .icon { font-size: 4rem; margin-bottom: 1rem; }
    h2 { color: #6366f1; margin-bottom: 1rem; }
    p { color: #94a3b8; margin-bottom: 1.5rem; }
    ul { text-align: left; max-width: 500px; margin: 0 auto 2rem; color: #cbd5e1; }
    li { margin: 0.5rem 0; }
    .back-link { display: inline-block; padding: 0.75rem 1.5rem; background: linear-gradient(135deg, #6366f1, #8b5cf6); border-radius: 0.5rem; color: white; text-decoration: none; }
  `]
})
export class GraphVisualizerComponent {}
