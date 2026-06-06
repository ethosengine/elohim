import { css, html, LitElement, nothing } from 'lit';
import { property, state } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

export type GateFeedbackType = 'flag' | 'challenge' | 'feedback' | 'report';

export interface GateFeedbackMenuItem {
  type: GateFeedbackType;
  label: string;
}

export interface GateFeedbackPostedEvent {
  feedbackType: GateFeedbackType;
  contentId: string;
}

const DEFAULT_MENU_ITEMS: GateFeedbackMenuItem[] = [
  { type: 'flag', label: 'Flag' },
  { type: 'challenge', label: 'Challenge' },
  { type: 'feedback', label: 'Feedback' },
  { type: 'report', label: 'Report Issue' },
];

const MODAL_TITLES: Record<GateFeedbackType, string> = {
  flag: 'Flag Content',
  challenge: 'Challenge Content',
  feedback: 'Share Feedback',
  report: 'Report Issue',
};

const MODAL_PLACEHOLDERS: Record<GateFeedbackType, string> = {
  flag: 'Describe the issue...',
  challenge: 'State your case...',
  feedback: 'Share your thoughts...',
  report: 'What happened?',
};

/**
 * <elohim-gate-feedback-trigger> — governance gate feedback trigger primitive.
 *
 * Renders a vertical-ellipsis trigger button that opens a dropdown menu of
 * feedback action types (Flag, Challenge, Feedback, Report). Selecting an
 * item opens a modal dialog with a text area for the feedback message.
 *
 * Service dependencies are accepted as callbacks — no Angular DI. The element
 * dispatches events; callers handle the API submissions.
 *
 * Migrated from GateFeedbackTriggerComponent + GateFeedbackModalComponent
 * (app/elohim-app/elohim/components/gate-feedback/).
 *
 * @element elohim-gate-feedback-trigger
 *
 * @prop {string} contentId - Content ID to attach the feedback to
 * @prop {GateFeedbackMenuItem[]} menuItems - Menu items to show
 * @prop {Function | null} onFeedbackSubmit - Called with the feedback text and type
 *
 * @fires feedback-menu-open - when the menu opens
 * @fires feedback-type-selected - { detail: { type: GateFeedbackType } }
 * @fires feedback-submit - { detail: { type, contentId, text } }
 * @fires feedback-closed - when modal is closed
 *
 * @cssprop --elohim-gate-trigger-btn-color - Trigger button icon color
 * @cssprop --elohim-gate-trigger-btn-hover-color - Trigger button hover color
 * @cssprop --elohim-gate-menu-bg - Dropdown menu background
 * @cssprop --elohim-gate-menu-border - Dropdown menu border
 * @cssprop --elohim-gate-menu-radius - Dropdown menu border radius
 * @cssprop --elohim-gate-menu-shadow - Dropdown menu box shadow
 * @cssprop --elohim-gate-menu-item-hover-bg - Menu item hover background
 * @cssprop --elohim-gate-modal-bg - Modal panel background
 * @cssprop --elohim-gate-modal-radius - Modal panel border radius
 * @cssprop --elohim-gate-modal-shadow - Modal panel box shadow
 * @cssprop --elohim-gate-submit-bg - Submit button background
 * @cssprop --elohim-gate-submit-fg - Submit button foreground
 * @cssprop --elohim-gate-submit-radius - Submit button radius
 *
 * @csspart trigger-btn - The vertical-ellipsis trigger button
 * @csspart menu - The dropdown menu container
 * @csspart menu-item - Each menu item button
 * @csspart modal-overlay - The modal backdrop overlay
 * @csspart modal-panel - The modal dialog panel
 * @csspart modal-title - The modal heading
 * @csspart modal-textarea - The feedback textarea
 * @csspart submit-btn - The modal submit button
 * @csspart close-btn - The modal close button
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings pilot | steward
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimGateFeedbackTrigger extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-block;
      position: relative;
    }

    :host([hidden]) {
      display: none;
    }

    .trigger-btn {
      background: none;
      border: none;
      font-size: 1.25rem;
      cursor: pointer;
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      line-height: 1;
      color: var(--elohim-gate-trigger-btn-color, GrayText);
      min-block-size: 44px;
      min-inline-size: 44px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }

    .trigger-btn:hover,
    .trigger-btn:focus-visible {
      color: var(--elohim-gate-trigger-btn-hover-color, CanvasText);
      outline: 2px solid Highlight;
      outline-offset: 1px;
      border-radius: 0.25rem;
    }

    .menu {
      position: absolute;
      inset-block-start: 100%;
      inset-inline-end: 0;
      background: var(--elohim-gate-menu-bg, Canvas);
      border: var(
        --elohim-gate-menu-border,
        1px solid color-mix(in oklch, currentColor 20%, transparent)
      );
      border-radius: var(--elohim-gate-menu-radius, 0.5rem);
      box-shadow: var(--elohim-gate-menu-shadow, 0 2px 8px rgb(0 0 0 / 15%));
      z-index: 100;
      min-inline-size: 140px;
    }

    .menu-item {
      display: block;
      inline-size: 100%;
      background: none;
      border: none;
      padding-block: 0.5rem;
      padding-inline: 1rem;
      text-align: start;
      cursor: pointer;
      font: inherit;
      font-size: 0.875rem;
      color: CanvasText;
      min-block-size: 44px;
    }

    .menu-item:hover,
    .menu-item:focus-visible {
      background: var(
        --elohim-gate-menu-item-hover-bg,
        color-mix(in oklch, Canvas 90%, CanvasText)
      );
      outline: none;
    }

    .modal-overlay {
      position: fixed;
      inset: 0;
      background: rgb(0 0 0 / 50%);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1000;
    }

    .modal-panel {
      background: var(--elohim-gate-modal-bg, Canvas);
      border-radius: var(--elohim-gate-modal-radius, 0.75rem);
      padding: 1.5rem;
      inline-size: min(600px, calc(100vw - 2rem));
      max-block-size: 90vh;
      overflow-y: auto;
      box-shadow: var(--elohim-gate-modal-shadow, 0 8px 32px rgb(0 0 0 / 20%));
      color: CanvasText;
    }

    .modal-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-block-end: 1rem;
    }

    .modal-title {
      margin: 0;
      font-size: 1.25rem;
    }

    .close-btn {
      background: none;
      border: none;
      font-size: 1.5rem;
      cursor: pointer;
      color: GrayText;
      line-height: 1;
      padding-block: 0.25rem;
      padding-inline: 0.25rem;
      min-block-size: 44px;
      min-inline-size: 44px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }

    .close-btn:hover,
    .close-btn:focus-visible {
      color: CanvasText;
      outline: 2px solid Highlight;
      outline-offset: 1px;
      border-radius: 0.25rem;
    }

    .modal-textarea {
      inline-size: 100%;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 1px solid color-mix(in oklch, currentcolor 25%, transparent);
      border-radius: 0.25rem;
      font: inherit;
      font-size: 0.875rem;
      resize: vertical;
      background: Canvas;
      color: CanvasText;
      box-sizing: border-box;
      min-block-size: 120px;
      margin-block-end: 1rem;
    }

    .submit-btn {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: none;
      border-radius: var(--elohim-gate-submit-radius, 0.25rem);
      background: var(--elohim-gate-submit-bg, ButtonFace);
      color: var(--elohim-gate-submit-fg, ButtonText);
      cursor: pointer;
      font: inherit;
      min-block-size: 44px;
    }

    .submit-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .submit-btn:focus-visible,
    .close-btn:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    @media (forced-colors: active) {
      .trigger-btn {
        color: ButtonText;
      }

      .menu {
        border-color: ButtonText;
        background: Canvas;
      }

      .menu-item {
        color: CanvasText;
      }

      .menu-item:hover {
        background: Highlight;
        color: HighlightText;
      }

      .modal-panel {
        border: 1px solid CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      .modal-textarea {
        border-color: ButtonText;
        background: Canvas;
        color: CanvasText;
      }

      .submit-btn {
        background: ButtonFace;
        color: ButtonText;
        border: 1px solid ButtonText;
      }
    }
  `;

  @property({ attribute: 'content-id' }) contentId = '';

  @property({ attribute: false })
  menuItems: GateFeedbackMenuItem[] = DEFAULT_MENU_ITEMS;

  @property({ attribute: false })
  onFeedbackSubmit: ((type: GateFeedbackType, contentId: string, text: string) => void) | null =
    null;

  @state() private menuOpen = false;
  @state() private modalOpen = false;
  @state() private activeFeedbackType: GateFeedbackType = 'feedback';
  @state() private feedbackText = '';
  @state() private isSubmitting = false;

  override render() {
    return html`
      <button
        class="trigger-btn"
        part="trigger-btn"
        type="button"
        aria-label="Governance feedback"
        aria-expanded=${this.menuOpen ? 'true' : 'false'}
        aria-haspopup="menu"
        data-testid="feedback-trigger-btn"
        @click=${this.toggleMenu}
      >
        &#x22EE;
      </button>

      ${this.menuOpen
        ? html`
            <div
              class="menu"
              part="menu"
              role="menu"
              data-testid="feedback-trigger-menu"
              @keydown=${this.handleMenuKeydown}
            >
              ${this.menuItems.map(
                item => html`
                  <button
                    class="menu-item"
                    part="menu-item"
                    type="button"
                    role="menuitem"
                    data-testid="feedback-menu-item-${item.type}"
                    @click=${() => this.openModal(item.type)}
                  >
                    ${item.label}
                  </button>
                `
              )}
            </div>
          `
        : nothing}
      ${this.modalOpen ? this.renderModal() : nothing}
    `;
  }

  private renderModal() {
    const title = MODAL_TITLES[this.activeFeedbackType];
    const placeholder = MODAL_PLACEHOLDERS[this.activeFeedbackType];

    return html`
      <div
        class="modal-overlay"
        part="modal-overlay"
        data-testid="feedback-modal-backdrop"
        role="presentation"
        @click=${this.handleOverlayClick}
        @keydown=${(e: KeyboardEvent) => {
          if (e.key === 'Escape') this.closeModal();
        }}
      >
        <div
          class="modal-panel"
          part="modal-panel"
          role="dialog"
          aria-modal="true"
          aria-labelledby="gate-modal-title"
          data-testid="feedback-modal-panel"
          @click=${(e: Event) => e.stopPropagation()}
          @keydown=${(e: KeyboardEvent) => {
            // Keep clicks/keys inside the panel from reaching the overlay, but let
            // Escape bubble so the overlay's close handler still fires.
            if (e.key !== 'Escape') e.stopPropagation();
          }}
        >
          <div class="modal-header">
            <h3
              id="gate-modal-title"
              class="modal-title"
              part="modal-title"
              data-testid="feedback-modal-title"
            >
              ${title}
            </h3>
            <button
              class="close-btn"
              part="close-btn"
              type="button"
              aria-label="Close modal"
              data-testid="feedback-modal-close"
              @click=${this.closeModal}
            >
              &times;
            </button>
          </div>
          <textarea
            class="modal-textarea"
            part="modal-textarea"
            placeholder="${placeholder}"
            data-testid="feedback-modal-textarea"
            .value=${this.feedbackText}
            @input=${(e: Event) => {
              this.feedbackText = (e.target as HTMLTextAreaElement).value;
            }}
          ></textarea>
          <button
            class="submit-btn"
            part="submit-btn"
            type="button"
            ?disabled=${!this.feedbackText.trim() || this.isSubmitting}
            data-testid="feedback-modal-submit"
            @click=${this.submitFeedback}
          >
            ${this.isSubmitting ? 'Submitting...' : 'Submit'}
          </button>
        </div>
      </div>
    `;
  }

  private toggleMenu(): void {
    this.menuOpen = !this.menuOpen;
    if (this.menuOpen) {
      this.dispatchEvent(new CustomEvent('feedback-menu-open', { bubbles: true, composed: true }));
    }
  }

  private openModal(type: GateFeedbackType): void {
    this.activeFeedbackType = type;
    this.menuOpen = false;
    this.feedbackText = '';
    this.modalOpen = true;

    this.dispatchEvent(
      new CustomEvent('feedback-type-selected', {
        detail: { type },
        bubbles: true,
        composed: true,
      })
    );
  }

  private closeModal(): void {
    this.modalOpen = false;
    this.feedbackText = '';
    this.dispatchEvent(new CustomEvent('feedback-closed', { bubbles: true, composed: true }));
  }

  private handleOverlayClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) {
      this.closeModal();
    }
  }

  private handleMenuKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      this.menuOpen = false;
    }
  }

  private submitFeedback(): void {
    const text = this.feedbackText.trim();
    if (!text || this.isSubmitting) return;

    this.isSubmitting = true;

    if (this.onFeedbackSubmit) {
      this.onFeedbackSubmit(this.activeFeedbackType, this.contentId, text);
    }

    this.dispatchEvent(
      new CustomEvent('feedback-submit', {
        detail: { type: this.activeFeedbackType, contentId: this.contentId, text },
        bubbles: true,
        composed: true,
      })
    );

    this.isSubmitting = false;
    this.closeModal();
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-gate-feedback-trigger': ElohimGateFeedbackTrigger;
  }
}
