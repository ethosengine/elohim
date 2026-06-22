import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, nothing } from 'lit';
import { property } from 'lit/decorators.js';

import type { ContributorPresenceView } from '@elohim/storage-client';

export type ContributorPresenceState = 'unclaimed' | 'stewarded' | 'claimed' | 'claiming';

/**
 * Derives a human-readable badge label from a `presenceState` string.
 * Degrades gracefully to a capitalised fallback on unknown values.
 *
 * @internal — exposed for unit-testing badge logic in isolation.
 */
export function presenceStateBadge(state: string): {
  label: string;
  variant: 'unclaimed' | 'stewarded' | 'claimed' | 'claiming' | 'unknown';
} {
  switch (state) {
    case 'unclaimed':
      return { label: 'Open to steward', variant: 'unclaimed' };
    case 'stewarded':
      return { label: 'Stewarded', variant: 'stewarded' };
    case 'claimed':
      return { label: 'Claimed', variant: 'claimed' };
    case 'claiming':
      return { label: 'Claiming…', variant: 'claiming' };
    default:
      // Unknown variant — surface the raw value in sentence-case so the UI
      // doesn't explode on a future state added in the backend.
      return {
        label: state.length > 0 ? state.charAt(0).toUpperCase() + state.slice(1) : 'Unknown',
        variant: 'unknown',
      };
  }
}

/**
 * <elohim-contributor-card> — renders ONE contributor presence as a card.
 *
 * Used in the "Contributors / Inspired by" panel on the EPR content viewer
 * and, later, in the presence list and profile surface.
 *
 * Data shape: `ContributorPresenceView` from `@elohim/storage-client`
 * (the ts-rs generated wire type — the authoritative shape).
 *
 * **Presence type (person vs organisation):** the wire type does not carry a
 * `presenceType` discriminator. Consumers that have this information can
 * supply it via the `presence-type` property or by slotting a glyph into the
 * named `presence-type-glyph` slot.  When neither is supplied the indicator
 * is omitted — that is the correct blank-slate behaviour.
 *
 * **Selection affordance:** activation emits `contributor-select`
 * `{ id: string }` on the element (bubbles, composed). Consumers that already
 * have a route may also supply `href`; when set the element renders an `<a>`
 * instead of a `<button>` for the activation target.
 *
 * **Unclaimed state framing:** `presenceState === 'unclaimed'` means the
 * contributor has not yet been matched with a steward. This is an *opportunity*
 * — recognition-before-registration — never an error. The badge label and
 * variant name reflect this; Library B can reinforce the framing via token
 * binding.
 *
 * @element elohim-contributor-card
 *
 * @prop {ContributorPresenceView | null} presence - The contributor presence view
 * @prop {string | null} href - Optional route to the contributor profile page
 * @prop {boolean} compact - When true, hides affinityTotal and uniqueEngagers
 * @prop {'person' | 'organization' | null} presenceType - Optional type hint (not on wire type)
 *
 * @slot presence-type-glyph - Glyph or icon for the presence type (person/org indicator);
 *   rendered inline next to the display name.
 *
 * @fires {CustomEvent<{id: string}>} contributor-select - Emitted on card activation
 *   (click or keyboard Enter/Space on the activation target). Always emitted whether
 *   or not `href` is set.
 *
 * @cssprop --elohim-contributor-card-bg - Card background (default: Canvas)
 * @cssprop --elohim-contributor-card-fg - Card foreground text (default: CanvasText)
 * @cssprop --elohim-contributor-card-border - Card border (default: 1px solid currentColor at 15% opacity)
 * @cssprop --elohim-contributor-card-radius - Card border-radius (default: 0.5rem)
 * @cssprop --elohim-contributor-card-padding - Card padding (default: 1rem)
 * @cssprop --elohim-contributor-card-gap - Inner layout gap (default: 0.75rem)
 * @cssprop --elohim-contributor-card-avatar-size - Avatar diameter (default: 3rem)
 * @cssprop --elohim-contributor-card-avatar-bg - Avatar placeholder background (default: 20% currentColor)
 * @cssprop --elohim-contributor-card-badge-bg - State badge background (default: transparent)
 * @cssprop --elohim-contributor-card-badge-fg - State badge text color (default: inherit)
 * @cssprop --elohim-contributor-card-badge-border - State badge border (default: 1px solid currentColor at 30%)
 * @cssprop --elohim-contributor-card-badge-radius - State badge border-radius (default: 999px)
 * @cssprop --elohim-contributor-card-stats-fg - Stats row foreground (default: 70% currentColor)
 * @cssprop --elohim-contributor-card-hover-bg - Background on activation target hover/focus (default: 4% currentColor)
 *
 * @csspart container - The outer card container (<article>)
 * @csspart avatar - The avatar image or placeholder circle
 * @csspart header - The name + badge row
 * @csspart display-name - The contributor display name
 * @csspart presence-type-indicator - Wrapper for the presence-type-glyph slot
 * @csspart state-badge - The presenceState badge
 * @csspart stats - The recognition stats row
 * @csspart recognition-score - The recognition score value
 * @csspart citation-count - The citation count value
 * @csspart affinity-row - The affinity/engager row (hidden in compact mode)
 * @csspart activation - The activation anchor or button
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimContributorCard extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      font: inherit;
    }

    article {
      display: flex;
      flex-flow: column nowrap;
      gap: var(--elohim-contributor-card-gap, 0.75rem);
      padding: var(--elohim-contributor-card-padding, 1rem);
      background: var(--elohim-contributor-card-bg, Canvas);
      color: var(--elohim-contributor-card-fg, CanvasText);
      border: var(
        --elohim-contributor-card-border,
        1px solid color-mix(in srgb, currentColor 15%, transparent)
      );
      border-radius: var(--elohim-contributor-card-radius, 0.5rem);
    }

    /* Activation target — button (no href) or anchor (with href); single focusable */

    [part='activation'] {
      display: flex;
      flex-flow: column nowrap;
      gap: var(--elohim-contributor-card-gap, 0.75rem);
      inline-size: 100%;
      text-align: start;
      color: inherit;
      text-decoration: none;
      font: inherit;
      cursor: pointer;
      background: transparent;
      border: none;
      padding: 0;
    }

    [part='activation']:hover,
    [part='activation']:focus-visible {
      background: var(
        --elohim-contributor-card-hover-bg,
        color-mix(in srgb, currentColor 4%, var(--elohim-contributor-card-bg, Canvas))
      );
    }

    /* Touch target — ensure min 44×44 under pointer:coarse */
    @media (pointer: coarse) {
      [part='activation'] {
        min-block-size: 44px;
      }
    }

    /* Header row: avatar + name/badge */

    .card-header {
      display: flex;
      flex-flow: row nowrap;
      align-items: flex-start;
      gap: var(--elohim-contributor-card-gap, 0.75rem);
    }

    [part='avatar'] {
      flex-shrink: 0;
      inline-size: var(--elohim-contributor-card-avatar-size, 3rem);
      block-size: var(--elohim-contributor-card-avatar-size, 3rem);
      border-radius: 50%;
      object-fit: cover;
      background: var(
        --elohim-contributor-card-avatar-bg,
        color-mix(in srgb, currentColor 20%, transparent)
      );
      display: inline-flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
    }

    [part='avatar'] img {
      inline-size: 100%;
      block-size: 100%;
      object-fit: cover;
    }

    .avatar-initials {
      font-size: 0.875rem;
      font-weight: 600;
      line-height: 1;
    }

    [part='header'] {
      flex: 1;
      min-inline-size: 0;
      display: flex;
      flex-flow: column nowrap;
      gap: 0.25rem;
    }

    .name-row {
      display: flex;
      flex-flow: row wrap;
      align-items: center;
      gap: 0.375rem;
    }

    [part='display-name'] {
      font-weight: 600;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    [part='presence-type-indicator'] {
      display: inline-flex;
      align-items: center;
      line-height: 1;
    }

    /* State badge */

    [part='state-badge'] {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding-block: 0.125rem;
      padding-inline: 0.5rem;
      font-size: 0.75rem;
      background: var(--elohim-contributor-card-badge-bg, transparent);
      color: var(--elohim-contributor-card-badge-fg, inherit);
      border: var(
        --elohim-contributor-card-badge-border,
        1px solid color-mix(in srgb, currentColor 30%, transparent)
      );
      border-radius: var(--elohim-contributor-card-badge-radius, 999px);
      white-space: nowrap;
    }

    /* Prefix marker — ensures state is never conveyed by color alone (a11y). */
    [part='state-badge']::before {
      content: attr(data-badge-marker);
    }

    /* Stats row */

    [part='stats'] {
      display: flex;
      flex-flow: row wrap;
      gap: 1rem;
      font-size: 0.8125rem;
      color: var(
        --elohim-contributor-card-stats-fg,
        color-mix(in srgb, currentColor 70%, transparent)
      );
    }

    .stat-item {
      display: flex;
      flex-flow: row nowrap;
      align-items: center;
      gap: 0.25rem;
    }

    .stat-label {
      font-size: 0.75rem;
    }

    .stat-value {
      font-weight: 600;
    }

    /* Empty state (no presence supplied) */

    .empty-state {
      padding: var(--elohim-contributor-card-padding, 1rem);
      text-align: center;
      font-size: 0.875rem;
    }

    /* forced-colors */

    @media (forced-colors: active) {
      article {
        border: 1px solid CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      [part='activation']:hover,
      [part='activation']:focus-visible {
        background: Highlight;
        color: HighlightText;
      }

      [part='state-badge'] {
        border: 1px solid CanvasText;
        background: ButtonFace;
        color: ButtonText;
      }

      [part='avatar'] {
        border: 1px solid CanvasText;
        background: ButtonFace;
        color: ButtonText;
      }

      [part='stats'] {
        color: CanvasText;
      }
    }
  `;

  /** The contributor presence view to render. */
  @property({ attribute: false }) presence: ContributorPresenceView | null = null;

  /** Optional profile page route. When set, renders an anchor instead of a button. */
  @property() href: string | null = null;

  /**
   * When true, the affinityTotal and uniqueEngagers stats are hidden.
   * Useful in tight-width contexts.
   */
  @property({ type: Boolean }) compact = false;

  /**
   * **Navigation note:** when `href` is set the element renders a native `<a>` for the
   * activation target and `contributor-select` is still emitted on click. However, native
   * anchor navigation is **not suppressed** — the browser will follow the href. Consumers
   * doing client-side routing should either (a) listen to `contributor-select` with
   * `href=null` (button path), or (b) call `event.preventDefault()` in their own
   * `contributor-select` handler before performing a SPA navigation.
   */

  /**
   * Optional presence type hint — NOT part of the wire type, supplied by
   * the consumer who has domain context. When omitted (null) the type
   * indicator is hidden.
   */
  @property({ attribute: 'presence-type' }) presenceType: 'person' | 'organization' | null = null;

  override render() {
    if (!this.presence) {
      return html`
        <div class="empty-state" role="status">No contributor data available.</div>
      `;
    }

    const p = this.presence;
    const badge = presenceStateBadge(p.presenceState);
    const badgeMarker = this._badgeMarker(badge.variant);
    const initials = this._initials(p.displayName);
    const hasTypeGlyph = this.presenceType !== null;

    const cardContent = html`
      <div class="card-header">
        <div part="avatar" aria-hidden="true" role="presentation">
          ${p.image
            ? html`
                <img src="${p.image}" alt="" loading="lazy" />
              `
            : html`
                <span class="avatar-initials" aria-hidden="true">${initials}</span>
              `}
        </div>

        <div part="header">
          <div class="name-row">
            <span part="display-name">${p.displayName}</span>
            ${hasTypeGlyph
              ? html`
                  <span
                    part="presence-type-indicator"
                    aria-label="${this.presenceType === 'organization' ? 'Organisation' : 'Person'}"
                  >
                    <slot name="presence-type-glyph">
                      ${this.presenceType === 'organization' ? '◻' : '◉'}
                    </slot>
                  </span>
                `
              : html`
                  <span part="presence-type-indicator" hidden>
                    <slot name="presence-type-glyph"></slot>
                  </span>
                `}
          </div>

          <span
            part="state-badge"
            data-variant="${badge.variant}"
            data-badge-marker="${badgeMarker}"
            aria-label="Status: ${badge.label}"
          >
            ${badge.label}
          </span>
        </div>
      </div>

      <div part="stats" aria-label="Recognition statistics">
        <span class="stat-item">
          <span class="stat-label">Score</span>
          <span part="recognition-score" class="stat-value">${Math.round(p.recognitionScore)}</span>
        </span>
        <span class="stat-item">
          <span class="stat-label">Citations</span>
          <span part="citation-count" class="stat-value">${p.citationCount}</span>
        </span>
        ${!this.compact
          ? html`
              <span part="affinity-row" class="stat-item">
                <span class="stat-label">Affinity</span>
                <span class="stat-value">${Math.round(p.affinityTotal)}</span>
              </span>
              <span class="stat-item">
                <span class="stat-label">Engagers</span>
                <span class="stat-value">${p.uniqueEngagers}</span>
              </span>
            `
          : nothing}
      </div>
    `;

    return html`
      <article part="container" aria-label="Contributor: ${p.displayName}">
        ${this.href
          ? html`
              <a
                part="activation"
                href="${this.href}"
                aria-label="View profile for ${p.displayName}"
                @click=${this._onActivate}
              >
                ${cardContent}
              </a>
            `
          : html`
              <button
                part="activation"
                type="button"
                aria-label="Select contributor ${p.displayName}"
                @click=${this._onActivate}
              >
                ${cardContent}
              </button>
            `}
      </article>
    `;
  }

  private _badgeMarker(variant: string): string {
    switch (variant) {
      case 'unclaimed':
        return '◇ ';
      case 'stewarded':
        return '◆ ';
      case 'claimed':
        return '● ';
      case 'claiming':
        return '◌ ';
      default:
        return '○ ';
    }
  }

  private _initials(name: string): string {
    const parts = name.trim().split(/\s+/);
    const first = parts[0];
    if (!first) return '?';
    if (parts.length === 1) return first.slice(0, 2).toUpperCase();
    const last = parts[parts.length - 1];
    return ((first[0] ?? '') + (last?.[0] ?? '')).toUpperCase();
  }

  private _onActivate = (_e: Event) => {
    if (!this.presence) return;
    this.dispatchEvent(
      new CustomEvent('contributor-select', {
        detail: { id: this.presence.id },
        bubbles: true,
        composed: true,
      })
    );
  };
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-contributor-card': ElohimContributorCard;
  }
}
