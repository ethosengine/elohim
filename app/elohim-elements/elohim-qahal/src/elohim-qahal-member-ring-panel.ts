import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

import './elohim-qahal-imagodei-badge.js';

export interface MemberEntry {
  id: string;
  name: string;
}

export interface MemberTier {
  id: string;
  label: string;
  count: number;
  members: MemberEntry[];
}

const TIER_NOTES: Record<string, string> = {
  'contributor-presences':
    'non-protocol participants whose recognition accrues to the Qahal commons; in trust until direct participation resolves it',
  'compute-hosting-stewards':
    'lending stewarded compute allocation for resilience, edge distribution, discovery',
};

const MAX_BADGES = 8;

/**
 * Member-ring panel — tier-aware member drill-down with network reach headline.
 *
 * Renders the stratified member view: a network reach headline followed by
 * four tier sections, each showing a label, count, imagodei badges (up to 8),
 * and a "+ N more" indicator for overflow. Selected tiers include contextual notes.
 *
 * @element elohim-qahal-member-ring-panel
 *
 * @prop {number} reach - Total network reach count
 * @prop {MemberTier[]} tiers - Array of member tier sections to render
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalMemberRingPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    .reach-headline {
      margin-block-end: 1.5rem;
    }

    .reach-label {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #666);
      text-transform: uppercase;
      letter-spacing: 0.04em;
      display: block;
    }

    .reach-count {
      font-size: 2rem;
      font-weight: 700;
      line-height: 1.1;
      color: var(--elohim-color-fg-1, #222);
    }

    .tier-section {
      margin-block-end: 1.25rem;
    }

    .tier-header {
      display: flex;
      align-items: baseline;
      gap: 0.5rem;
      margin-block-end: 0.5rem;
    }

    .tier-label {
      font-size: 0.875rem;
      font-weight: 600;
      color: var(--elohim-color-fg-1, #222);
    }

    .tier-count {
      font-size: 0.75rem;
      color: var(--elohim-color-fg-2, #666);
    }

    .tier-note {
      font-size: 0.75rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
      margin-block: 0.25rem 0.5rem;
      margin-inline: 0;
    }

    .tier-members {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
      align-items: center;
    }

    .tier-overflow {
      font-size: 0.75rem;
      color: var(--elohim-color-fg-2, #666);
    }
  `;

  @property({ type: Number }) reach = 0;
  @property({ type: Array }) tiers: MemberTier[] = [];

  private _renderTier(tier: MemberTier) {
    const note = TIER_NOTES[tier.id];
    const shown = tier.members.slice(0, MAX_BADGES);
    const overflow = tier.count - shown.length;
    return html`
      <section class="tier-section" data-tier-id=${tier.id}>
        <div class="tier-header">
          <span class="tier-label">${tier.label}</span>
          <span class="tier-count">${tier.count}</span>
        </div>
        ${note
          ? html`
              <p class="tier-note">${note}</p>
            `
          : ''}
        <div class="tier-members">
          ${shown.map(
            m => html`
              <elohim-qahal-imagodei-badge
                name=${m.name}
                standing-tier="visitor"
              ></elohim-qahal-imagodei-badge>
            `
          )}
          ${overflow > 0
            ? html`
                <span class="tier-overflow">+ ${overflow} more</span>
              `
            : ''}
        </div>
      </section>
    `;
  }

  override render() {
    return html`
      <div>
        <div class="reach-headline" aria-label="Network reach: ${this.reach}">
          <span class="reach-label">Network reach</span>
          <span class="reach-count">${this.reach}</span>
        </div>
        ${this.tiers.map(t => this._renderTier(t))}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-member-ring-panel': ElohimQahalMemberRingPanel;
  }
}
