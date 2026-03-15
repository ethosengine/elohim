import {
  Component,
  EventEmitter,
  Input,
  Output,
  ChangeDetectionStrategy,
} from '@angular/core';

import { GateArtifactCardComponent } from '../gate-artifact-card/gate-artifact-card.component';
import type { ReachTier } from '../../services/gate-interaction.service';

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
  @Input() contentId = '';

  @Output() commentPosted = new EventEmitter<{ reachTier: ReachTier }>();
  @Output() commentSettled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();

  onPosted(event: { reachTier: ReachTier }): void {
    this.commentPosted.emit(event);
  }

  onSettled(event: { boundary: string; appealPath: string | null }): void {
    this.commentSettled.emit(event);
  }
}
