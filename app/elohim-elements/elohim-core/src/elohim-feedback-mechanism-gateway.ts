import { css, html, LitElement, nothing } from 'lit';
import { property, state } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

export type FeedbackMechanismLevel = 0 | 1 | 2;
export type FeedbackRenderTarget = 'angular' | 'psephos';

export interface MechanismSelection {
  level: FeedbackMechanismLevel;
  renderTarget: FeedbackRenderTarget;
}

export interface AccumulationStatus {
  readyForSensemaking: boolean;
  controversyDetected: boolean;
  settled: boolean;
}

export interface GatewayLoadResult {
  selection: MechanismSelection;
  accumulationStatus: AccumulationStatus | null;
}

/**
 * <elohim-feedback-mechanism-gateway> — governance feedback gateway shell.
 *
 * Orchestrating element that renders the appropriate feedback mechanism sub-slot
 * based on the resolved mechanism level (0 = context-menu-only, 1 = reactions,
 * 2 = graduated + reactions). Psephos ballot rendering (levels 3-7) is out of
 * scope for this primitive — callers provide psephos content via slot.
 *
 * All service dependencies are accepted via properties/callbacks. The element
 * does not fetch governance data itself — callers pass in resolved state.
 *
 * Migrated from FeedbackMechanismGatewayComponent (app/elohim-app/qahal).
 *
 * @element elohim-feedback-mechanism-gateway
 *
 * @prop {string} entityType - Entity type (e.g. 'content', 'proposal')
 * @prop {string} entityId - Entity ID
 * @prop {MechanismSelection | null} selection - Resolved mechanism selection
 * @prop {AccumulationStatus | null} accumulationStatus - Signal accumulation status
 * @prop {boolean} loading - Whether governance data is loading
 * @prop {Function | null} onNavigateToSensemaking - Called when sensemaking link is clicked
 * @prop {Function | null} onChallengeAction - Called when challenge action is triggered (level 0)
 *
 * @fires challenge-action - { detail: { entityType, entityId } } when challenge is triggered
 * @fires sensemaking-navigate - { detail: { entityType, entityId } } when sensemaking link clicked
 *
 * @slot reactions - Slot for the reactions bar (level 1+)
 * @slot graduated-feedback - Slot for graduated feedback (level 2)
 * @slot psephos - Slot for Psephos ballot (render target: psephos)
 * @slot aggregate - Slot for feedback aggregate display
 * @slot context-menu - Slot for context-menu-only (level 0)
 *
 * @cssprop --elohim-gateway-loading-color - Loading text color
 * @cssprop --elohim-gateway-loading-padding - Loading state padding
 * @cssprop --elohim-gateway-badge-radius - Accumulation badge border radius
 * @cssprop --elohim-gateway-sensemaking-bg - Sensemaking badge background
 * @cssprop --elohim-gateway-sensemaking-fg - Sensemaking badge foreground
 * @cssprop --elohim-gateway-controversy-bg - Controversy badge background
 * @cssprop --elohim-gateway-controversy-fg - Controversy badge foreground
 * @cssprop --elohim-gateway-settled-bg - Settled badge background
 * @cssprop --elohim-gateway-settled-fg - Settled badge foreground
 *
 * @csspart loading - The loading state container
 * @csspart content - The content container when loaded
 * @csspart badge - Each accumulation badge
 * @csspart sensemaking-badge - The sensemaking badge
 * @csspart controversy-badge - The controversy badge
 * @csspart settled-badge - The settled badge
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:designed, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimFeedbackMechanismGateway extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      position: relative;
    }

    :host([hidden]) {
      display: none;
    }

    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: var(--elohim-gateway-loading-padding, 0.75rem);
      font-size: 0.8125rem;
      color: var(--elohim-gateway-loading-color, CanvasText);
    }

    .badge {
      display: inline-block;
      margin-block-start: 0.5rem;
      padding-block: 0.25rem;
      padding-inline: 0.625rem;
      font-size: 0.75rem;
      font-weight: 500;
      border-radius: var(--elohim-gateway-badge-radius, 999px);
      line-height: 1.4;
    }

    .badge-sensemaking {
      background: var(--elohim-gateway-sensemaking-bg, Canvas);
      color: var(--elohim-gateway-sensemaking-fg, LinkText);
      cursor: pointer;
      border: none;
      font: inherit;
      font-size: 0.75rem;
      padding-block: 0.25rem;
      padding-inline: 0.625rem;
      border-radius: var(--elohim-gateway-badge-radius, 999px);
    }

    .badge-sensemaking:hover {
      text-decoration: underline;
    }

    .badge-controversy {
      background: var(--elohim-gateway-controversy-bg, Canvas);
      color: var(--elohim-gateway-controversy-fg, CanvasText);
    }

    .badge-settled {
      background: var(--elohim-gateway-settled-bg, Canvas);
      color: var(--elohim-gateway-settled-fg, CanvasText);
    }

    @media (forced-colors: active) {
      .badge,
      .badge-sensemaking {
        border: 1px solid ButtonText;
        background: ButtonFace;
        color: ButtonText;
      }

      .badge-sensemaking:hover {
        background: Highlight;
        color: HighlightText;
      }
    }
  `;

  @property() entityType = '';
  @property() entityId = '';
  @property({ attribute: false }) selection: MechanismSelection | null = null;
  @property({ attribute: false }) accumulationStatus: AccumulationStatus | null = null;
  @property({ type: Boolean }) loading = false;

  @property({ attribute: false })
  onNavigateToSensemaking: (() => void) | null = null;

  @property({ attribute: false })
  onChallengeAction: (() => void) | null = null;

  @state() private showChallengeForm = false;

  override render() {
    if (this.loading || !this.selection) {
      return html`
        <div class="loading" part="loading" data-testid="gateway-loading">
          Loading governance...
        </div>
      `;
    }

    const sel = this.selection;

    return html`
      <div part="content">
        ${sel.renderTarget === 'angular'
          ? html`
              ${sel.level === 0
                ? html`<slot name="context-menu"></slot>`
                : nothing}
              ${sel.level >= 1
                ? html`<slot name="reactions"></slot>`
                : nothing}
              ${sel.level >= 2
                ? html`<slot name="graduated-feedback"></slot>`
                : nothing}
            `
          : nothing}

        ${sel.renderTarget === 'psephos'
          ? html`<slot name="psephos"></slot>`
          : nothing}

        <slot name="aggregate"></slot>

        ${this.accumulationStatus
          ? this.renderAccumulationBadges(this.accumulationStatus)
          : nothing}
      </div>
    `;
  }

  private renderAccumulationBadges(status: AccumulationStatus) {
    return html`
      ${status.readyForSensemaking
        ? html`
            <button
              class="badge-sensemaking"
              part="badge sensemaking-badge"
              type="button"
              data-testid="sensemaking-link"
              @click=${this.handleSensemakingClick}
            >
              Diverse perspectives on this content — sensemaking available
            </button>
          `
        : nothing}
      ${status.controversyDetected
        ? html`
            <div class="badge badge-controversy" part="badge controversy-badge">
              Active discussion
            </div>
          `
        : nothing}
      ${status.settled
        ? html`
            <div class="badge badge-settled" part="badge settled-badge">
              Community consensus
            </div>
          `
        : nothing}
    `;
  }

  private handleSensemakingClick(): void {
    if (this.onNavigateToSensemaking) {
      this.onNavigateToSensemaking();
    } else {
      this.dispatchEvent(
        new CustomEvent('sensemaking-navigate', {
          detail: { entityType: this.entityType, entityId: this.entityId },
          bubbles: true,
          composed: true,
        }),
      );
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-feedback-mechanism-gateway': ElohimFeedbackMechanismGateway;
  }
}
