import {
  Component,
  ChangeDetectionStrategy,
  EventEmitter,
  Output,
  input,
  signal,
} from '@angular/core';

import { GateFeedbackModalComponent, type FeedbackType } from './gate-feedback-modal.component';

import type { ReachTier } from '../../services/gate-interaction.service';

interface MenuItem {
  type: FeedbackType;
  label: string;
}

const MENU_ITEMS: MenuItem[] = [
  { type: 'flag', label: 'Flag' },
  { type: 'challenge', label: 'Challenge' },
  { type: 'feedback', label: 'Feedback' },
  { type: 'report', label: 'Report Issue' },
];

@Component({
  selector: 'app-gate-feedback-trigger',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [GateFeedbackModalComponent],
  template: `
    <div class="trigger-wrapper">
      <button
        class="trigger-btn"
        data-testid="feedback-trigger-btn"
        aria-label="Governance feedback"
        (click)="toggleMenu()"
      >
        &#x22EE;
      </button>

      @if (menuOpen()) {
        <div class="trigger-menu" data-testid="feedback-trigger-menu">
          @for (item of menuItems; track item.type) {
            <button
              class="menu-item"
              [attr.data-testid]="'feedback-menu-item-' + item.type"
              (click)="openModal(item.type)"
            >
              {{ item.label }}
            </button>
          }
        </div>
      }
    </div>

    @if (modalOpen()) {
      <app-gate-feedback-modal
        [feedbackType]="activeFeedbackType()"
        [contentId]="contentId()"
        (posted)="onModalPosted($event)"
        (settled)="onModalSettled($event)"
        (closed)="closeModal()"
      />
    }
  `,
  styles: [
    `
      .trigger-wrapper {
        position: relative;
        display: inline-block;
      }

      .trigger-btn {
        background: none;
        border: none;
        font-size: 1.25rem;
        cursor: pointer;
        padding: 0.25rem 0.5rem;
        line-height: 1;
        color: var(--text-secondary, #5f6368);
      }

      .trigger-btn:hover {
        color: var(--text-primary, #202124);
      }

      .trigger-menu {
        position: absolute;
        top: 100%;
        right: 0;
        background: var(--surface-elevated, #fff);
        border: 1px solid var(--border, #dadce0);
        border-radius: var(--radius-md, 8px);
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
        z-index: 100;
        min-width: 140px;
      }

      .menu-item {
        display: block;
        width: 100%;
        background: none;
        border: none;
        padding: 0.5rem 1rem;
        text-align: left;
        cursor: pointer;
        font-size: 0.875rem;
      }

      .menu-item:hover {
        background: var(--surface-hover, #f1f3f4);
      }
    `,
  ],
})
export class GateFeedbackTriggerComponent {
  readonly contentId = input('');

  @Output() readonly feedbackPosted = new EventEmitter<{
    feedbackType: string;
    reachTier: ReachTier;
  }>();
  @Output() readonly feedbackSettled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();

  readonly menuOpen = signal(false);
  readonly modalOpen = signal(false);
  readonly activeFeedbackType = signal<FeedbackType>('feedback');
  readonly menuItems = MENU_ITEMS;

  toggleMenu(): void {
    this.menuOpen.update(v => !v);
  }

  openModal(type: FeedbackType): void {
    this.activeFeedbackType.set(type);
    this.menuOpen.set(false);
    this.modalOpen.set(true);
  }

  closeModal(): void {
    this.modalOpen.set(false);
  }

  onModalPosted(event: { reachTier: ReachTier }): void {
    this.feedbackPosted.emit({
      feedbackType: this.activeFeedbackType(),
      reachTier: event.reachTier,
    });
  }

  onModalSettled(event: { boundary: string; appealPath: string | null }): void {
    this.feedbackSettled.emit(event);
  }
}
