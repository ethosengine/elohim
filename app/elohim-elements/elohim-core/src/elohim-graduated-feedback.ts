import { css, html, LitElement, nothing } from 'lit';
import { property, state } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

export type FeedbackContext = 'accuracy' | 'usefulness' | 'proposal' | 'clarity' | 'relevance';

export interface ScalePosition {
  index: number; // 0-1 normalized
  label: string;
  color: string;
  requiresReasoning?: boolean;
}

export interface ScaleDefinition {
  label: string;
  positions: ScalePosition[];
}

export interface GraduatedFeedbackInput {
  context: FeedbackContext;
  position: string;
  positionIndex: number;
  intensity: number;
  reasoning?: string;
}

export type FeedbackDistribution = Record<string, number>;

export type FeedbackContextScales = Record<FeedbackContext, ScaleDefinition>;

const DEFAULT_SCALES: FeedbackContextScales = {
  accuracy: {
    label: 'How accurate is this content?',
    positions: [
      { index: 0, label: 'False', color: '#e74c3c' },
      { index: 0.25, label: 'Inaccurate', color: '#e67e22' },
      { index: 0.5, label: 'Uncertain', color: '#f1c40f' },
      { index: 0.75, label: 'Mostly Accurate', color: '#2ecc71' },
      { index: 1, label: 'Accurate', color: '#27ae60' },
    ],
  },
  usefulness: {
    label: 'How useful did you find this content?',
    positions: [
      { index: 0, label: 'Not Useful', color: '#95a5a6' },
      { index: 0.25, label: 'Slightly Useful', color: '#bdc3c7' },
      { index: 0.5, label: 'Useful', color: '#3498db' },
      { index: 0.75, label: 'Very Useful', color: '#2980b9' },
      { index: 1, label: 'Transformative', color: '#6c5ce7' },
    ],
  },
  proposal: {
    label: 'What is your position on this proposal?',
    positions: [
      { index: 0, label: 'Block', color: '#e74c3c', requiresReasoning: true },
      { index: 0.25, label: 'Disagree', color: '#e67e22' },
      { index: 0.5, label: 'Abstain', color: '#95a5a6' },
      { index: 0.75, label: 'Agree', color: '#2ecc71' },
      { index: 1, label: 'Strongly Agree', color: '#27ae60' },
    ],
  },
  clarity: {
    label: 'How clear was this content?',
    positions: [
      { index: 0, label: 'Confusing', color: '#e74c3c' },
      { index: 0.25, label: 'Unclear', color: '#e67e22' },
      { index: 0.5, label: 'Adequate', color: '#f1c40f' },
      { index: 0.75, label: 'Clear', color: '#2ecc71' },
      { index: 1, label: 'Crystal Clear', color: '#27ae60' },
    ],
  },
  relevance: {
    label: 'How relevant is this to your learning goals?',
    positions: [
      { index: 0, label: 'Irrelevant', color: '#95a5a6' },
      { index: 0.25, label: 'Tangential', color: '#bdc3c7' },
      { index: 0.5, label: 'Related', color: '#3498db' },
      { index: 0.75, label: 'Highly Relevant', color: '#2980b9' },
      { index: 1, label: 'Essential', color: '#6c5ce7' },
    ],
  },
};

/**
 * <elohim-graduated-feedback> — context-aware scaled feedback primitive.
 *
 * Inspired by Loomio's Gradients of Agreement and Forby's ARCH voting.
 * Positions are context-specific; intensity captures how strongly the
 * respondent feels (ARCH pattern); optional reasoning for deliberative
 * engagement.
 *
 * Service dependencies accepted as callbacks — no Angular DI.
 *
 * Migrated from GraduatedFeedbackComponent (app/elohim-app/qahal).
 *
 * @element elohim-graduated-feedback
 *
 * @prop {string} entityType - Entity type (e.g. 'content', 'proposal')
 * @prop {string} entityId - Entity ID
 * @prop {FeedbackContext} context - Feedback context determining the scale
 * @prop {boolean} requiresReasoning - Whether reasoning is always required
 * @prop {boolean} showAggregates - Show aggregate distribution
 * @prop {boolean} compact - Compact mode
 * @prop {FeedbackDistribution | null} distribution - Aggregate distribution from parent
 * @prop {number} totalResponses - Total response count for distribution bar
 * @prop {boolean} hasSubmitted - Whether the current user has already submitted
 * @prop {Function | null} onSubmit - Called when user submits feedback
 *
 * @fires feedback-submit - { detail: GraduatedFeedbackInput } when feedback is submitted
 * @fires feedback-reset - when form is reset
 *
 * @cssprop --elohim-feedback-padding - Container padding
 * @cssprop --elohim-feedback-label-color - Label text color
 * @cssprop --elohim-feedback-border - Section border
 * @cssprop --elohim-feedback-submit-bg - Submit button background
 * @cssprop --elohim-feedback-submit-fg - Submit button foreground
 * @cssprop --elohim-feedback-submit-radius - Submit button border radius
 * @cssprop --elohim-feedback-distribution-height - Distribution bar height
 *
 * @csspart container - The feedback container
 * @csspart label - The scale label heading
 * @csspart positions - The positions radiogroup
 * @csspart position-btn - Each position button
 * @csspart intensity-section - The intensity slider section
 * @csspart reasoning-section - The reasoning textarea section
 * @csspart submit-section - The submit/reset button section
 * @csspart submit-btn - The submit button
 * @csspart reset-btn - The reset button
 * @csspart aggregate-section - The community aggregate section
 * @csspart distribution-bar - The distribution progress bar
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings pilot | steward
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimGraduatedFeedback extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    :host([hidden]) {
      display: none;
    }

    .container {
      padding: var(--elohim-feedback-padding, 1rem);
    }

    :host([compact]) .container {
      padding: 0.5rem;
    }

    .feedback-label {
      margin: 0 0 0.75rem;
      font-size: 1rem;
      font-weight: 500;
    }

    .positions {
      display: flex;
      gap: 0.5rem;
      flex-wrap: wrap;
    }

    .position-btn {
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 2px solid;
      border-radius: 0.375rem;
      background: transparent;
      cursor: pointer;
      font: inherit;
      font-size: 0.875rem;
      font-weight: 500;
    }

    .position-btn[aria-checked='true'] {
      font-weight: 600;
    }

    .position-btn:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    .intensity-section {
      margin-block-start: 1rem;
    }

    .intensity-label {
      display: block;
      font-size: 0.875rem;
      margin-block-end: 0.25rem;
      color: var(--elohim-feedback-label-color, CanvasText);
    }

    .intensity-slider {
      inline-size: 100%;
      accent-color: Highlight;
    }

    .reasoning-section {
      margin-block-start: 1rem;
    }

    .reasoning-label {
      display: block;
      font-size: 0.875rem;
      margin-block-end: 0.25rem;
      color: var(--elohim-feedback-label-color, CanvasText);
    }

    .reasoning-textarea {
      inline-size: 100%;
      padding-block: 0.5rem;
      padding-inline: 0.5rem;
      border: 1px solid
        var(--elohim-feedback-border, color-mix(in oklch, currentColor 25%, transparent));
      border-radius: 0.25rem;
      font: inherit;
      font-size: 0.875rem;
      resize: vertical;
      background: Canvas;
      color: CanvasText;
      box-sizing: border-box;
    }

    .toggle-reasoning-btn {
      margin-block-start: 0.5rem;
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      border: none;
      background: transparent;
      color: LinkText;
      cursor: pointer;
      font: inherit;
      font-size: 0.813rem;
    }

    .submit-section {
      margin-block-start: 1rem;
      display: flex;
      gap: 0.5rem;
    }

    .submit-btn {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: none;
      border-radius: var(--elohim-feedback-submit-radius, 0.25rem);
      background: var(--elohim-feedback-submit-bg, ButtonFace);
      color: var(--elohim-feedback-submit-fg, ButtonText);
      cursor: pointer;
      font: inherit;
      font-size: 0.875rem;
    }

    .submit-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .reset-btn {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid
        var(--elohim-feedback-border, color-mix(in oklch, currentColor 25%, transparent));
      border-radius: 0.25rem;
      background: transparent;
      cursor: pointer;
      font: inherit;
      font-size: 0.875rem;
    }

    .aggregate-section {
      margin-block-start: 1rem;
      padding-block-start: 0.75rem;
      border-block-start: 1px solid
        var(--elohim-feedback-border, color-mix(in oklch, currentColor 15%, transparent));
    }

    .aggregate-header {
      font-size: 0.813rem;
      color: var(--elohim-feedback-label-color, CanvasText);
      margin: 0 0 0.5rem;
    }

    .distribution-bar {
      display: flex;
      block-size: var(--elohim-feedback-distribution-height, 0.5rem);
      border-radius: 0.25rem;
      overflow: hidden;
    }

    .distribution-segment {
      flex-shrink: 0;
    }

    @media (pointer: coarse) {
      .position-btn,
      .submit-btn,
      .reset-btn {
        min-block-size: 44px;
      }
    }

    @media (forced-colors: active) {
      .position-btn {
        border-color: ButtonText;
        background: ButtonFace;
        color: ButtonText;
      }

      .position-btn[aria-checked='true'] {
        background: Highlight;
        color: HighlightText;
        border-color: Highlight;
      }

      .submit-btn {
        background: ButtonFace;
        color: ButtonText;
        border: 1px solid ButtonText;
      }

      .reasoning-textarea {
        border-color: ButtonText;
        background: Canvas;
        color: CanvasText;
      }
    }
  `;

  @property() entityType = 'content';
  @property() entityId = '';
  @property() context: FeedbackContext = 'usefulness';
  @property({ type: Boolean }) requiresReasoning = false;
  @property({ type: Boolean }) showAggregates = true;
  @property({ type: Boolean, reflect: true }) compact = false;
  @property({ attribute: false }) distribution: FeedbackDistribution | null = null;
  @property({ type: Number }) totalResponses = 0;
  @property({ type: Boolean }) hasSubmitted = false;
  @property({ attribute: false }) onSubmit: ((feedback: GraduatedFeedbackInput) => void) | null =
    null;

  @state() private selectedPosition: number | null = null;
  @state() private intensity = 5;
  @state() private reasoning = '';
  @state() private isSubmitting = false;
  @state() private showReasoningField = false;

  private get currentScale(): ScaleDefinition {
    return DEFAULT_SCALES[this.context];
  }

  private get selectedPositionData(): ScalePosition | null {
    if (this.selectedPosition === null) return null;
    return this.currentScale.positions.find(p => p.index === this.selectedPosition) ?? null;
  }

  private get reasoningIsRequired(): boolean {
    if (this.requiresReasoning) return true;
    if (this.selectedPositionData?.requiresReasoning) return true;
    if (this.selectedPosition !== null && this.selectedPosition <= 0.25) return true;
    return false;
  }

  private get canSubmit(): boolean {
    if (this.selectedPosition === null) return false;
    if (this.reasoningIsRequired && !this.reasoning.trim()) return false;
    return true;
  }

  /**
   * Luminance-picked readable fg for a SELECTED position's fixed scale hex.
   * `CanvasText` was theme-blind here: dark-scheme white on the lighter scale
   * hexes (e.g. usefulness blue #3498db) computed 3.15:1. The scale colors
   * are element-owned semantics (not theme tokens), so the readable pair is
   * derived, not bound. White passes 4.5:1 below L≈0.1833; black above.
   */
  private static readableOn(hex: string): string {
    const n = Number.parseInt(hex.slice(1), 16);
    const lin = (c: number): number => {
      const s = c / 255;
      return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    };
    const l = 0.2126 * lin((n >> 16) & 255) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255);
    return l > 0.1833 ? '#000000' : '#ffffff';
  }

  override render() {
    const scale = this.currentScale;

    return html`
      <div class="container" part="container">
        <h4 class="feedback-label" part="label">${scale.label}</h4>

        <div class="positions" part="positions" role="radiogroup" aria-label="${scale.label}">
          ${scale.positions.map(position => {
            const selected = this.selectedPosition === position.index;
            return html`
              <button
                class="position-btn"
                part="position-btn"
                type="button"
                role="radio"
                aria-checked=${selected ? 'true' : 'false'}
                aria-label="${position.label}"
                data-testid="position-${position.label.toLowerCase().replace(/\s+/g, '-')}"
                style="
                  border-color: ${position.color};
                  background-color: ${selected ? position.color : 'transparent'};
                  color: ${selected
                  ? ElohimGraduatedFeedback.readableOn(position.color)
                  : 'CanvasText'};
                "
                @click=${() => this.selectPosition(position)}
              >
                ${position.label}
              </button>
            `;
          })}
        </div>

        ${this.renderIntensitySection()} ${this.renderReasoningSection()}
        ${this.renderSubmitSection()}
        ${this.showAggregates && this.totalResponses > 0 && this.distribution
          ? this.renderAggregates()
          : nothing}
      </div>
    `;
  }

  private renderIntensitySection(): unknown {
    if (this.selectedPosition === null) return nothing;
    return html`
      <div class="intensity-section" part="intensity-section">
        <label class="intensity-label" for="intensity-slider-${this.entityId}">
          Intensity: ${this.getIntensityLabel()} (${this.intensity}/10)
        </label>
        <input
          id="intensity-slider-${this.entityId}"
          class="intensity-slider"
          type="range"
          min="1"
          max="10"
          .value=${String(this.intensity)}
          data-testid="intensity-slider"
          @input=${(e: Event) => {
            this.intensity = Number((e.target as HTMLInputElement).value);
          }}
        />
      </div>
    `;
  }

  private renderReasoningSection(): unknown {
    if (this.showReasoningField || this.reasoningIsRequired) {
      const reasoningLabel = this.reasoningIsRequired
        ? 'Reasoning (required)'
        : 'Reasoning (optional)';
      return html`
        <div class="reasoning-section" part="reasoning-section">
          <label class="reasoning-label" for="reasoning-text-${this.entityId}">
            ${reasoningLabel}
          </label>
          <textarea
            id="reasoning-text-${this.entityId}"
            class="reasoning-textarea"
            placeholder="Share your thinking..."
            rows="3"
            data-testid="reasoning-textarea"
            .value=${this.reasoning}
            @input=${(e: Event) => {
              this.reasoning = (e.target as HTMLTextAreaElement).value;
            }}
          ></textarea>
        </div>
      `;
    }
    if (this.selectedPosition === null) return nothing;
    return html`
      <button
        class="toggle-reasoning-btn"
        type="button"
        data-testid="toggle-reasoning-btn"
        @click=${this.toggleReasoning}
      >
        Add reasoning
      </button>
    `;
  }

  private renderSubmitSection(): unknown {
    if (this.selectedPosition === null) return nothing;
    let submitLabel = 'Submit';
    if (this.isSubmitting) {
      submitLabel = 'Submitting...';
    } else if (this.hasSubmitted) {
      submitLabel = 'Update Feedback';
    }
    return html`
      <div class="submit-section" part="submit-section">
        <button
          class="submit-btn"
          part="submit-btn"
          type="button"
          ?disabled=${!this.canSubmit || this.isSubmitting}
          data-testid="submit-feedback-btn"
          @click=${this.submit}
        >
          ${submitLabel}
        </button>
        ${this.hasSubmitted
          ? html`
              <button
                class="reset-btn"
                part="reset-btn"
                type="button"
                data-testid="reset-feedback-btn"
                @click=${this.reset}
              >
                Reset
              </button>
            `
          : nothing}
      </div>
    `;
  }

  private renderAggregates() {
    const scale = this.currentScale;
    const total = this.totalResponses;
    const dist = this.distribution!;

    return html`
      <div class="aggregate-section" part="aggregate-section">
        <p class="aggregate-header">
          Community response (${total} ${total === 1 ? 'response' : 'responses'})
        </p>
        <div class="distribution-bar" part="distribution-bar">
          ${scale.positions.map((position): unknown => {
            const count = dist[position.label] ?? 0;
            const pct = total > 0 ? (count / total) * 100 : 0;
            if (pct === 0) return nothing;
            return html`
              <div
                class="distribution-segment"
                style="width: ${pct}%; background-color: ${position.color};"
                title="${position.label}: ${Math.round(pct)}%"
              ></div>
            `;
          })}
        </div>
      </div>
    `;
  }

  private selectPosition(position: ScalePosition): void {
    this.selectedPosition = position.index;
    if (position.requiresReasoning || this.requiresReasoning || position.index <= 0.25) {
      this.showReasoningField = true;
    }
  }

  private toggleReasoning(): void {
    this.showReasoningField = !this.showReasoningField;
  }

  private submit(): void {
    if (!this.canSubmit || this.isSubmitting) return;

    const positionData = this.selectedPositionData!;
    const feedback: GraduatedFeedbackInput = {
      context: this.context,
      position: positionData.label,
      positionIndex: positionData.index,
      intensity: this.intensity,
      reasoning: this.reasoning.trim() || undefined,
    };

    this.isSubmitting = true;

    if (this.onSubmit) {
      this.onSubmit(feedback);
      this.isSubmitting = false;
      this.hasSubmitted = true;
    }

    this.dispatchEvent(
      new CustomEvent('feedback-submit', {
        detail: feedback,
        bubbles: true,
        composed: true,
      })
    );

    this.isSubmitting = false;
    this.hasSubmitted = true;
  }

  private reset(): void {
    this.selectedPosition = null;
    this.intensity = 5;
    this.reasoning = '';
    this.showReasoningField = false;
    this.hasSubmitted = false;

    this.dispatchEvent(
      new CustomEvent('feedback-reset', {
        bubbles: true,
        composed: true,
      })
    );
  }

  private getIntensityLabel(): string {
    if (this.intensity <= 2) return 'Slightly';
    if (this.intensity <= 4) return 'Moderately';
    if (this.intensity <= 6) return 'Fairly';
    if (this.intensity <= 8) return 'Strongly';
    return 'Very Strongly';
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-graduated-feedback': ElohimGraduatedFeedback;
  }
}
