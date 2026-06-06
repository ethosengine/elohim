import { css, html, LitElement, nothing } from 'lit';
import { property, state } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

export type EmotionalReactionType =
  | 'moved'
  | 'grateful'
  | 'inspired'
  | 'hopeful'
  | 'grieving'
  | 'challenged'
  | 'concerned'
  | 'uncomfortable';

export interface ReactionConstraints {
  permittedTypes: EmotionalReactionType[];
  mediatedTypes?: MediatedReaction[];
}

export interface MediatedReaction {
  type: EmotionalReactionType;
  constitutionalReasoning: string;
  suggestedAlternatives?: EmotionalReactionType[];
  proceedBehavior: { visibleToOthers: boolean };
}

export interface ReactionCount {
  type: EmotionalReactionType;
  count: number;
}

export interface ReactionSubmitEvent {
  type: EmotionalReactionType;
  entityType: string;
  entityId: string;
  context?: string;
}

export interface MediationProceedEvent {
  type: EmotionalReactionType;
  proceededAnyway: boolean;
  alternativeChosen?: EmotionalReactionType;
}

const REACTION_ICONS: Record<EmotionalReactionType, string> = {
  moved: '💫',
  grateful: '🙏',
  inspired: '✨',
  hopeful: '🌱',
  grieving: '🕊️',
  challenged: '🤔',
  concerned: '⚠️',
  uncomfortable: '😟',
};

const REACTION_LABELS: Record<EmotionalReactionType, string> = {
  moved: 'Moved',
  grateful: 'Grateful',
  inspired: 'Inspired',
  hopeful: 'Hopeful',
  grieving: 'Grieving',
  challenged: 'Challenged',
  concerned: 'Concerned',
  uncomfortable: 'Uncomfortable',
};

const DEFAULT_PERMITTED_TYPES: EmotionalReactionType[] = [
  'moved',
  'grateful',
  'inspired',
  'hopeful',
  'grieving',
  'challenged',
];

/**
 * <elohim-reaction-bar> — protocol-native emotional feedback primitive.
 *
 * NOT Facebook-style likes. These are contextual emotional responses that respect
 * content's feedback constraints, distinguish supportive vs critical reactions,
 * mediate potentially harmful reactions (constitutional teaching), and track
 * patterns for social health monitoring.
 *
 * Service dependencies are accepted as properties (callbacks), NOT via Angular DI.
 * The consuming wrapper injects the callbacks; the element dispatches events and
 * calls back through them.
 *
 * Migrated from ReactionBarComponent (app/elohim-app/qahal).
 *
 * @element elohim-reaction-bar
 *
 * @prop {string} entityType - Entity type for governance signals (e.g. 'content', 'proposal')
 * @prop {string} entityId - Entity ID
 * @prop {ReactionConstraints | null} constraints - Reaction constraints from content FeedbackProfile
 * @prop {ReactionCount[]} counts - Current reaction counts from the parent
 * @prop {EmotionalReactionType[]} userReactions - Set of reactions the current user has submitted
 * @prop {boolean} compact - Compact mode (icons only, no labels)
 * @prop {boolean} showCounts - Show reaction counts
 * @prop {Function | null} onReactionSubmit - Called when user submits a reaction
 * @prop {Function | null} onMediationProceed - Called when user proceeds through mediation
 *
 * @fires reaction-submit - { detail: ReactionSubmitEvent } when a reaction is selected
 * @fires mediation-proceed - { detail: MediationProceedEvent } when mediation dialog is resolved
 * @fires reaction-remove - { detail: { type: EmotionalReactionType, entityId: string } } when reaction toggled off
 *
 * @cssprop --elohim-reaction-bar-gap - Gap between reaction buttons
 * @cssprop --elohim-reaction-btn-bg - Reaction button background
 * @cssprop --elohim-reaction-btn-border - Reaction button border
 * @cssprop --elohim-reaction-btn-fg - Reaction button foreground color
 * @cssprop --elohim-reaction-btn-active-bg - Active (selected) button background
 * @cssprop --elohim-reaction-btn-active-border - Active button border color
 * @cssprop --elohim-reaction-btn-radius - Reaction button border radius
 * @cssprop --elohim-reaction-count-fg - Count text color
 * @cssprop --elohim-reaction-mediation-bg - Mediation dialog background
 * @cssprop --elohim-reaction-mediation-fg - Mediation dialog foreground text color
 * @cssprop --elohim-reaction-warning-color - Mediation indicator / warning color
 *
 * @csspart bar - The reaction bar container
 * @csspart button - Each reaction button
 * @csspart icon - The reaction icon span
 * @csspart label - The reaction label span
 * @csspart count - The reaction count span
 * @csspart mediation-overlay - The mediation dialog overlay
 * @csspart mediation-dialog - The mediation dialog panel
 *
 * @slot - No default slot; all content is generated from props
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimReactionBar extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    :host([hidden]) {
      display: none;
    }

    .bar {
      display: flex;
      flex-wrap: wrap;
      gap: var(--elohim-reaction-bar-gap, 0.5rem);
      padding-block: 0.5rem;
    }

    :host([compact]) .bar {
      gap: 0.25rem;
    }

    .reaction-btn {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      border: 1px solid
        var(--elohim-reaction-btn-border, color-mix(in oklch, currentColor 25%, transparent));
      border-radius: var(--elohim-reaction-btn-radius, 1rem);
      background: var(--elohim-reaction-btn-bg, Canvas);
      color: var(--elohim-reaction-btn-fg, CanvasText);
      cursor: pointer;
      font: inherit;
      font-size: 0.875rem;
    }

    .reaction-btn[aria-pressed='true'] {
      background: var(--elohim-reaction-btn-active-bg, Highlight);
      color: var(--elohim-reaction-btn-active-fg, HighlightText);
      border-color: var(--elohim-reaction-btn-active-border, Highlight);
    }

    .reaction-btn[data-mediated='true'] {
      border-style: dashed;
    }

    .reaction-btn:hover,
    .reaction-btn:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }

    .reaction-icon {
      font-size: 1.125rem;
      line-height: 1;
    }

    .reaction-count {
      font-size: 0.75rem;

      /* CanvasText, not GrayText: counts are ACTIVE content and GrayText is
       the system DISABLED color (3.95:1 on a light Canvas — fails 4.5:1) */
      color: var(--elohim-reaction-count-fg, CanvasText);
      min-inline-size: 1rem;
      text-align: center;
    }

    .mediation-indicator {
      color: var(--elohim-reaction-warning-color, Canvas);
      font-weight: bold;
    }

    .mediation-overlay {
      position: fixed;
      inset: 0;
      background: rgb(0 0 0 / 50%);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1000;
    }

    .mediation-dialog {
      background: var(--elohim-reaction-mediation-bg, Canvas);
      color: var(--elohim-reaction-mediation-fg, CanvasText);
      border-radius: 0.5rem;
      padding: 1.5rem;
      max-inline-size: 24rem;
      inline-size: 90%;
    }

    .mediation-dialog h3 {
      margin: 0 0 0.75rem;
    }

    .mediation-actions {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
      margin-block-start: 1rem;
    }

    .mediation-alt-btn,
    .mediation-proceed-btn,
    .mediation-cancel-btn {
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border-radius: 0.25rem;
      cursor: pointer;
      font: inherit;
    }

    .mediation-alt-btn {
      border: 1px solid
        var(--elohim-reaction-btn-border, color-mix(in oklch, currentColor 25%, transparent));
      background: Canvas;
    }

    .mediation-proceed-btn {
      border: 1px solid var(--elohim-reaction-warning-color, Canvas);
      background: transparent;
      color: var(--elohim-reaction-warning-color, Canvas);
    }

    .mediation-cancel-btn {
      border: 1px solid
        var(--elohim-reaction-btn-border, color-mix(in oklch, currentColor 25%, transparent));
      background: transparent;
    }

    @media (pointer: coarse) {
      .reaction-btn {
        min-block-size: 44px;
        min-inline-size: 44px;
      }
    }

    @media (forced-colors: active) {
      .reaction-btn {
        border-color: ButtonText;
        background: ButtonFace;
        color: ButtonText;
      }

      .reaction-btn[aria-pressed='true'] {
        background: Highlight;
        color: HighlightText;
        border-color: Highlight;
      }

      .reaction-btn:hover,
      .reaction-btn:focus-visible {
        outline: 2px solid Highlight;
      }

      .mediation-dialog {
        background: Canvas;
        color: CanvasText;
        border: 1px solid CanvasText;
      }
    }
  `;

  /** Entity type for governance signals */
  @property() entityType = 'content';

  /** Entity ID for governance signals */
  @property() entityId = '';

  /** Reaction constraints from content FeedbackProfile */
  @property({ attribute: false })
  constraints: ReactionConstraints | null = null;

  /** Current reaction counts */
  @property({ attribute: false })
  counts: ReactionCount[] = [];

  /** Reactions the current user has submitted */
  @property({ attribute: false })
  userReactions: EmotionalReactionType[] = [];

  /** Compact mode — icons only */
  @property({ type: Boolean, reflect: true })
  compact = false;

  /** Show reaction counts */
  @property({ type: Boolean })
  showCounts = true;

  /**
   * Optional callback for when a reaction is submitted.
   * Called with the reaction event; parent handles the service calls.
   */
  @property({ attribute: false })
  onReactionSubmit: ((event: ReactionSubmitEvent) => void) | null = null;

  /**
   * Optional callback for when user proceeds through mediation.
   */
  @property({ attribute: false })
  onMediationProceed: ((event: MediationProceedEvent) => void) | null = null;

  @state() private showMediationDialog = false;
  @state() private mediationReaction: (EmotionalReactionType & string) | null = null;
  @state() private mediationConfig: MediatedReaction | null = null;

  private get permittedReactions(): EmotionalReactionType[] {
    if (this.constraints) {
      return this.constraints.permittedTypes;
    }
    return DEFAULT_PERMITTED_TYPES;
  }

  private get mediatedMap(): Map<EmotionalReactionType, MediatedReaction> {
    const map = new Map<EmotionalReactionType, MediatedReaction>();
    for (const m of this.constraints?.mediatedTypes ?? []) {
      map.set(m.type, m);
    }
    return map;
  }

  private getCount(type: EmotionalReactionType): number {
    return this.counts.find(c => c.type === type)?.count ?? 0;
  }

  private isUserReaction(type: EmotionalReactionType): boolean {
    return this.userReactions.includes(type);
  }

  override render() {
    const mediatedMap = this.mediatedMap;

    return html`
      <div class="bar" part="bar" role="group" aria-label="Reactions">
        ${this.permittedReactions.map(type => {
          const mediated = mediatedMap.has(type);
          const count = this.getCount(type);
          const pressed = this.isUserReaction(type);
          const label = REACTION_LABELS[type];

          return html`
            <button
              class="reaction-btn"
              part="button"
              type="button"
              aria-label="${label}"
              aria-pressed=${pressed ? 'true' : 'false'}
              data-testid="reaction-${type}"
              data-mediated=${mediated ? 'true' : 'false'}
              title="${label}"
              @click=${() => this.handleReactionClick(type, mediated, mediatedMap.get(type))}
            >
              <span class="reaction-icon" part="icon" aria-hidden="true">
                ${REACTION_ICONS[type]}
              </span>
              ${this.compact
                ? nothing
                : html`
                    <span class="reaction-label" part="label">${label}</span>
                  `}
              ${this.showCounts && count > 0
                ? html`
                    <span class="reaction-count" part="count">${count}</span>
                  `
                : nothing}
              ${mediated
                ? html`
                    <span class="mediation-indicator" aria-hidden="true">*</span>
                  `
                : nothing}
            </button>
          `;
        })}
      </div>

      ${this.showMediationDialog && this.mediationConfig ? this.renderMediationDialog() : nothing}
    `;
  }

  private renderMediationDialog() {
    const config = this.mediationConfig!;
    return html`
      <div
        class="mediation-overlay"
        part="mediation-overlay"
        role="button"
        tabindex="-1"
        aria-label="Close mediation dialog"
        data-testid="mediation-overlay"
        @click=${this.closeMediationDialog}
        @keydown=${(e: KeyboardEvent) => {
          if (e.key === 'Escape') this.closeMediationDialog();
        }}
      >
        <div
          class="mediation-dialog"
          part="mediation-dialog"
          role="dialog"
          aria-label="Mediation reflection"
          data-testid="mediation-dialog"
          @click=${(e: Event) => e.stopPropagation()}
          @keydown=${(e: KeyboardEvent) => {
            // Keep clicks/keys inside the dialog from reaching the overlay, but let
            // Escape bubble so the overlay's close handler still fires.
            if (e.key !== 'Escape') e.stopPropagation();
          }}
        >
          <h3>A moment of reflection</h3>
          <p>${config.constitutionalReasoning}</p>
          <div class="mediation-actions">
            ${config.suggestedAlternatives?.length
              ? html`
                  <p>Consider expressing this differently:</p>
                  ${config.suggestedAlternatives.map(
                    alt => html`
                      <button
                        class="mediation-alt-btn"
                        type="button"
                        data-testid="mediation-alt-${alt}"
                        @click=${() => this.onMediationResponse(false, alt)}
                      >
                        ${REACTION_ICONS[alt]} ${REACTION_LABELS[alt]}
                      </button>
                    `
                  )}
                `
              : nothing}
            <button
              class="mediation-proceed-btn"
              type="button"
              data-testid="mediation-proceed-btn"
              @click=${() => this.onMediationResponse(true)}
            >
              Proceed anyway
            </button>
            <button
              class="mediation-cancel-btn"
              type="button"
              data-testid="mediation-cancel-btn"
              @click=${this.closeMediationDialog}
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    `;
  }

  private handleReactionClick(
    type: EmotionalReactionType,
    mediated: boolean,
    mediationConfig: MediatedReaction | undefined
  ): void {
    // Toggle: if user already reacted, remove
    if (this.isUserReaction(type)) {
      this.dispatchEvent(
        new CustomEvent('reaction-remove', {
          detail: { type, entityId: this.entityId },
          bubbles: true,
          composed: true,
        })
      );
      return;
    }

    // Mediated reaction — show dialog first
    if (mediated && mediationConfig) {
      this.mediationReaction = type;
      this.mediationConfig = mediationConfig;
      this.showMediationDialog = true;
      return;
    }

    // Direct submission
    this.submitReaction(type);
  }

  private onMediationResponse(proceed: boolean, alt?: EmotionalReactionType): void {
    if (!this.mediationReaction || !this.mediationConfig) {
      this.closeMediationDialog();
      return;
    }

    const proceedEvent: MediationProceedEvent = {
      type: this.mediationReaction,
      proceededAnyway: proceed,
      alternativeChosen: alt,
    };

    if (this.onMediationProceed) {
      this.onMediationProceed(proceedEvent);
    }

    this.dispatchEvent(
      new CustomEvent('mediation-proceed', {
        detail: proceedEvent,
        bubbles: true,
        composed: true,
      })
    );

    if (proceed) {
      if (this.mediationConfig.proceedBehavior.visibleToOthers) {
        this.submitReaction(this.mediationReaction);
      }
    } else if (alt) {
      this.submitReaction(alt);
    }

    this.closeMediationDialog();
  }

  private closeMediationDialog(): void {
    this.showMediationDialog = false;
    this.mediationReaction = null;
    this.mediationConfig = null;
  }

  private submitReaction(type: EmotionalReactionType): void {
    const event: ReactionSubmitEvent = {
      type,
      entityType: this.entityType,
      entityId: this.entityId,
    };

    if (this.onReactionSubmit) {
      this.onReactionSubmit(event);
    }

    this.dispatchEvent(
      new CustomEvent('reaction-submit', {
        detail: event,
        bubbles: true,
        composed: true,
      })
    );
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-reaction-bar': ElohimReactionBar;
  }
}
