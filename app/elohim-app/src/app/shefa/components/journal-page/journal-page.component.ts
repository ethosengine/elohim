import {
  Component,
  ChangeDetectionStrategy,
  DestroyRef,
  inject,
  viewChild,
} from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { Subject, switchMap, takeUntil } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

import { JournalEditorComponent } from './journal-editor.component';
import { ElohimSidebarComponent } from './elohim-sidebar.component';

@Component({
  selector: 'app-journal-page',
  standalone: true,
  imports: [JournalEditorComponent, ElohimSidebarComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="journal-layout" data-testid="journal-layout">
      <div class="journal-main">
        <app-journal-editor [contentId]="contentId" />
      </div>
      <div class="journal-sidebar">
        <app-elohim-sidebar />
      </div>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
        height: 100%;
      }

      .journal-layout {
        display: flex;
        height: 100%;
      }

      .journal-main {
        flex: 1;
        min-width: 0;
      }

      .journal-sidebar {
        flex-shrink: 0;
        position: relative;
      }
    `,
  ],
})
export class JournalPageComponent {
  contentId = '';

  private readonly route = inject(ActivatedRoute);
  private readonly storageApi = inject(StorageApiService);
  private readonly destroyRef = inject(DestroyRef);
  private readonly editor = viewChild(JournalEditorComponent);

  constructor() {
    const destroy$ = new Subject<void>();
    this.destroyRef.onDestroy(() => {
      destroy$.next();
      destroy$.complete();
    });

    this.route.paramMap
      .pipe(
        switchMap((params) => {
          this.contentId = params.get('id') ?? '';
          return this.storageApi.getContent(this.contentId);
        }),
        takeUntil(destroy$),
      )
      .subscribe((content) => {
        if (content) {
          this.editor()?.loadContent(content.title ?? '', content.contentBody ?? '');
        }
      });
  }
}
