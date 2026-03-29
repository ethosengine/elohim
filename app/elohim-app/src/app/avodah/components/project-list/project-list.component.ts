import { Component, OnInit, inject } from '@angular/core';
import { Router } from '@angular/router';

import { ContentNode } from '@app/lamad/models/content-node.model';

import { parseWorkProjectMeta } from '../../models/work-project.model';
import { AvodahApiService } from '../../services/avodah-api.service';

@Component({
  selector: 'app-project-list',
  standalone: true,
  imports: [],
  template: `
    <div class="projects-shell">
      <header class="projects-header">
        <h1 class="projects-title">Projects</h1>
        <button class="btn-new" data-testid="btn-new-project">+ New Project</button>
      </header>
      @if (projects.length === 0) {
        <div class="empty-state">
          <p>No projects yet.</p>
          <p class="hint">Create a project to start tracking work stories.</p>
        </div>
      }
      <div class="project-grid">
        @for (project of projects; track project.id) {
          <div class="project-card" data-testid="project-card">
            <div class="card-body">
              <h2 class="card-title">{{ project.title }}</h2>
              @if (project.description) {
                <p class="card-desc">{{ project.description }}</p>
              }
              <div class="card-meta">
                <span class="vis-badge vis-{{ projectVisibility(project) }}">
                  {{ projectVisibility(project) }}
                </span>
              </div>
              @if (project.tags.length > 0) {
                <div class="card-tags">
                  @for (tag of project.tags; track tag) {
                    <span class="tag">#{{ tag }}</span>
                  }
                </div>
              }
            </div>
            <div class="card-actions">
              <button
                class="action-btn"
                (click)="navigate(project, 'board')"
                data-testid="btn-board"
              >
                ▦ Board
              </button>
              <button
                class="action-btn"
                (click)="navigate(project, 'backlog')"
                data-testid="btn-backlog"
              >
                ≡ Backlog
              </button>
              <button
                class="action-btn"
                (click)="navigate(project, 'tasks')"
                data-testid="btn-tasks"
              >
                ↺ Tasks
              </button>
            </div>
          </div>
        }
      </div>
    </div>
  `,
  styles: [
    `
      .projects-shell {
        padding: 1.5rem 2rem;
        max-width: 1100px;
        margin: 0 auto;
      }
      .projects-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 1.5rem;
      }
      .projects-title {
        font-size: 1.4rem;
        font-weight: 700;
        margin: 0;
      }
      .btn-new {
        background: var(--lamad-accent-primary, #6366f1);
        color: white;
        border: none;
        border-radius: 8px;
        padding: 0.5rem 1.25rem;
        font-size: 0.85rem;
        cursor: pointer;
        font-weight: 500;
      }
      .btn-new:hover {
        opacity: 0.9;
      }
      .empty-state {
        text-align: center;
        padding: 4rem 1rem;
        color: var(--lamad-text-muted, #64748b);
      }
      .hint {
        font-size: 0.8rem;
        margin-top: 0.5rem;
      }
      .project-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
        gap: 1.25rem;
      }
      .project-card {
        background: rgba(15, 15, 26, 0.7);
        border: 1px solid rgba(99, 102, 241, 0.15);
        border-radius: 12px;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        transition:
          border-color 0.15s ease,
          box-shadow 0.15s ease;
      }
      .project-card:hover {
        border-color: rgba(99, 102, 241, 0.4);
        box-shadow: 0 4px 20px rgba(99, 102, 241, 0.12);
      }
      .card-body {
        padding: 1.25rem;
        flex: 1;
      }
      .card-title {
        font-size: 1rem;
        font-weight: 600;
        margin: 0 0 0.5rem;
        color: var(--lamad-text-primary, #f1f5f9);
      }
      .card-desc {
        font-size: 0.8rem;
        color: var(--lamad-text-muted, #64748b);
        margin: 0 0 0.75rem;
        line-height: 1.5;
      }
      .card-meta {
        margin-bottom: 0.75rem;
      }
      .vis-badge {
        font-size: 0.7rem;
        padding: 0.15rem 0.5rem;
        border-radius: 999px;
        font-weight: 500;
      }
      .vis-private {
        background: rgba(71, 85, 105, 0.2);
        color: #94a3b8;
      }
      .vis-community {
        background: rgba(59, 130, 246, 0.15);
        color: #60a5fa;
      }
      .card-tags {
        display: flex;
        flex-wrap: wrap;
        gap: 0.35rem;
      }
      .tag {
        font-size: 0.7rem;
        color: var(--lamad-text-muted, #64748b);
        background: rgba(99, 102, 241, 0.08);
        border-radius: 4px;
        padding: 0.1rem 0.4rem;
      }
      .card-actions {
        display: flex;
        border-top: 1px solid rgba(99, 102, 241, 0.1);
      }
      .action-btn {
        flex: 1;
        background: none;
        border: none;
        border-right: 1px solid rgba(99, 102, 241, 0.1);
        padding: 0.65rem 0.5rem;
        font-size: 0.78rem;
        color: var(--lamad-text-muted, #64748b);
        cursor: pointer;
        transition:
          background 0.12s ease,
          color 0.12s ease;
      }
      .action-btn:last-child {
        border-right: none;
      }
      .action-btn:hover {
        background: rgba(99, 102, 241, 0.1);
        color: var(--lamad-accent-primary, #6366f1);
      }
    `,
  ],
})
export class ProjectListComponent implements OnInit {
  private readonly router = inject(Router);
  private readonly api = inject(AvodahApiService);

  projects: ContentNode[] = [];

  ngOnInit(): void {
    void this.load();
  }

  private async load(): Promise<void> {
    this.projects = await this.api.getProjects();
  }

  navigate(project: ContentNode, view: string): void {
    void this.router.navigate(['/avodah/projects', project.id, view]);
  }

  projectVisibility(project: ContentNode): string {
    const meta = parseWorkProjectMeta(project.metadata as Record<string, unknown>);
    return meta.visibility ?? 'private';
  }
}
