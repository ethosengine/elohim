import {
  Component,
  ChangeDetectionStrategy,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { of } from 'rxjs';

import { GateArtifactCardComponent } from '@app/elohim/components/gate-artifact-card/gate-artifact-card.component';

import { CannedResponseService } from '../../services/canned-response.service';
import {
  ElohimMessageListComponent,
  type ChatMessage,
} from './elohim-message-list.component';

@Component({
  selector: 'app-elohim-sidebar',
  standalone: true,
  imports: [ElohimMessageListComponent, GateArtifactCardComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (!expanded()) {
      <button
        class="sidebar-toggle"
        (click)="expanded.set(true)"
        data-testid="sidebar-toggle"
        aria-label="Open elohim chat"
      >
        &#10022;
      </button>
    } @else {
      <div class="sidebar-panel" data-testid="sidebar-panel">
        <header class="sidebar-header">
          <span>Elohim</span>
          <button
            class="sidebar-close"
            (click)="expanded.set(false)"
            data-testid="sidebar-close"
            aria-label="Close elohim chat"
          >
            &times;
          </button>
        </header>
        <app-elohim-message-list [messages]="messages()" />
        <div class="sidebar-input">
          <app-gate-artifact-card
            #sidebarCard
            placeholder="Talk to your elohim..."
            mutationType="journal-chat"
            [contextMetadata]="{}"
            [gateApiCall]="passthroughGate"
            (posted)="onCardPosted()"
          />
        </div>
      </div>
    }
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .sidebar-toggle {
        position: fixed;
        right: 0;
        top: 50%;
        transform: translateY(-50%);
        width: 2.5rem;
        height: 3rem;
        border: 1px solid var(--border-color, #e9ecef);
        border-right: none;
        border-radius: 8px 0 0 8px;
        background: var(--surface-elevated, #fff);
        font-size: 1.25rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--primary, #4285f4);
        box-shadow: -2px 0 8px rgba(0, 0, 0, 0.08);
      }

      .sidebar-toggle:hover {
        background: var(--primary-light, #e8f0fe);
      }

      .sidebar-panel {
        position: fixed;
        right: 0;
        top: 0;
        bottom: 0;
        width: 340px;
        max-width: 90vw;
        background: var(--surface-elevated, #fff);
        border-left: 1px solid var(--border-color, #e9ecef);
        box-shadow: -4px 0 16px rgba(0, 0, 0, 0.08);
        display: flex;
        flex-direction: column;
        z-index: 100;
      }

      .sidebar-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.75rem 1rem;
        border-bottom: 1px solid var(--border-color, #e9ecef);
        font-weight: 600;
        font-size: 0.9375rem;
      }

      .sidebar-close {
        background: none;
        border: none;
        font-size: 1.25rem;
        cursor: pointer;
        color: var(--text-secondary, #5f6368);
        padding: 0.25rem;
        line-height: 1;
      }

      .sidebar-close:hover {
        color: var(--text-primary, #202124);
      }

      .sidebar-input {
        border-top: 1px solid var(--border-color, #e9ecef);
        padding: 0.5rem;
      }
    `,
  ],
})
export class ElohimSidebarComponent {
  private readonly cannedResponse = inject(CannedResponseService);
  private readonly card =
    viewChild<GateArtifactCardComponent>('sidebarCard');

  readonly expanded = signal(false);
  readonly messages = signal<ChatMessage[]>([]);
  readonly passthroughGate = () => of({});

  onCardPosted(): void {
    const cardRef = this.card();
    if (!cardRef) return;

    const text = cardRef.interaction.draftText();
    const reply = this.cannedResponse.respond(text);

    this.messages.update((msgs) => [
      ...msgs,
      { role: 'human' as const, text },
      { role: 'elohim' as const, text: reply },
    ]);

    cardRef.interaction.reset();
  }
}
