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

import { JournalRoutingService } from '../../services/journal-routing.service';
import { JournalEditorComponent } from './journal-editor.component';
import { JournalConfirmComponent } from './journal-confirm.component';
import { JournalRoutingCardsComponent } from './journal-routing-cards.component';
import { JournalRoutedComponent } from './journal-routed.component';
import { ElohimSidebarComponent } from './elohim-sidebar.component';

@Component({
  selector: 'app-journal-page',
  standalone: true,
  imports: [
    JournalEditorComponent,
    ElohimSidebarComponent,
    JournalConfirmComponent,
    JournalRoutingCardsComponent,
    JournalRoutedComponent,
  ],
  providers: [JournalRoutingService],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="journal-layout" data-testid="journal-layout">
      <div class="journal-main">
        @switch (routing.state()) {
          @case ('writing') {
            <app-journal-editor
              [contentId]="contentId"
              (finished)="onFinish($event)"
            />
          }
          @case ('confirming') {
            <app-journal-confirm
              [text]="routing.journalText()"
              [intentSummary]="routing.intentSummary()"
              [analyzing]="!routing.intentSummary()"
              (confirmed)="onConfirm()"
              (editRequested)="onEdit()"
            />
          }
          @case ('routing') {
            <app-journal-routing-cards
              [suggestions]="routing.suggestions()"
              [journalText]="routing.journalText()"
              (postCard)="onPostCard($event)"
              (dismissCard)="onDismissCard($event)"
              (editRequested)="onEdit()"
            />
          }
          @case ('routed') {
            <app-journal-routed
              [suggestions]="routing.suggestions()"
              (writeAnother)="onWriteAnother()"
            />
          }
        }
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

  readonly routing = inject(JournalRoutingService);

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

  onFinish(event: { title: string; body: string }): void {
    this.routing.setContentId(this.contentId);
    this.routing.finish(event.body);
  }

  onConfirm(): void {
    this.routing.confirm();
  }

  onEdit(): void {
    this.routing.edit();
  }

  onPostCard(id: string): void {
    this.routing.postCard(id);
  }

  onDismissCard(id: string): void {
    this.routing.dismissCard(id);
  }

  onWriteAnother(): void {
    // Navigate to new journal — for now just reset
    this.routing.edit();
  }
}
