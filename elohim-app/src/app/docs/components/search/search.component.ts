import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { DocumentGraphService } from '../../services/document-graph.service';
import { DocumentNode } from '../../models';

@Component({
  selector: 'app-search',
  standalone: true,
  imports: [CommonModule, RouterLink, FormsModule],
  template: `
    <div class="search-page">
      <h1>Search Documentation</h1>
      <div class="search-form">
        <input [(ngModel)]="query" (keyup.enter)="performSearch()" placeholder="Search..." class="search-input" />
        <button (click)="performSearch()" class="search-btn">Search</button>
      </div>

      <div class="results-section" *ngIf="hasSearched">
        <h2>Results ({{results.length}})</h2>
        <div class="results-list" *ngIf="results.length > 0">
          <a *ngFor="let node of results"
             [routerLink]="getNodeRoute(node)"
             class="result-card">
            <div class="result-type">{{getNodeTypeIcon(node.type)}} {{node.type}}</div>
            <h3>{{node.title}}</h3>
            <p>{{node.description}}</p>
            <div class="result-tags">
              <span *ngFor="let tag of node.tags.slice(0, 3)" class="tag">{{tag}}</span>
            </div>
          </a>
        </div>
        <div class="no-results" *ngIf="results.length === 0">
          <p>No results found for "{{query}}"</p>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .search-page { padding: 2rem; }
    h1 { font-size: 2rem; margin-bottom: 2rem; color: #e0e6ed; }
    .search-form { display: flex; gap: 1rem; margin-bottom: 3rem; }
    .search-input { flex: 1; padding: 1rem; background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(99, 102, 241, 0.3); border-radius: 0.5rem; color: #e0e6ed; font-size: 1rem; }
    .search-btn { padding: 1rem 2rem; background: linear-gradient(135deg, #6366f1, #8b5cf6); border: none; border-radius: 0.5rem; color: white; cursor: pointer; }
    h2 { color: #94a3b8; margin-bottom: 1.5rem; }
    .results-list { display: flex; flex-direction: column; gap: 1rem; }
    .result-card { background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(99, 102, 241, 0.2); border-radius: 1rem; padding: 1.5rem; text-decoration: none; color: inherit; transition: all 0.2s; }
    .result-card:hover { border-color: rgba(99, 102, 241, 0.5); transform: translateY(-2px); }
    .result-type { font-size: 0.75rem; text-transform: uppercase; color: #6366f1; margin-bottom: 0.5rem; }
    .result-card h3 { margin: 0 0 0.5rem; color: #e0e6ed; }
    .result-card p { color: #94a3b8; margin: 0 0 0.75rem; }
    .result-tags { display: flex; gap: 0.5rem; flex-wrap: wrap; }
    .tag { padding: 0.25rem 0.75rem; background: rgba(99, 102, 241, 0.15); border-radius: 1rem; font-size: 0.75rem; color: #a5b4fc; }
    .no-results { text-align: center; padding: 3rem; color: #64748b; }
  `]
})
export class SearchComponent implements OnInit {
  query = '';
  results: DocumentNode[] = [];
  hasSearched = false;

  constructor(
    private readonly route: ActivatedRoute,
    private readonly documentGraphService: DocumentGraphService
  ) {}

  ngOnInit(): void {
    this.route.queryParams.subscribe(params => {
      if (params['q']) {
        this.query = params['q'];
        this.performSearch();
      }
    });
  }

  performSearch(): void {
    if (!this.query.trim()) return;
    this.hasSearched = true;
    this.results = this.documentGraphService.searchNodes(this.query);
  }

  getNodeRoute(node: DocumentNode): string[] {
    return ['/docs', node.type, node.id];
  }

  getNodeTypeIcon(type: string): string {
    const icons: Record<string, string> = { epic: '📖', feature: '⚡', scenario: '✓' };
    return icons[type] || '📄';
  }
}
