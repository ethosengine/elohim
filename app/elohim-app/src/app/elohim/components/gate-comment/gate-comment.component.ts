import {
  Component,
  EventEmitter,
  Input,
  Output,
  ChangeDetectionStrategy,
  inject,
} from '@angular/core';

import { Observable } from 'rxjs';

import { StorageApiService } from '../../services/storage-api.service';
import { GateArtifactCardComponent } from '../gate-artifact-card/gate-artifact-card.component';

import type { MutationContext, ReachTier } from '../../services/gate-interaction.service';

@Component({
  selector: 'app-gate-comment',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [GateArtifactCardComponent],
  template: `
    <div class="gate-comment">
      <app-gate-artifact-card
        [placeholder]="'Add a comment...'"
        [mutationType]="'comment'"
        [contextMetadata]="{ contentId: contentId }"
        [gateApiCall]="apiCall"
        (posted)="onPosted($event)"
        (settled)="onSettled($event)"
      ></app-gate-artifact-card>
    </div>
  `,
  styles: [
    `
      .gate-comment {
        margin: 1rem 0;
      }
    `,
  ],
})
export class GateCommentComponent {
  private readonly storageApi = inject(StorageApiService);

  @Input() contentId = '';

  @Output() commentPosted = new EventEmitter<{ reachTier: ReachTier }>();
  @Output() commentSettled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();

  readonly apiCall = (text: string, context: MutationContext): Observable<unknown> => {
    return this.storageApi.createComment(context.contentId as string, text);
  };

  onPosted(event: { reachTier: ReachTier }): void {
    this.commentPosted.emit(event);
  }

  onSettled(event: { boundary: string; appealPath: string | null }): void {
    this.commentSettled.emit(event);
  }
}
