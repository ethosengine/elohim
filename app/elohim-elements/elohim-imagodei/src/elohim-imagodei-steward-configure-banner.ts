import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * Steward configure banner — prominent banner shown when a steward edits a stewardee's settings.
 *
 * Displays at the top of the settings palette when a steward is configuring another person's
 * settings. Includes a witness/attestation affordance — making clear that changes will be
 * co-signed by the commons-elohim of the relevant Qahal.
 *
 * @element elohim-imagodei-steward-configure-banner
 *
 * @prop {string} stewardName - The steward doing the editing
 * @prop {string} stewardeeName - The human whose settings are being configured
 * @prop {string} witnessQahalName - The Qahal whose commons-elohim will co-sign the change
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimImagodeiStewardConfigureBanner extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    .banner {
      display: block;
      padding-block: 0.875rem;
      padding-inline: 1rem;
      border-radius: 6px;
      background: var(--elohim-color-protected-bg, #fef3c7);
      border: 1.5px solid var(--elohim-color-protected-border, #f59e0b);
      color: var(--elohim-color-protected-fg, #92400e);
    }

    .banner-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-block-end: 0.5rem;
    }

    .icon {
      font-size: 1.125rem;
      line-height: 1;
      flex-shrink: 0;
    }

    .title {
      font-size: 0.875rem;
      font-weight: 700;
    }

    .message {
      font-size: 0.875rem;
      line-height: 1.5;
      margin-block-end: 0.75rem;
    }

    .witness {
      font-size: 0.8rem;
      color: var(--elohim-color-protected-fg, #92400e);
      opacity: 0.85;
      margin-block-end: 0.75rem;
    }

    .witness strong {
      font-weight: 600;
    }

    .continue-btn {
      font-size: 0.8rem;
      padding-block: 0.35rem;
      padding-inline: 0.875rem;
      background: var(--elohim-color-protected-fg, #92400e);
      color: #fff;
      border: 0;
      border-radius: 4px;
      cursor: pointer;
      font-weight: 500;
    }

    .continue-btn:focus-visible {
      outline: 2px solid var(--elohim-color-focus, #6c9);
      outline-offset: 2px;
    }
  `;

  @property({ type: String, attribute: 'steward-name' }) stewardName = '';
  @property({ type: String, attribute: 'stewardee-name' }) stewardeeName = '';
  @property({ type: String, attribute: 'witness-qahal-name' }) witnessQahalName = '';

  override render() {
    return html`
      <div
        class="banner"
        role="region"
        aria-label="Steward configuration mode — ${this.stewardName} configuring settings for ${this
          .stewardeeName}"
      >
        <div class="banner-header">
          <span class="icon" aria-hidden="true">⚠</span>
          <span class="title">Steward Configuration Mode</span>
        </div>
        <p class="message">
          You (${this.stewardName}) are configuring settings for
          <strong>${this.stewardeeName}</strong>
          .
        </p>
        <p class="witness">
          Changes will be witnessed by the commons-elohim of
          <strong>${this.witnessQahalName}</strong>
          .
        </p>
        <button
          class="continue-btn"
          aria-label="Continue configuring settings for ${this.stewardeeName}"
        >
          Continue
        </button>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-steward-configure-banner': ElohimImagodeiStewardConfigureBanner;
  }
}
