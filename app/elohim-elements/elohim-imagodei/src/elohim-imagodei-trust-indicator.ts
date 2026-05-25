import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type TrustMode = 'doorway-host' | 'peer-conductor';

/**
 * <elohim-imagodei-trust-indicator> — small chip showing where the conductor
 * lives. Two modes, two glyphs, two accent colors. Tap emits a tap event
 * that consumers (omnibar, portal-shell) can surface a details panel from.
 *
 * Security framing: the trust mode is OPERATIONAL (which conductor signs
 * for you), NOT philosophical. Both modes carry the same community-attested
 * security; the difference is where the conductor-compute lives. Avoid any
 * crypto-bro/sovereign-key framing — see spec §0 + §1.1.
 *
 * @element elohim-imagodei-trust-indicator
 *
 * @prop {TrustMode} trustMode - 'doorway-host' | 'peer-conductor'
 * @prop {string} authorityLabel - human-readable authority description
 * @prop {boolean} flywheelHint - render the "you can graduate" hint (doorway-host only)
 *
 * @fires {CustomEvent<{trustMode: TrustMode, authorityLabel: string}>} trust-indicator-tap
 *
 * @cssprop --elohim-trust-bg - Chip background color (default: transparent)
 * @cssprop --elohim-trust-fg - Chip foreground / text color (default: inherit)
 * @cssprop --elohim-trust-border - Chip border (overrides the per-mode accent)
 * @cssprop --elohim-trust-host-accent - Border accent for doorway-host mode
 * @cssprop --elohim-trust-peer-accent - Border accent for peer-conductor mode
 * @cssprop --elohim-trust-radius - Border radius (default: 999px pill)
 * @cssprop --elohim-trust-padding-block - Block-axis padding
 * @cssprop --elohim-trust-padding-inline - Inline-axis padding
 * @cssprop --elohim-trust-hint-size - Font size for the flywheel hint span
 *
 * @csspart mode - The outer button element; carries data-mode attribute
 * @csspart glyph - The mode-differentiating icon span (aria-hidden)
 * @csspart label - The text content span
 * @csspart flywheel-hint - The "(flywheel)" graduation hint span; absent when not shown
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiTrustIndicator extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-block;
      font: inherit;
    }

    button {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      padding-block: var(--elohim-trust-padding-block, 0.25rem);
      padding-inline: var(--elohim-trust-padding-inline, 0.625rem);
      border: var(--elohim-trust-border, 1px solid color-mix(in srgb, currentColor 25%, transparent));
      border-radius: var(--elohim-trust-radius, 999px);
      background: var(--elohim-trust-bg, transparent);
      color: var(--elohim-trust-fg, inherit);
      cursor: pointer;
      font: inherit;
      text-align: start;
    }

    button:hover,
    button:focus-visible {
      background: color-mix(in srgb, currentColor 6%, transparent);
      outline: none;
    }

    button:focus-visible {
      outline: 2px solid currentColor;
      outline-offset: 2px;
    }

    [data-mode='doorway-host'] {
      border-color: var(
        --elohim-trust-host-accent,
        color-mix(in srgb, currentColor 30%, transparent)
      );
    }

    [data-mode='peer-conductor'] {
      border-color: var(
        --elohim-trust-peer-accent,
        color-mix(in srgb, currentColor 30%, transparent)
      );
    }

    [part='glyph'] {
      line-height: 1;
      flex-shrink: 0;
    }

    [part='flywheel-hint'] {
      font-size: var(--elohim-trust-hint-size, 0.75rem);
      opacity: 0.7;
      margin-inline-start: 0.25rem;
    }

    @media (forced-colors: active) {
      button {
        border: 1px solid CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      button:hover,
      button:focus-visible {
        background: Highlight;
        color: HighlightText;
        outline: 2px solid ButtonText;
      }
    }
  `;

  @property({ attribute: 'trust-mode' }) trustMode: TrustMode = 'doorway-host';
  @property({ attribute: 'authority-label' }) authorityLabel = '';
  @property({ attribute: 'flywheel-hint', type: Boolean }) flywheelHint = false;

  override render() {
    const isDoorwayHost = this.trustMode === 'doorway-host';
    const modeIcon = isDoorwayHost ? '⌂' : '◇'; // ⌂ or ◇
    const modeLabel = isDoorwayHost ? 'Hosted via' : 'Your conductor —';
    const showFlywheelHint = this.flywheelHint && isDoorwayHost;

    return html`
      <button
        type="button"
        part="mode"
        data-mode=${this.trustMode}
        aria-label="${modeLabel} ${this.authorityLabel}${showFlywheelHint ? ' (flywheel — you can graduate to your own conductor)' : ''}"
        @click=${this._onTap}
      >
        <span part="glyph" aria-hidden="true">${modeIcon}</span>
        <span part="label"><strong>${modeLabel}</strong> ${this.authorityLabel}</span>
        ${showFlywheelHint
          ? html`<span part="flywheel-hint">(flywheel)</span>`
          : ''}
      </button>
    `;
  }

  private _onTap = () => {
    this.dispatchEvent(
      new CustomEvent('trust-indicator-tap', {
        detail: { trustMode: this.trustMode, authorityLabel: this.authorityLabel },
        bubbles: true,
        composed: true,
      })
    );
  };
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-trust-indicator': ElohimImagodeiTrustIndicator;
  }
}
