import {
  Component,
  ChangeDetectionStrategy,
  input,
  ElementRef,
  viewChild,
  effect,
} from '@angular/core';

export interface ChatMessage {
  role: 'human' | 'elohim';
  text: string;
}

@Component({
  selector: 'app-elohim-message-list',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="message-list" #scrollContainer data-testid="message-list">
      @for (msg of messages(); track $index) {
        <div
          class="chat-bubble"
          [class.human]="msg.role === 'human'"
          [class.elohim]="msg.role === 'elohim'"
          [attr.data-testid]="'chat-message-' + $index"
        >
          {{ msg.text }}
        </div>
      }
    </div>
  `,
  styles: [
    `
      .message-list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        overflow-y: auto;
        flex: 1;
        padding: 1rem;
      }

      .chat-bubble {
        max-width: 85%;
        padding: 0.625rem 0.875rem;
        border-radius: 12px;
        font-size: 0.875rem;
        line-height: 1.4;
        word-wrap: break-word;
      }

      .chat-bubble.human {
        align-self: flex-end;
        background: var(--primary-light, #e8f0fe);
        color: var(--text-primary, #202124);
      }

      .chat-bubble.elohim {
        align-self: flex-start;
        background: var(--surface-variant, #f1f3f4);
        color: var(--text-primary, #202124);
      }
    `,
  ],
})
export class ElohimMessageListComponent {
  readonly messages = input<ChatMessage[]>([]);
  private readonly scrollContainer = viewChild<ElementRef>('scrollContainer');

  constructor() {
    effect(() => {
      const msgs = this.messages();
      if (msgs.length > 0) {
        const container = this.scrollContainer()?.nativeElement;
        if (container) {
          requestAnimationFrame(() => {
            container.scrollTop = container.scrollHeight;
          });
        }
      }
    });
  }
}
