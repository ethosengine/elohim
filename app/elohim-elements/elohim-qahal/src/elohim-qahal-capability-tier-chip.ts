import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type CapabilityTier =
  | 'visitor'
  | 'engaged'
  | 'contributor'
  | 'steward'
  | 'elohim-support'
  | 'child'
  | 'idd_member'
  | 'elder_under_guardianship'
  | 'legal_steward_protected';

const PROTECTED = new Set<CapabilityTier>([
  'child',
  'idd_member',
  'elder_under_guardianship',
  'legal_steward_protected',
]);

/**
 * Capability tier chip — inline label for a tier with visual distinction for protected tiers.
 *
 * Used for capability-gating affordances and protected-tier marking on imagodei profiles.
 *
 * @element elohim-qahal-capability-tier-chip
 * @prop {CapabilityTier} tier - The tier to display
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
export class ElohimQahalCapabilityTierChip extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-block;
    }

    .chip {
      display: inline-block;
      padding-block: 0.125rem;
      padding-inline: 0.5rem;
      border-radius: 999px;
      font-size: 0.75rem;
      font-weight: 500;
      background: var(--elohim-color-surface-2, #eee);
      color: var(--elohim-color-fg-1, #222);
      white-space: nowrap;
    }

    :host([protected]) .chip {
      background: var(--elohim-color-protected-bg, #fef3c7);
      color: var(--elohim-color-protected-fg, #92400e);
      border: 1px solid var(--elohim-color-protected-border, #fbbf24);
    }
  `;

  @property({ type: String, reflect: true }) tier: CapabilityTier = 'visitor';

  override willUpdate(changed: Map<string, unknown>) {
    if (changed.has('tier')) {
      if (PROTECTED.has(this.tier)) {
        this.setAttribute('protected', '');
      } else {
        this.removeAttribute('protected');
      }
    }
  }

  private get label(): string {
    return this.tier.replace(/_/g, ' ');
  }

  override render() {
    return html`
      <span class="chip">${this.label}</span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-capability-tier-chip': ElohimQahalCapabilityTierChip;
  }
}
