import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property, state } from 'lit/decorators.js';

import type { TrustMode } from './elohim-imagodei-trust-indicator.js';

export type { TrustMode };

export interface ClaimRef {
  id: string;
  label: string;
  description?: string;
}

export interface RequestingClient {
  id: string;
  displayName: string;
  brandMark?: string;
}

/**
 * <elohim-imagodei-consent-card> — RFC-6749 authorization-step consent surface.
 *
 * Renders the RP brand strip (id + displayName + optional brandMark) plus
 * per-claim toggleable rows. Required claims are locked-on: their toggle is
 * checked + disabled. Inherits `trustMode` from the parent portal-shell so the
 * user can see which mode they are signing this consent as.
 *
 * Defense-in-depth: if a required claim somehow gets toggled off (shouldn't be
 * possible in UI, but might be via direct DOM manipulation), the approve handler
 * converts the event to a decline with reason `partial-decline-blocked` rather
 * than allowing a partial grant to land.
 *
 * @element elohim-imagodei-consent-card
 *
 * @prop {RequestingClient} requestingClient - RP identity (id, displayName, optional brandMark)
 * @prop {ClaimRef[]} requestedClaims - Full list of claims the RP is requesting
 * @prop {string[]} requiredClaims - Subset of claim ids that cannot be toggled off
 * @prop {TrustMode} trustMode - Inherited from parent shell; adapts contextual copy
 *
 * @slot policy-link - Link to the RP's privacy policy / terms (rendered below claim rows)
 * @slot claim-detail - Per-claim detail content (named by claim id; rendered inside each row)
 *
 * @fires {CustomEvent<{grantedClaims: string[]}>} approve - User approved; detail lists granted claim ids
 * @fires {CustomEvent<{reason: string}>} decline - User declined; reason is 'user-rejected' or 'partial-decline-blocked'
 *
 * @cssprop --elohim-consent-rp-bg - RP brand strip background (default: 6% currentColor mix)
 * @cssprop --elohim-consent-rp-fg - RP brand strip foreground (default: inherit)
 * @cssprop --elohim-consent-claim-row-bg - Claim row background (default: transparent)
 * @cssprop --elohim-consent-claim-row-border - Claim row border (default: 14% currentColor mix)
 * @cssprop --elohim-consent-approve-bg - Approve button background (default: transparent)
 * @cssprop --elohim-consent-approve-fg - Approve button foreground (default: inherit)
 * @cssprop --elohim-consent-decline-bg - Decline button background (default: transparent)
 * @cssprop --elohim-consent-decline-fg - Decline button foreground (default: inherit)
 * @cssprop --elohim-consent-gap - Grid gap between sections (default: 1rem)
 * @cssprop --elohim-consent-radius - Border radius for rows and buttons (default: 6px)
 * @cssprop --elohim-consent-muted-opacity - Opacity for muted / secondary text (default: 0.7)
 *
 * @csspart rp - The RP brand strip container
 * @csspart rp-mark - The brand mark / icon placeholder inside the RP strip
 * @csspart client-name - The RP display name element inside the RP strip
 * @csspart claim-row - Individual claim row (carries data-claim-id attribute)
 * @csspart actions - The approve/decline button row
 * @csspart approve - The Approve button
 * @csspart decline - The Decline button
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiConsentCard extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      font: inherit;
    }

    .card {
      display: grid;
      gap: var(--elohim-consent-gap, 1rem);
    }

    [part='rp'] {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding-block: 0.75rem;
      padding-inline: 0.75rem;
      background: var(
        --elohim-consent-rp-bg,
        color-mix(in srgb, currentColor 6%, transparent)
      );
      color: var(--elohim-consent-rp-fg, inherit);
      border-radius: var(--elohim-consent-radius, 6px);
    }

    [part='rp-mark'] {
      inline-size: 32px;
      block-size: 32px;
      border-radius: 50%;
      background: color-mix(in srgb, currentColor 16%, transparent);
      flex-shrink: 0;
      overflow: hidden;
    }

    [part='rp-mark'] img {
      inline-size: 100%;
      block-size: 100%;
      object-fit: cover;
    }

    .rp-details {
      display: grid;
      gap: 0.125rem;
    }

    [part='client-name'] {
      font-weight: bold;
    }

    .rp-subtext {
      opacity: var(--elohim-consent-muted-opacity, 0.7);
      font-size: 0.875rem;
    }

    [part='claim-row'] {
      display: grid;
      grid-template-columns: auto 1fr;
      gap: 0.75rem;
      align-items: start;
      padding-block: 0.75rem;
      padding-inline: 0.75rem;
      background: var(--elohim-consent-claim-row-bg, transparent);
      border: 1px solid
        var(
          --elohim-consent-claim-row-border,
          color-mix(in srgb, currentColor 14%, transparent)
        );
      border-radius: var(--elohim-consent-radius, 6px);
      cursor: pointer;
    }

    [part='claim-row'] input[type='checkbox'] {
      margin-block-start: 0.25rem;
      cursor: pointer;
      flex-shrink: 0;
    }

    [part='claim-row'] input[type='checkbox']:disabled {
      cursor: default;
    }

    .claim-details {
      display: grid;
      gap: 0.25rem;
    }

    .claim-label {
      font-weight: bold;
    }

    .required-badge {
      opacity: var(--elohim-consent-muted-opacity, 0.7);
      font-size: 0.8125rem;
      font-weight: normal;
    }

    .claim-description {
      opacity: var(--elohim-consent-muted-opacity, 0.7);
      font-size: 0.875rem;
    }

    [part='actions'] {
      display: flex;
      gap: 0.5rem;
      justify-content: end;
    }

    [part='approve'],
    [part='decline'] {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid currentColor;
      border-radius: var(--elohim-consent-radius, 6px);
      font: inherit;
      cursor: pointer;
    }

    [part='approve'] {
      background: var(--elohim-consent-approve-bg, transparent);
      color: var(--elohim-consent-approve-fg, inherit);
    }

    [part='decline'] {
      background: var(--elohim-consent-decline-bg, transparent);
      color: var(--elohim-consent-decline-fg, inherit);
    }

    [part='approve']:focus-visible,
    [part='decline']:focus-visible {
      outline: 2px solid currentColor;
      outline-offset: 2px;
    }

    @media (forced-colors: active) {
      [part='rp'] {
        background: Canvas;
        color: CanvasText;
        border: 1px solid CanvasText;
      }

      [part='rp-mark'] {
        background: ButtonFace;
        border: 1px solid CanvasText;
      }

      [part='claim-row'] {
        border-color: CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      [part='approve'] {
        border-color: ButtonText;
        background: ButtonFace;
        color: ButtonText;
      }

      [part='decline'] {
        border-color: CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      [part='approve']:hover,
      [part='approve']:focus-visible,
      [part='decline']:hover,
      [part='decline']:focus-visible {
        background: Highlight;
        color: HighlightText;
        outline: 2px solid ButtonText;
      }
    }
  `;

  /** RP identity: id, displayName, optional brandMark URL. */
  @property({ attribute: false }) requestingClient: RequestingClient = { id: '', displayName: '' };

  /** Full list of claims the RP is requesting access to. */
  @property({ attribute: false }) requestedClaims: ClaimRef[] = [];

  /**
   * Subset of requestedClaims ids that the user cannot opt out of.
   * The toggle for these is rendered checked + disabled.
   */
  @property({ attribute: false }) requiredClaims: string[] = [];

  /**
   * Operational trust mode set by portal-shell on slotchange.
   * Controls contextual copy. Default: doorway-host.
   */
  @property() trustMode: TrustMode = 'doorway-host';

  /** Current set of granted claim ids. Initialized from requestedClaims on first update. */
  @state() private _granted: Set<string> = new Set();
  @state() private _initialized = false;

  protected override updated(): void {
    if (!this._initialized && this.requestedClaims.length > 0) {
      // All claims start granted; the user may then opt out of optional ones.
      this._granted = new Set(this.requestedClaims.map((c) => c.id));
      this._initialized = true;
    }
  }

  override render() {
    return html`
      <div class="card">
        ${this._renderRpStrip()}
        ${this.requestedClaims.map((c) => this._renderClaimRow(c))}
        <slot name="policy-link"></slot>
        <div class="actions" part="actions">
          <button type="button" part="decline" @click=${this._decline}>Decline</button>
          <button type="button" part="approve" @click=${this._approve}>Approve</button>
        </div>
      </div>
    `;
  }

  private _renderRpStrip() {
    const { displayName, brandMark } = this.requestingClient;
    return html`
      <div part="rp">
        <div part="rp-mark" aria-hidden="true">
          ${brandMark ? html`<img src=${brandMark} alt="" />` : ''}
        </div>
        <div class="rp-details">
          <div part="client-name">${displayName}</div>
          <div class="rp-subtext">is asking for access to:</div>
        </div>
      </div>
    `;
  }

  private _renderClaimRow(claim: ClaimRef) {
    const isRequired = this.requiredClaims.includes(claim.id);
    const isGranted = this._granted.has(claim.id);
    return html`
      <label part="claim-row" data-claim-id=${claim.id}>
        <input
          type="checkbox"
          ?checked=${isGranted}
          ?disabled=${isRequired}
          @change=${(e: Event) => this._toggleClaim(claim.id, (e.target as HTMLInputElement).checked)}
        />
        <div class="claim-details">
          <div class="claim-label">
            ${claim.label}
            ${isRequired
              ? html`<span class="required-badge">(required)</span>`
              : ''}
          </div>
          ${claim.description
            ? html`<div class="claim-description">${claim.description}</div>`
            : ''}
          <slot name="claim-detail"></slot>
        </div>
      </label>
    `;
  }

  private _toggleClaim(id: string, on: boolean): void {
    const next = new Set(this._granted);
    if (on) {
      next.add(id);
    } else {
      next.delete(id);
    }
    this._granted = next;
  }

  private _approve = (): void => {
    // Defense-in-depth: if any required claim has been removed (e.g. via direct
    // DOM manipulation), block the approval and emit decline instead.
    for (const required of this.requiredClaims) {
      if (!this._granted.has(required)) {
        this.dispatchEvent(
          new CustomEvent('decline', {
            detail: { reason: 'partial-decline-blocked' },
            bubbles: true,
            composed: true,
          })
        );
        return;
      }
    }
    this.dispatchEvent(
      new CustomEvent('approve', {
        detail: { grantedClaims: Array.from(this._granted) },
        bubbles: true,
        composed: true,
      })
    );
  };

  private _decline = (): void => {
    this.dispatchEvent(
      new CustomEvent('decline', {
        detail: { reason: 'user-rejected' },
        bubbles: true,
        composed: true,
      })
    );
  };
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-consent-card': ElohimImagodeiConsentCard;
  }
}
