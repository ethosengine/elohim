import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

import './elohim-qahal-capability-tier-chip.js';
import './elohim-qahal-standing-ring.js';
import type { BloomTier } from './elohim-qahal-standing-ring.js';

export interface StandingBreakdown {
  tier: string;
  bloomTier: string;
  attestations: number;
  affinity: number;
  debits: number;
}

/**
 * Standing inspector panel — viewer's standing breakdown in this Qahal.
 *
 * Visual stub at MVP. Renders the key standing dimensions as labelled rows
 * using Phase 1 primitives (capability-tier-chip, standing-ring). Full
 * attestation-chain walk + back-propagation traceback are deferred to
 * post-MVP (Sprint 6+).
 *
 * @element elohim-qahal-standing-inspector-panel
 *
 * @prop {StandingBreakdown} standing - The viewer's standing breakdown in this Qahal
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalStandingInspectorPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    h2 {
      font-size: 1rem;
      font-weight: 600;
      margin-block: 0 1rem;
      margin-inline: 0;
    }

    .breakdown-table {
      border-collapse: collapse;
      inline-size: 100%;
      margin-block-end: 1rem;
    }

    .breakdown-table th,
    .breakdown-table td {
      text-align: start;
      padding-block: 0.375rem;
      padding-inline: 0.5rem;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
    }

    .breakdown-table th {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--elohim-color-fg-2, #555);
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }

    .breakdown-table td {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-1, #222);
    }

    .breakdown-table tr:last-child td {
      border-block-end: none;
    }

    .stub-note {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
      border-block-start: 1px solid var(--elohim-color-border, #ddd);
      padding-block-start: 0.75rem;
      padding-block-end: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }
  `;

  @property({ type: Object }) standing: StandingBreakdown | null = null;

  override render() {
    if (!this.standing) {
      return html`
        <section aria-label="Standing Inspector">
          <h2>Standing Inspector</h2>
        </section>
      `;
    }

    const { tier, bloomTier, attestations, affinity, debits } = this.standing;

    return html`
      <section aria-label="Standing Inspector">
        <h2>Standing Inspector</h2>
        <table class="breakdown-table">
          <thead>
            <tr>
              <th scope="col">Dimension</th>
              <th scope="col">Value</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Tier</td>
              <td>
                <elohim-qahal-capability-tier-chip tier=${tier}></elohim-qahal-capability-tier-chip>
              </td>
            </tr>
            <tr>
              <td>Bloom tier</td>
              <td>
                <elohim-qahal-standing-ring
                  bloom-tier=${bloomTier as BloomTier}
                ></elohim-qahal-standing-ring>
                ${bloomTier}
              </td>
            </tr>
            <tr>
              <td>Attestations</td>
              <td>${attestations}</td>
            </tr>
            <tr>
              <td>Affinity score</td>
              <td>${affinity}</td>
            </tr>
            <tr>
              <td>Debits</td>
              <td>${debits}</td>
            </tr>
          </tbody>
        </table>
        <p class="stub-note">
          Visual stub at MVP. Full attestation-chain walk + back-propagation traceback is deferred
          to post-MVP (Sprint 6+).
        </p>
      </section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-standing-inspector-panel': ElohimQahalStandingInspectorPanel;
  }
}
