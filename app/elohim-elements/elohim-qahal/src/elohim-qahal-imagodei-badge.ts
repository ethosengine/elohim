import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type StandingTier = 'visitor' | 'engaged' | 'contributor' | 'steward';

/**
 * Inline imagodei badge — avatar + name + standing ring, lensed through current Qahal.
 *
 * Used in stream items, member-ring drill-downs, and co-steward observations.
 * Substrate primitive for "who is this person in this Qahal."
 *
 * @element elohim-qahal-imagodei-badge
 *
 * @prop {string} name - The human's display name
 * @prop {string} avatarUrl - Optional avatar image URL; falls back to initials when absent
 * @prop {StandingTier} standingTier - Their standing in this Qahal's lens
 *
 * @cssprop --elohim-qahal-imagodei-badge-size - Override avatar size (default 1.5rem)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalImagodeiBadge extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
    }

    .avatar {
      width: var(--elohim-qahal-imagodei-badge-size, 1.5rem);
      height: var(--elohim-qahal-imagodei-badge-size, 1.5rem);
      border-radius: 50%;
      background: var(--elohim-color-surface-2, #eee);
      display: inline-flex;
      align-items: center;
      justify-content: center;
      font-size: 0.625rem;
      font-weight: 500;
      overflow: hidden;
      flex-shrink: 0;
    }

    .avatar img {
      width: 100%;
      height: 100%;
      border-radius: 50%;
      object-fit: cover;
    }

    .name {
      font-weight: 500;
    }

    .ring {
      inline-size: 0.5rem;
      block-size: 0.5rem;
      border-radius: 50%;
      flex-shrink: 0;
    }

    .ring[data-tier='visitor'] {
      background: var(--elohim-tier-visitor, #ccc);
    }

    .ring[data-tier='engaged'] {
      background: var(--elohim-tier-engaged, #99c);
    }

    .ring[data-tier='contributor'] {
      background: var(--elohim-tier-contributor, #6c9);
    }

    .ring[data-tier='steward'] {
      background: var(--elohim-tier-steward, #c96);
    }
  `;

  @property({ type: String })
  name = '';

  @property({ type: String, attribute: 'avatar-url' })
  avatarUrl = '';

  @property({ type: String, reflect: true, attribute: 'standing-tier' })
  standingTier: StandingTier = 'visitor';

  private get initials(): string {
    return this.name
      .split(/\s+/)
      .map(w => w[0] ?? '')
      .slice(0, 2)
      .join('')
      .toUpperCase();
  }

  override render() {
    return html`
      <span class="avatar" aria-hidden="true">
        ${this.avatarUrl
          ? html`
              <img src=${this.avatarUrl} alt="" />
            `
          : html`
              <span>${this.initials}</span>
            `}
      </span>
      <span class="name">${this.name}</span>
      <span
        class="ring"
        role="img"
        data-tier=${this.standingTier}
        aria-label="Standing tier: ${this.standingTier}"
      ></span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-imagodei-badge': ElohimQahalImagodeiBadge;
  }
}
