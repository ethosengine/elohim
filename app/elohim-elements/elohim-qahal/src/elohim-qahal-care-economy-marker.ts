import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type CareEconomyKind = 'care' | 'presence' | 'repair' | 'growth' | 'time';

const ICONS: Record<CareEconomyKind, string> = {
  care: '✨',
  presence: '👁',
  repair: '🛠',
  growth: '🌱',
  time: '⏰',
};

/**
 * Care-economy marker — small inline REA event indicator.
 *
 * Used inline within stream items to surface care-economy contributions
 * (+care, +presence, +repair, +growth, +time).
 *
 * @element elohim-qahal-care-economy-marker
 * @prop {CareEconomyKind} kind - Which kind of care-economy contribution
 * @prop {number} tokens - Count of tokens (default 1)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalCareEconomyMarker extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      font-size: 0.75rem;
      color: var(--elohim-color-fg-2, #555);
      white-space: nowrap;
    }
  `;

  @property({ type: String, reflect: true }) kind: CareEconomyKind = 'care';
  @property({ type: Number }) tokens = 1;

  override render() {
    const icon = (ICONS as Record<string, string>)[this.kind] ?? '';
    return html`
      <span aria-label="${this.kind} contribution, ${this.tokens} tokens">
        ${icon} +${this.tokens} ${this.kind}
      </span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-care-economy-marker': ElohimQahalCareEconomyMarker;
  }
}
