import { Component, OnInit, inject } from '@angular/core';
import { RouterLink, ActivatedRoute, Router } from '@angular/router';

import { ContentNode } from '@app/lamad/models/content-node.model';

import {
  BoardColumn,
  DEFAULT_BOARD_COLUMNS,
  parseWorkProjectMeta,
} from '../../models/work-project.model';
import { parseWorkStoryMeta, type WorkStoryStatus } from '../../models/work-story.model';
import { AvodahApiService } from '../../services/avodah-api.service';
import { StoryCardComponent } from '../story-card/story-card.component';

@Component({
  selector: 'app-project-board',
  standalone: true,
  imports: [RouterLink, StoryCardComponent],
  template: `
    <div class="board-shell">
      <nav class="board-sidebar">
        <div class="project-name">{{ project?.title ?? 'Project' }}</div>
        <ul>
          <li class="active">▦ Board</li>
          <li><a [routerLink]="['../backlog']" data-testid="nav-backlog">≡ Backlog</a></li>
          <li><a [routerLink]="['../tasks']" data-testid="nav-tasks">↺ Tasks</a></li>
        </ul>
        <a routerLink="/avodah/projects" class="new-project" data-testid="nav-projects">
          + New Project
        </a>
      </nav>
      <main class="board-main">
        <div class="columns">
          @for (col of columns; track col.id) {
            <div class="column">
              <div class="col-header">
                <span>{{ col.name }}</span>
                <span class="col-count">{{ storiesInColumn(col.id).length }}</span>
              </div>
              <div
                class="col-stories"
                [attr.data-column-id]="col.id"
                (dragover)="onDragOver($event)"
                (dragleave)="onDragLeave($event)"
                (drop)="onDrop($event, col)"
              >
                @for (story of storiesInColumn(col.id); track story.id) {
                  <app-story-card
                    [story]="story"
                    draggable="true"
                    [attr.data-story-id]="story.id"
                    (dragstart)="onDragStart($event, story)"
                    (cardClick)="openStory(story)"
                  />
                }
              </div>
              @if (addingInColumn === col.id) {
                <input
                  class="add-story-input"
                  placeholder="Story title…"
                  data-testid="add-story-input"
                  (keydown.enter)="submitNewStory($event, col.id)"
                  (keydown.escape)="addingInColumn = null"
                  (blur)="addingInColumn = null"
                />
              } @else {
                <div
                  class="add-story"
                  data-testid="add-story-btn"
                  (click)="addingInColumn = col.id"
                >+ Add story</div>
              }
            </div>
          }
        </div>
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
      .board-main {
        flex: 1;
        overflow-x: auto;
        padding: 1rem;
      }
      .columns {
        display: flex;
        gap: 1rem;
        align-items: flex-start;
      }
      .column {
        width: 280px;
        flex-shrink: 0;
        background: rgba(15, 15, 26, 0.6);
        border-radius: 10px;
        padding: 0.75rem;
      }
      .col-header {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.75rem;
        font-weight: 600;
        font-size: 0.85rem;
      }
      .col-count {
        background: rgba(99, 102, 241, 0.2);
        border-radius: 999px;
        padding: 1px 7px;
        font-size: 0.75rem;
        color: var(--lamad-text-muted, #64748b);
      }
      .col-stories {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }
      .add-story {
        padding: 0.5rem;
        font-size: 0.8rem;
        color: var(--lamad-text-muted, #64748b);
        cursor: pointer;
        border-radius: 6px;
        text-align: center;
        border: 1px dashed rgba(99, 102, 241, 0.15);
        margin-top: 0.5rem;
      }
      .add-story:hover {
        border-color: var(--lamad-accent-primary, #6366f1);
        color: var(--lamad-accent-primary, #6366f1);
      }
      .col-stories.drop-target {
        background: rgba(99, 102, 241, 0.08);
        border-radius: 6px;
        outline: 2px dashed rgba(99, 102, 241, 0.3);
      }
      .add-story-input {
        width: 100%;
        padding: 0.5rem;
        font-size: 0.8rem;
        border: 1px solid var(--lamad-accent-primary, #6366f1);
        border-radius: 6px;
        background: rgba(15, 15, 26, 0.8);
        color: var(--lamad-text-secondary, #e2e8f0);
        margin-top: 0.5rem;
        outline: none;
        box-sizing: border-box;
      }
      app-story-card[draggable='true'] {
        cursor: grab;
      }
      app-story-card[draggable='true']:active {
        cursor: grabbing;
        opacity: 0.6;
      }
    `,
  ],
})
export class ProjectBoardComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly api = inject(AvodahApiService);
  private draggedStoryId: string | null = null;

  addingInColumn: string | null = null;
  project: ContentNode | null = null;
  stories: ContentNode[] = [];
  columns: BoardColumn[] = DEFAULT_BOARD_COLUMNS;

  ngOnInit(): void {
    void this.load();
  }

  private async load(): Promise<void> {
    const projectId = this.route.snapshot.params['id'] as string;
    const projects = await this.api.getProjects();
    this.project = projects.find(p => p.id === projectId) ?? null;
    if (this.project) {
      const meta = parseWorkProjectMeta(this.project.metadata as Record<string, unknown>);
      this.columns = meta.columns;
    }
    this.stories = await this.api.getStoriesForProject(projectId);
  }

  storiesInColumn(columnId: string): ContentNode[] {
    return this.stories.filter(s => {
      const meta = parseWorkStoryMeta(s.metadata as Record<string, unknown>);
      return meta.status === columnId;
    });
  }

  onDragStart(event: DragEvent, story: ContentNode): void {
    this.draggedStoryId = story.id;
    event.dataTransfer?.setData('text/plain', story.id);
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  onDragOver(event: DragEvent): void {
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
    (event.currentTarget as HTMLElement).classList.add('drop-target');
  }

  onDragLeave(event: DragEvent): void {
    (event.currentTarget as HTMLElement).classList.remove('drop-target');
  }

  onDrop(event: DragEvent, column: BoardColumn): void {
    event.preventDefault();
    (event.currentTarget as HTMLElement).classList.remove('drop-target');
    const storyId = this.draggedStoryId;
    this.draggedStoryId = null;
    if (!storyId) return;

    const story = this.stories.find(s => s.id === storyId);
    if (story) {
      (story.metadata as Record<string, unknown>)['status'] = column.id;
    }
    void this.api.updateStoryStatus(
      storyId,
      column.id as WorkStoryStatus,
      column.isTerminal ?? false,
    );
  }

  async submitNewStory(event: Event, columnId: string): Promise<void> {
    const input = event.target as HTMLInputElement;
    const title = input.value.trim();
    if (!title) return;
    const projectId = this.route.snapshot.params['id'] as string;
    const newStory = await this.api.createStory(projectId, title, columnId as WorkStoryStatus);
    this.stories = [...this.stories, newStory];
    this.addingInColumn = null;
  }

  openStory(story: ContentNode): void {
    const projectId = this.route.snapshot.params['id'] as string;
    void this.router.navigate(['/avodah/projects', projectId, 'stories', story.id]);
  }
}
