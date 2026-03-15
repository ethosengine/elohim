import { Component, OnInit, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { ActivatedRoute } from '@angular/router';

import { ContentNode } from '@app/lamad/models/content-node.model';

import { CadenceInterval, parseWorkStoryMeta } from '../../models/work-story.model';
import { AvodahApiService } from '../../services/avodah-api.service';

@Component({
  selector: 'app-task-list',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="board-shell">
      <nav class="board-sidebar">
        <div class="project-name">{{ projectId }}</div>
        <ul>
          <li><a [routerLink]="['../board']" data-testid="nav-board">▦ Board</a></li>
          <li><a [routerLink]="['../backlog']" data-testid="nav-backlog">≡ Backlog</a></li>
          <li class="active">↺ Tasks</li>
        </ul>
        <a routerLink="/avodah/projects" class="new-project" data-testid="nav-projects">+ New Project</a>
      </nav>
      <main class="tasks-main">
        @if (recurringStories.length === 0) {
          <div class="empty-state">
            <p>No recurring tasks for this project.</p>
            <p class="hint">Add a cadence to a story in the backlog to make it recurring.</p>
          </div>
        }
        @for (group of groups; track group) {
          @if (storiesInGroup(group).length > 0) {
            <section class="group-section">
              <h2 class="group-heading">
                <span class="group-icon">{{ groupIcon(group) }}</span>
                {{ group | titlecase }}
                <span class="group-count">{{ storiesInGroup(group).length }}</span>
              </h2>
              <ul class="task-rows" role="list">
                @for (story of storiesInGroup(group); track story.id) {
                  <li class="task-row" data-testid="task-row">
                    <button
                      class="check-btn"
                      [attr.aria-label]="'Mark complete: ' + story.title"
                      data-testid="task-check"
                    >○</button>
                    <span class="task-title">{{ story.title }}</span>
                    <span class="task-next">
                      {{ nextOccurrence(story) }}
                    </span>
                    @if (hasExchange(story)) {
                      <span class="badge badge--exchange" data-testid="badge-exchange">⚖ exchange</span>
                    }
                    @if (hasAttestation(story)) {
                      <span class="badge badge--attestation" data-testid="badge-attestation">🎓 attested</span>
                    }
                  </li>
                }
              </ul>
            </section>
          }
        }
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
      .tasks-main {
        flex: 1;
        overflow-y: auto;
        padding: 1rem 1.5rem;
      }
      .empty-state {
        text-align: center;
        padding: 3rem 1rem;
        color: var(--lamad-text-muted, #64748b);
      }
      .hint {
        font-size: 0.8rem;
        margin-top: 0.5rem;
      }
      .group-section {
        margin-bottom: 2rem;
      }
      .group-heading {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.85rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: var(--lamad-text-muted, #64748b);
        margin: 0 0 0.75rem;
        padding-bottom: 0.5rem;
        border-bottom: 1px solid rgba(99, 102, 241, 0.1);
      }
      .group-icon {
        font-size: 1rem;
      }
      .group-count {
        margin-left: auto;
        background: rgba(99, 102, 241, 0.2);
        border-radius: 999px;
        padding: 1px 7px;
        font-size: 0.7rem;
        color: var(--lamad-text-muted, #64748b);
        font-weight: 500;
        text-transform: none;
        letter-spacing: 0;
      }
      .task-rows {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
      }
      .task-row {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.5rem 0.75rem;
        border-radius: 8px;
        background: rgba(15, 15, 26, 0.5);
        border: 1px solid rgba(99, 102, 241, 0.08);
        font-size: 0.85rem;
      }
      .task-row:hover {
        background: rgba(99, 102, 241, 0.06);
        border-color: rgba(99, 102, 241, 0.18);
      }
      .check-btn {
        background: none;
        border: 1px solid rgba(99, 102, 241, 0.3);
        border-radius: 50%;
        width: 22px;
        height: 22px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.75rem;
        cursor: pointer;
        color: var(--lamad-text-muted, #64748b);
        flex-shrink: 0;
        padding: 0;
      }
      .check-btn:hover {
        border-color: var(--lamad-accent-primary, #6366f1);
        color: var(--lamad-accent-primary, #6366f1);
      }
      .task-title {
        flex: 1;
        color: var(--lamad-text-secondary, #e2e8f0);
      }
      .task-next {
        font-size: 0.75rem;
        color: var(--lamad-text-muted, #64748b);
        white-space: nowrap;
      }
      .badge {
        font-size: 0.65rem;
        padding: 0.1rem 0.45rem;
        border-radius: 999px;
        font-weight: 500;
      }
      .badge--exchange {
        background: rgba(245, 158, 11, 0.15);
        color: #fbbf24;
        border: 1px solid rgba(245, 158, 11, 0.3);
      }
      .badge--attestation {
        background: rgba(139, 92, 246, 0.15);
        color: #a78bfa;
        border: 1px solid rgba(139, 92, 246, 0.3);
      }
    `,
  ],
})
export class TaskListComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private api = inject(AvodahApiService);

  readonly groups = ['daily', 'weekly', 'monthly', 'custom'] as const;

  projectId = '';
  recurringStories: ContentNode[] = [];

  async ngOnInit(): Promise<void> {
    this.projectId = this.route.snapshot.params['id'] as string;
    const all = await this.api.getStoriesForProject(this.projectId);
    this.recurringStories = all.filter(s => {
      const meta = parseWorkStoryMeta(s.metadata as Record<string, unknown>);
      return meta.cadence !== undefined;
    });
  }

  storiesInGroup(interval: CadenceInterval): ContentNode[] {
    return this.recurringStories.filter(s => {
      const meta = parseWorkStoryMeta(s.metadata as Record<string, unknown>);
      return meta.cadence?.interval === interval;
    });
  }

  nextOccurrence(story: ContentNode): string {
    const meta = parseWorkStoryMeta(story.metadata as Record<string, unknown>);
    if (!meta.cadence?.nextOccurrence) return '—';
    const date = new Date(meta.cadence.nextOccurrence);
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }

  hasExchange(story: ContentNode): boolean {
    const meta = parseWorkStoryMeta(story.metadata as Record<string, unknown>);
    return meta.visibility === 'exchange';
  }

  hasAttestation(story: ContentNode): boolean {
    const meta = parseWorkStoryMeta(story.metadata as Record<string, unknown>);
    return Array.isArray(meta.attestationGates) && meta.attestationGates.length > 0;
  }

  groupIcon(interval: CadenceInterval): string {
    const icons: Record<CadenceInterval, string> = {
      daily: '☀',
      weekly: '📅',
      monthly: '🗓',
      custom: '⚙',
    };
    return icons[interval];
  }
}
