import { Component, OnInit, inject, signal } from '@angular/core';
import { ActivatedRoute, Router, RouterLink } from '@angular/router';

import { ContentNode } from '@app/lamad/models/content-node.model';

import {
  parseWorkStoryMeta,
  type WorkStoryMeta,
  type WorkStoryStatus,
  type WorkPriority,
  type WorkVisibility,
} from '../../models/work-story.model';
import { AvodahApiService } from '../../services/avodah-api.service';

@Component({
  selector: 'app-story-detail',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="detail-shell">
      @if (story) {
        <header class="detail-header">
          <a
            [routerLink]="['/avodah/projects', projectId, 'board']"
            class="back-link"
            data-testid="back-to-board"
          >
            &larr; Back to Board
          </a>
          <span class="project-ref">{{ projectTitle }}</span>
        </header>

        <main class="detail-body">
          @if (editingTitle()) {
            <input
              class="title-input"
              [value]="story.title"
              data-testid="title-input"
              (keydown.enter)="saveTitle($event)"
              (keydown.escape)="editingTitle.set(false)"
              (blur)="saveTitle($event)"
            />
          } @else {
            <h1
              class="story-title"
              data-testid="story-title"
              (click)="editingTitle.set(true)"
            >
              {{ story.title }}
            </h1>
          }

          @if (editingDescription()) {
            <textarea
              class="desc-input"
              [value]="story.description"
              data-testid="desc-input"
              rows="3"
              (keydown.escape)="editingDescription.set(false)"
              (blur)="saveDescription($event)"
            ></textarea>
          } @else {
            <p
              class="story-desc"
              data-testid="story-desc"
              (click)="editingDescription.set(true)"
            >
              {{ story.description || 'Click to add a description...' }}
            </p>
          }

          <div class="meta-row">
            <div class="meta-card">
              <label>Status</label>
              <select
                [value]="meta().status"
                (change)="changeStatus($event)"
                data-testid="status-select"
              >
                <option value="backlog">Backlog</option>
                <option value="todo">To Do</option>
                <option value="in-progress">In Progress</option>
                <option value="review">Review</option>
                <option value="done">Done</option>
              </select>
            </div>
            <div class="meta-card">
              <label>Priority</label>
              <select
                [value]="meta().priority"
                (change)="changePriority($event)"
                data-testid="priority-select"
              >
                <option value="low">Low</option>
                <option value="medium">Medium</option>
                <option value="high">High</option>
                <option value="urgent">Urgent</option>
              </select>
            </div>
            <div class="meta-card">
              <label>Visibility</label>
              <select
                [value]="meta().visibility"
                (change)="changeVisibility($event)"
                data-testid="visibility-select"
              >
                <option value="private">Private</option>
                <option value="community">Community</option>
                <option value="exchange">Exchange</option>
              </select>
            </div>
          </div>

          <div class="meta-row">
            <div class="meta-card">
              <label>Assigned</label>
              <span class="meta-value">{{
                meta().assigneeId ? '@' + meta().assigneeId : 'Unassigned'
              }}</span>
            </div>
            <div class="meta-card">
              <label>Story Points</label>
              <span class="meta-value">{{ meta().storyPoints ?? '—' }}</span>
            </div>
          </div>

          <div class="section">
            <label>Tags</label>
            <div class="tags-row">
              @for (tag of story.tags; track tag) {
                <span class="tag">#{{ tag }}</span>
              }
              @if (story.tags.length === 0) {
                <span class="empty-hint">No tags</span>
              }
            </div>
          </div>

          <div class="section">
            <label>Cadence</label>
            @if (meta().cadence) {
              <span class="meta-value">
                {{ meta().cadence!.interval }}
                — next: {{ formatDate(meta().cadence!.nextOccurrence) }}
              </span>
            } @else {
              <span class="empty-hint">One-time story (no recurrence)</span>
            }
          </div>

          <div class="section">
            <label>Attestation Gates</label>
            @if (meta().attestationGates?.length) {
              <ul class="gates-list">
                @for (gate of meta().attestationGates!; track gate) {
                  <li>{{ gate }}</li>
                }
              </ul>
            } @else {
              <span class="empty-hint">Open to all — no mastery required</span>
            }
          </div>
        </main>
      } @else {
        <div class="loading">Loading story...</div>
      }
    </div>
  `,
  styles: [
    `
      .detail-shell {
        max-width: 720px;
        margin: 0 auto;
        padding: 2rem 1.5rem;
      }
      .detail-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2rem;
      }
      .back-link {
        color: var(--lamad-accent-primary, #6366f1);
        text-decoration: none;
        font-size: 0.85rem;
      }
      .back-link:hover {
        text-decoration: underline;
      }
      .project-ref {
        font-size: 0.8rem;
        color: var(--lamad-text-muted, #64748b);
      }
      .story-title {
        font-size: 1.5rem;
        font-weight: 600;
        margin: 0 0 0.5rem;
        cursor: pointer;
        border-bottom: 1px dashed transparent;
      }
      .story-title:hover {
        border-bottom-color: rgba(99, 102, 241, 0.3);
      }
      .title-input {
        font-size: 1.5rem;
        font-weight: 600;
        width: 100%;
        background: rgba(15, 15, 26, 0.8);
        border: 1px solid var(--lamad-accent-primary, #6366f1);
        border-radius: 6px;
        color: var(--lamad-text-secondary, #e2e8f0);
        padding: 0.25rem 0.5rem;
        margin-bottom: 0.5rem;
        outline: none;
      }
      .story-desc {
        color: var(--lamad-text-secondary, #e2e8f0);
        font-size: 0.9rem;
        line-height: 1.6;
        cursor: pointer;
        margin: 0 0 1.5rem;
        min-height: 1.5rem;
      }
      .desc-input {
        width: 100%;
        background: rgba(15, 15, 26, 0.8);
        border: 1px solid var(--lamad-accent-primary, #6366f1);
        border-radius: 6px;
        color: var(--lamad-text-secondary, #e2e8f0);
        padding: 0.5rem;
        font-size: 0.9rem;
        resize: vertical;
        margin-bottom: 1.5rem;
        outline: none;
        font-family: inherit;
        box-sizing: border-box;
      }
      .meta-row {
        display: flex;
        gap: 1rem;
        margin-bottom: 1rem;
        flex-wrap: wrap;
      }
      .meta-card {
        flex: 1;
        min-width: 150px;
        background: rgba(15, 15, 26, 0.6);
        border: 1px solid rgba(99, 102, 241, 0.12);
        border-radius: 8px;
        padding: 0.75rem;
      }
      .meta-card label {
        display: block;
        font-size: 0.7rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--lamad-text-muted, #64748b);
        margin-bottom: 0.375rem;
      }
      .meta-card select {
        background: transparent;
        border: none;
        color: var(--lamad-text-secondary, #e2e8f0);
        font-size: 0.85rem;
        cursor: pointer;
        padding: 0;
        width: 100%;
        outline: none;
      }
      .meta-card select option {
        background: #1a1a2e;
      }
      .meta-value {
        font-size: 0.85rem;
        color: var(--lamad-text-secondary, #e2e8f0);
      }
      .section {
        margin-bottom: 1.25rem;
      }
      .section label {
        display: block;
        font-size: 0.7rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--lamad-text-muted, #64748b);
        margin-bottom: 0.375rem;
      }
      .tags-row {
        display: flex;
        gap: 0.375rem;
        flex-wrap: wrap;
      }
      .tag {
        font-size: 0.75rem;
        color: var(--lamad-accent-primary, #6366f1);
        background: rgba(99, 102, 241, 0.1);
        padding: 0.125rem 0.5rem;
        border-radius: 999px;
      }
      .empty-hint {
        font-size: 0.8rem;
        color: var(--lamad-text-muted, #64748b);
        font-style: italic;
      }
      .gates-list {
        list-style: none;
        padding: 0;
        margin: 0;
      }
      .gates-list li {
        font-size: 0.8rem;
        padding: 0.25rem 0;
        color: #a78bfa;
      }
      .loading {
        text-align: center;
        padding: 3rem;
        color: var(--lamad-text-muted, #64748b);
      }
    `,
  ],
})
export class StoryDetailComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly api = inject(AvodahApiService);

  story: ContentNode | null = null;
  projectId = '';
  projectTitle = '';
  readonly editingTitle = signal(false);
  readonly editingDescription = signal(false);

  ngOnInit(): void {
    void this.load();
  }

  meta(): WorkStoryMeta {
    return parseWorkStoryMeta(
      (this.story?.metadata ?? {}) as Record<string, unknown>,
    );
  }

  formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    });
  }

  async changeStatus(event: Event): Promise<void> {
    const status = (event.target as HTMLSelectElement).value as WorkStoryStatus;
    if (!this.story) return;
    await this.api.updateStoryStatus(this.story.id, status);
    (this.story.metadata as Record<string, unknown>)['status'] = status;
  }

  async changePriority(event: Event): Promise<void> {
    const priority = (event.target as HTMLSelectElement).value as WorkPriority;
    if (!this.story) return;
    await this.api.updateStoryField(this.story.id, {
      metadata: { priority },
    });
    (this.story.metadata as Record<string, unknown>)['priority'] = priority;
  }

  async changeVisibility(event: Event): Promise<void> {
    const visibility = (event.target as HTMLSelectElement)
      .value as WorkVisibility;
    if (!this.story) return;
    await this.api.updateStoryField(this.story.id, {
      metadata: { visibility },
    });
    (this.story.metadata as Record<string, unknown>)['visibility'] = visibility;
  }

  async saveTitle(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    const title = input.value.trim();
    if (!title || !this.story || title === this.story.title) {
      this.editingTitle.set(false);
      return;
    }
    await this.api.updateStoryField(this.story.id, { title });
    this.story = { ...this.story, title };
    this.editingTitle.set(false);
  }

  async saveDescription(event: Event): Promise<void> {
    const textarea = event.target as HTMLTextAreaElement;
    const description = textarea.value.trim();
    if (!this.story) {
      this.editingDescription.set(false);
      return;
    }
    await this.api.updateStoryField(this.story.id, { description });
    this.story = { ...this.story, description };
    this.editingDescription.set(false);
  }

  private async load(): Promise<void> {
    this.projectId = this.route.snapshot.params['id'] as string;
    const storyId = this.route.snapshot.params['storyId'] as string;
    const projects = await this.api.getProjects();
    const project = projects.find((p) => p.id === this.projectId);
    this.projectTitle = project?.title ?? 'Project';
    const stories = await this.api.getStoriesForProject(this.projectId);
    this.story = stories.find((s) => s.id === storyId) ?? null;
    if (!this.story) {
      void this.router.navigate([
        '/avodah/projects',
        this.projectId,
        'board',
      ]);
    }
  }
}
