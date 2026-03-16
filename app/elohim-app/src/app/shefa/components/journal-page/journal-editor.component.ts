import {
  Component,
  ChangeDetectionStrategy,
  DestroyRef,
  inject,
  input,
  signal,
} from '@angular/core';
import { Subject, debounceTime, switchMap, takeUntil } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

@Component({
  selector: 'app-journal-editor',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="journal-editor">
      <input
        class="journal-title"
        data-testid="journal-title"
        aria-label="Journal title"
        [value]="title()"
        (input)="onTitleInput($event)"
        (blur)="saveTitle()"
        placeholder="Untitled"
      />
      <span class="save-status" data-testid="save-status">{{ saveStatus() }}</span>
      <textarea
        class="journal-body"
        data-testid="journal-body"
        aria-label="Journal body"
        [value]="body()"
        (input)="onBodyInput($event)"
        placeholder="Start writing..."
      ></textarea>
    </div>
  `,
  styles: [
    `
      .journal-editor {
        display: flex;
        flex-direction: column;
        height: 100%;
        padding: 2rem;
      }

      .journal-title {
        border: none;
        outline: none;
        font-size: 2rem;
        font-weight: 600;
        font-family: inherit;
        color: var(--text-primary, #202124);
        background: transparent;
        padding: 0;
        margin-bottom: 0.5rem;
      }

      .journal-title::placeholder {
        color: var(--text-disabled, #9aa0a6);
      }

      .save-status {
        font-size: 0.75rem;
        color: var(--text-secondary, #5f6368);
        margin-bottom: 1rem;
        min-height: 1rem;
      }

      .journal-body {
        border: none;
        outline: none;
        font-size: 1rem;
        font-family: inherit;
        line-height: 1.75;
        color: var(--text-primary, #202124);
        background: transparent;
        resize: none;
        flex: 1;
        padding: 0;
      }

      .journal-body::placeholder {
        color: var(--text-disabled, #9aa0a6);
      }
    `,
  ],
})
export class JournalEditorComponent {
  readonly contentId = input.required<string>();
  readonly title = signal('');
  readonly body = signal('');
  readonly saveStatus = signal('');

  private readonly storageApi = inject(StorageApiService);
  private readonly destroyRef = inject(DestroyRef);
  private readonly bodyChange$ = new Subject<string>();

  constructor() {
    const destroy$ = new Subject<void>();
    this.destroyRef.onDestroy(() => {
      destroy$.next();
      destroy$.complete();
      this.bodyChange$.complete();
    });

    this.bodyChange$
      .pipe(
        debounceTime(1500),
        switchMap((contentBody) => {
          this.saveStatus.set('Saving...');
          return this.storageApi.updateContent(this.contentId(), { contentBody });
        }),
        takeUntil(destroy$),
      )
      .subscribe({
        next: () => this.saveStatus.set('Saved'),
        error: () => this.saveStatus.set('Save failed'),
      });
  }

  /** Load content from parent (e.g. after fetch). */
  loadContent(title: string, body: string): void {
    this.title.set(title);
    this.body.set(body);
  }

  onTitleInput(event: Event): void {
    this.title.set((event.target as HTMLInputElement).value);
  }

  onBodyInput(event: Event): void {
    const value = (event.target as HTMLTextAreaElement).value;
    this.body.set(value);
    this.bodyChange$.next(value);
  }

  saveTitle(): void {
    const title = this.title();
    if (!title && !this.contentId()) return;
    this.saveStatus.set('Saving...');
    this.storageApi.updateContent(this.contentId(), { title }).subscribe({
      next: () => this.saveStatus.set('Saved'),
      error: () => this.saveStatus.set('Save failed'),
    });
  }
}
