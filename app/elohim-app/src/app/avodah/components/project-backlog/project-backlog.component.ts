import { Component, OnInit, inject, signal } from '@angular/core';
import { RouterLink, ActivatedRoute } from '@angular/router';

import { ContentNode } from '@app/lamad/models/content-node.model';

import { parseWorkStoryMeta } from '../../models/work-story.model';
import { AvodahApiService } from '../../services/avodah-api.service';

@Component({
  selector: 'app-project-backlog',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="board-shell">
      <nav class="board-sidebar">
        <div class="project-name">{{ projectId }}</div>
        <ul>
          <li><a [routerLink]="['../board']" data-testid="nav-board">▦ Board</a></li>
          <li class="active">≡ Backlog</li>
          <li><a [routerLink]="['../tasks']" data-testid="nav-tasks">↺ Tasks</a></li>
        </ul>
        <a routerLink="/avodah/projects" class="new-project" data-testid="nav-projects">
          + New Project
        </a>
      </nav>
      <main class="backlog-main">
        <div class="filter-bar">
          <label for="filter-status" class="sr-only">Filter by status</label>
          <select
            id="filter-status"
            (change)="filterStatus($event)"
            data-testid="filter-status"
            aria-label="Filter by status"
          >
            <option value="">All Statuses</option>
            <option value="backlog">Backlog</option>
            <option value="todo">To Do</option>
            <option value="in-progress">In Progress</option>
            <option value="review">Review</option>
            <option value="done">Done</option>
          </select>
          <label for="filter-priority" class="sr-only">Filter by priority</label>
          <select
            id="filter-priority"
            (change)="filterPriority($event)"
            data-testid="filter-priority"
            aria-label="Filter by priority"
          >
            <option value="">All Priorities</option>
            <option value="urgent">Urgent</option>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>
          <button class="btn-new" data-testid="btn-new-story">+ New Story</button>
        </div>
        <table class="backlog-table">
          <thead>
            <tr>
              <th>Title</th>
              <th>Status</th>
              <th>Priority</th>
              <th>Points</th>
              <th>Cadence</th>
              <th>Visibility</th>
            </tr>
          </thead>
          <tbody>
            @for (story of filteredStories(); track story.id) {
              <tr data-testid="backlog-row">
                <td class="title-cell">{{ story.title }}</td>
                <td>
                  <span class="status-badge status-{{ storyMeta(story).status }}">
                    {{ storyMeta(story).status }}
                  </span>
                </td>
                <td>
                  <span class="priority-badge priority-{{ storyMeta(story).priority }}">
                    {{ storyMeta(story).priority }}
                  </span>
                </td>
                <td class="points-cell">
                  @if (storyMeta(story).storyPoints !== undefined) {
                    {{ storyMeta(story).storyPoints }}pt
                  } @else {
                    —
                  }
                </td>
                <td class="cadence-cell">
                  @if (storyMeta(story).cadence) {
                    <span class="cadence-badge">{{ storyMeta(story).cadence!.interval }}</span>
                  } @else {
                    —
                  }
                </td>
                <td>
                  <span class="vis-badge vis-{{ storyMeta(story).visibility }}">
                    {{ storyMeta(story).visibility }}
                  </span>
                </td>
              </tr>
            }
            @if (filteredStories().length === 0) {
              <tr>
                <td colspan="6" class="empty">No stories match the current filters.</td>
              </tr>
            }
          </tbody>
        </table>
      </main>
    </div>
  `,
  styles: [
    `
      .board-shell {
        display: flex;
        height: calc(100vh - 60px);
        overflow: hidden;
      }
      .board-sidebar {
        width: 200px;
        flex-shrink: 0;
        background: rgba(20, 20, 36, 0.95);
        border-right: 1px solid rgba(99, 102, 241, 0.12);
        display: flex;
        flex-direction: column;
        padding: 1rem 0;
      }
      .project-name {
        padding: 0 1rem 1rem;
        border-bottom: 1px solid rgba(99, 102, 241, 0.12);
        font-weight: 600;
        font-size: 0.9rem;
      }
      ul {
        list-style: none;
        padding: 0;
        margin: 0.5rem 0 0;
        flex: 1;
      }
      li {
        padding: 0.6rem 1rem;
        font-size: 0.85rem;
        cursor: pointer;
        border-radius: 0 6px 6px 0;
        margin-right: 0.5rem;
      }
      li.active {
        background: var(--lamad-accent-primary, #6366f1);
        color: white;
      }
      li a {
        color: var(--lamad-text-secondary, #e2e8f0);
        text-decoration: none;
        display: block;
      }
      li:not(.active):hover {
        background: rgba(99, 102, 241, 0.1);
      }
      .new-project {
        padding: 0.75rem 1rem;
        font-size: 0.8rem;
        color: var(--lamad-accent-primary, #6366f1);
        text-decoration: none;
      }
      .backlog-main {
        flex: 1;
        overflow-y: auto;
        padding: 1rem 1.5rem;
      }
      .filter-bar {
        display: flex;
        gap: 0.75rem;
        align-items: center;
        margin-bottom: 1rem;
      }
      select {
        background: rgba(15, 15, 26, 0.8);
        border: 1px solid rgba(99, 102, 241, 0.25);
        color: var(--lamad-text-secondary, #e2e8f0);
        border-radius: 6px;
        padding: 0.4rem 0.75rem;
        font-size: 0.8rem;
        cursor: pointer;
      }
      .btn-new {
        margin-left: auto;
        background: var(--lamad-accent-primary, #6366f1);
        color: white;
        border: none;
        border-radius: 6px;
        padding: 0.4rem 1rem;
        font-size: 0.8rem;
        cursor: pointer;
      }
      .btn-new:hover {
        opacity: 0.9;
      }
      .backlog-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.85rem;
      }
      .backlog-table th {
        text-align: left;
        padding: 0.5rem 0.75rem;
        color: var(--lamad-text-muted, #64748b);
        font-weight: 500;
        border-bottom: 1px solid rgba(99, 102, 241, 0.12);
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
      }
      .backlog-table td {
        padding: 0.6rem 0.75rem;
        border-bottom: 1px solid rgba(99, 102, 241, 0.06);
        color: var(--lamad-text-secondary, #e2e8f0);
        vertical-align: middle;
      }
      .backlog-table tr:hover td {
        background: rgba(99, 102, 241, 0.04);
      }
      .title-cell {
        font-weight: 500;
      }
      .points-cell,
      .cadence-cell {
        color: var(--lamad-text-muted, #64748b);
        font-size: 0.8rem;
      }
      .empty {
        text-align: center;
        color: var(--lamad-text-muted, #64748b);
        padding: 2rem !important;
      }
      .status-badge,
      .priority-badge,
      .cadence-badge,
      .vis-badge {
        font-size: 0.7rem;
        padding: 0.15rem 0.5rem;
        border-radius: 999px;
        font-weight: 500;
      }
      .status-backlog {
        background: rgba(100, 116, 139, 0.15);
        color: #94a3b8;
      }
      .status-todo {
        background: rgba(99, 102, 241, 0.15);
        color: #818cf8;
      }
      .status-in-progress {
        background: rgba(245, 158, 11, 0.15);
        color: #fbbf24;
      }
      .status-review {
        background: rgba(139, 92, 246, 0.15);
        color: #a78bfa;
      }
      .status-done {
        background: rgba(16, 185, 129, 0.15);
        color: #34d399;
      }
      .priority-urgent {
        background: rgba(239, 68, 68, 0.15);
        color: #f87171;
      }
      .priority-high {
        background: rgba(245, 158, 11, 0.15);
        color: #fbbf24;
      }
      .priority-medium {
        background: rgba(99, 102, 241, 0.15);
        color: #818cf8;
      }
      .priority-low {
        background: rgba(71, 85, 105, 0.15);
        color: #94a3b8;
      }
      .cadence-badge {
        background: rgba(16, 185, 129, 0.15);
        color: #34d399;
      }
      .vis-private {
        background: rgba(71, 85, 105, 0.15);
        color: #94a3b8;
      }
      .vis-community {
        background: rgba(59, 130, 246, 0.15);
        color: #60a5fa;
      }
      .vis-exchange {
        background: rgba(245, 158, 11, 0.15);
        color: #fbbf24;
      }
    `,
  ],
})
export class ProjectBacklogComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly api = inject(AvodahApiService);

  projectId = '';
  stories: ContentNode[] = [];

  readonly activeStatusFilter = signal<string>('');
  readonly activePriorityFilter = signal<string>('');

  ngOnInit(): void {
    void this.load();
  }

  private async load(): Promise<void> {
    this.projectId = this.route.snapshot.params['id'] as string;
    this.stories = await this.api.getStoriesForProject(this.projectId);
  }

  storyMeta(story: ContentNode) {
    return parseWorkStoryMeta(story.metadata as Record<string, unknown>);
  }

  filteredStories(): ContentNode[] {
    return this.stories.filter(s => {
      const meta = this.storyMeta(s);
      const statusOk = !this.activeStatusFilter() || meta.status === this.activeStatusFilter();
      const priorityOk =
        !this.activePriorityFilter() || meta.priority === this.activePriorityFilter();
      return statusOk && priorityOk;
    });
  }

  filterStatus(event: Event): void {
    this.activeStatusFilter.set((event.target as HTMLSelectElement).value);
  }

  filterPriority(event: Event): void {
    this.activePriorityFilter.set((event.target as HTMLSelectElement).value);
  }
}
