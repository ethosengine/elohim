import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface ComputeHubStatus {
  hubId: string;
  status: 'healthy' | 'degraded' | 'down';
  allocatedGB: number;
  lastSeen: string;
}

export interface ComputeStewardRelationship {
  stewardId: string;
  stewardName: string;
  health: 'healthy' | 'degraded' | 'down';
  percent: number;
  lastSync: string;
  replicating: string;
}

export interface ComputeTopology {
  qahalId: string;
  selfHub: ComputeHubStatus;
  stewardsForUs: ComputeStewardRelationship[];
  weStewardFor: { stewardId: string; description: string }[];
  recoveryReadiness: {
    ready: boolean;
    shamirThreshold: string;
    lastDrill: string;
    nextDrill: string;
  };
}

const HEALTH_ICON: Record<ComputeHubStatus['status'], string> = {
  healthy: '●',
  degraded: '◐',
  down: '○',
};

/**
 * Social-compute topology panel — renders the Qahal's resilience topology.
 *
 * Displays the shefa↔qahal intersection: the self hub status, compute-stewards
 * providing resilience for this Qahal (with health markers and reciprocal
 * relationships), and recovery readiness with Shamir threshold and drill cadence.
 *
 * Accepts any object matching the ComputeTopology shape — real or mock. This
 * keeps the panel decoupled from Phase 2 fixture types while remaining
 * wire-compatible with MockComputeTopology.
 *
 * @element elohim-qahal-social-compute-panel
 *
 * @prop {ComputeTopology} topology - The compute topology data to render
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalSocialComputePanel extends CapabilityAwareElement(LitElement) {
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

    h3 {
      font-size: 0.875rem;
      font-weight: 600;
      margin-block: 0 0.5rem;
      margin-inline: 0;
      color: var(--elohim-color-fg-1, #222);
    }

    section {
      margin-block-end: 1.25rem;
    }

    .hub-status {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-1, #222);
    }

    .health {
      font-size: 0.875rem;
    }

    .health[data-health='healthy'] {
      color: var(--elohim-color-healthy, #2a7a2a);
    }

    .health[data-health='degraded'] {
      color: var(--elohim-color-degraded, #a07000);
    }

    .health[data-health='down'] {
      color: var(--elohim-color-down, #c0392b);
    }

    .active-ratio {
      font-size: 0.75rem;
      font-weight: 400;
      color: var(--elohim-color-fg-2, #555);
      margin-inline-start: 0.5rem;
    }

    .steward-list,
    .reciprocal-list {
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }

    .steward-item {
      padding-block: 0.5rem;
      padding-inline: 0;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
    }

    .steward-item:last-child {
      border-block-end: none;
    }

    .steward-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-block-end: 0.25rem;
    }

    .health-marker {
      font-size: 0.875rem;
    }

    .health-marker[data-health='healthy'] {
      color: var(--elohim-color-healthy, #2a7a2a);
    }

    .health-marker[data-health='degraded'] {
      color: var(--elohim-color-degraded, #a07000);
    }

    .health-marker[data-health='down'] {
      color: var(--elohim-color-down, #c0392b);
    }

    .steward-name {
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--elohim-color-fg-1, #222);
    }

    .steward-meta {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #555);
      margin-inline-start: auto;
    }

    .replicating {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #555);
      padding-inline-start: 1.5rem;
    }

    .reciprocal-item {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-1, #222);
      padding-block: 0.35rem;
      padding-inline: 0;
    }

    .note {
      font-size: 0.75rem;
      font-weight: 400;
      color: var(--elohim-color-fg-2, #555);
      margin-inline-start: 0.5rem;
    }

    .recovery {
      border-block-start: 1px solid var(--elohim-color-border, #ddd);
      padding-block-start: 0.75rem;
    }

    .readiness {
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--elohim-color-fg-1, #222);
      margin-block-end: 0.5rem;
    }

    .readiness-ready {
      color: var(--elohim-color-healthy, #2a7a2a);
    }

    .readiness-not-ready {
      color: var(--elohim-color-degraded, #a07000);
    }

    .drill-list {
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }

    .drill-list li {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #555);
      padding-block: 0.2rem;
      padding-inline: 0;
    }

    .drill-list li::before {
      content: '· ';
    }
  `;

  @property({ type: Object }) topology: ComputeTopology | null = null;

  private _renderSteward(s: ComputeStewardRelationship) {
    const icon = HEALTH_ICON[s.health];
    return html`
      <li class="steward-item">
        <div class="steward-row">
          <span class="health-marker" data-health=${s.health} aria-label=${s.health}>${icon}</span>
          <span class="steward-name">${s.stewardName}</span>
          <span class="steward-meta">${s.percent}% · last sync: ${s.lastSync}</span>
        </div>
        <div class="replicating">replicating: ${s.replicating}</div>
      </li>
    `;
  }

  override render() {
    if (!this.topology) {
      return html`
        <section aria-label="Social-Compute Topology"><h2>Social-Compute Topology</h2></section>
      `;
    }

    const { qahalId, selfHub, stewardsForUs, weStewardFor, recoveryReadiness } = this.topology;
    const selfIcon = HEALTH_ICON[selfHub.status];

    const activeCount = stewardsForUs.filter(s => s.health === 'healthy').length;
    const totalCount = stewardsForUs.length;
    const allActive = totalCount > 0 && activeCount === totalCount;
    const activeRatio = `(${activeCount}-of-${totalCount} active${allActive ? ' ✓' : ''})`;

    const readyLabel = recoveryReadiness.ready ? '✓ ready' : '⚠ not ready';
    const readyClass = recoveryReadiness.ready ? 'readiness-ready' : 'readiness-not-ready';

    return html`
      <section aria-label="Social-Compute Topology">
        <h2>Social-Compute Topology</h2>

        <section>
          <h3>Our hub · ${qahalId}</h3>
          <div class="hub-status">
            status:
            <span class="health" data-health=${selfHub.status} aria-label=${selfHub.status}>
              ${selfIcon}
            </span>
            ${selfHub.status} · ${selfHub.allocatedGB} GB allocated · last-seen: ${selfHub.lastSeen}
          </div>
        </section>

        <section>
          <h3>
            Compute-stewards FOR us
            <span class="active-ratio">${activeRatio}</span>
          </h3>
          ${stewardsForUs.length === 0
            ? ''
            : html`
                <ul class="steward-list">
                  ${stewardsForUs.map(s => this._renderSteward(s))}
                </ul>
              `}
        </section>

        <section>
          <h3>
            We compute-steward FOR
            <span class="note">(reciprocal trust)</span>
          </h3>
          ${weStewardFor.length === 0
            ? ''
            : html`
                <ul class="reciprocal-list">
                  ${weStewardFor.map(
                    w => html`
                      <li class="reciprocal-item">${w.stewardId} → ${w.description}</li>
                    `
                  )}
                </ul>
              `}
        </section>

        <section class="recovery">
          <div class="readiness">
            Recovery readiness:
            <span class=${readyClass}>${readyLabel}</span>
            · Shamir threshold: ${recoveryReadiness.shamirThreshold}
          </div>
          <ul class="drill-list">
            <li>Last recovery drill: ${recoveryReadiness.lastDrill}</li>
            <li>Next scheduled drill: ${recoveryReadiness.nextDrill}</li>
          </ul>
        </section>
      </section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-social-compute-panel': ElohimQahalSocialComputePanel;
  }
}
