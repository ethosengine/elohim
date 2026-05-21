import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type BloomTier = 'remember' | 'understand' | 'apply' | 'analyze' | 'evaluate' | 'create';

const TIERS: BloomTier[] = ['remember', 'understand', 'apply', 'analyze', 'evaluate', 'create'];

/**
 * Standing ring — Bloom-tier dots indicating capability tier at a glance.
 *
 * Renders six dots (●●●○○○-style) showing the cumulative tier at the current Bloom level.
 * Used in member-ring drill-downs and standing-inspector panels.
 *
 * @element elohim-qahal-standing-ring
 *
 * @prop {BloomTier} bloomTier - The tier to render (remember | understand | apply | analyze | evaluate | create)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalStandingRing extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-block;
    }

    [role='img'] {
      font-family: monospace;
      letter-spacing: -0.1em;
      color: var(--elohim-color-fg-2, #555);
    }
  `;

  @property({ type: String, reflect: true, attribute: 'bloom-tier' })
  bloomTier: BloomTier = 'remember';

  override render() {
    const idx = TIERS.indexOf(this.bloomTier) + 1;
    const filled = '●'.repeat(idx);
    const empty = '○'.repeat(6 - idx);
    return html`
      <span role="img" aria-label="Bloom tier: ${this.bloomTier} (${idx} of 6)">
        ${filled}${empty}
      </span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-standing-ring': ElohimQahalStandingRing;
  }
}
