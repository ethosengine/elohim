import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type ProtectedTier =
  | 'child'
  | 'idd_member'
  | 'elder_under_guardianship'
  | 'legal_steward_protected';

const KNOWN_TIERS = new Set<string>([
  'child',
  'idd_member',
  'elder_under_guardianship',
  'legal_steward_protected',
]);

/**
 * Protected-tier marker — compact amber pill badge for imagodei protected tiers.
 *
 * Displays which protected tier a user has (child, idd_member, elder_under_guardianship,
 * legal_steward_protected). Visually distinct from the general capability-tier-chip —
 * uses amber/gold styling and a shield prefix to clearly mark this as a protected-tier
 * indicator rather than a general tier indicator.
 *
 * @element elohim-imagodei-protected-tier-marker
 *
 * @prop {ProtectedTier} tier - The protected tier to display
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
export class ElohimImagodeiProtectedTierMarker extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-block;
    }

    .marker {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding-block: 0.2rem;
      padding-inline: 0.6rem;
      border-radius: 999px;
      font-size: 0.75rem;
      font-weight: 600;
      background: var(--elohim-color-protected-bg, #fef3c7);
      color: var(--elohim-color-protected-fg, #92400e);
      border: 1.5px solid var(--elohim-color-protected-border, #f59e0b);
      white-space: nowrap;
    }

    .shield {
      font-size: 0.875rem;
      line-height: 1;
    }

    .unknown {
      background: var(--elohim-color-surface-2, #eee);
      color: var(--elohim-color-fg-2, #555);
      border-color: var(--elohim-color-border, #ccc);
    }
  `;

  @property({ type: String, reflect: true }) tier: ProtectedTier = 'child';

  private get label(): string {
    return this.tier.replace(/_/g, ' ');
  }

  private get isKnown(): boolean {
    return KNOWN_TIERS.has(this.tier);
  }

  override render() {
    return html`
      <span
        class="marker${this.isKnown ? '' : ' unknown'}"
        role="status"
        aria-label="Protected tier: ${this.label}"
      >
        ${this.isKnown
          ? html`
              <span class="shield" aria-hidden="true">🛡</span>
            `
          : ''}
        ${this.label}
      </span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-protected-tier-marker': ElohimImagodeiProtectedTierMarker;
  }
}
