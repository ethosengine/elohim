import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

import './elohim-imagodei-protected-tier-marker.js';

import type { ProtectedTier } from './elohim-imagodei-protected-tier-marker.js';

export interface AffordanceTrace {
  name: string;
  visible: boolean;
  whyTrace: string;
}

export interface SubjectSetting {
  name: string;
  value: string;
  source: 'self' | 'steward' | 'system';
}

/**
 * Introspection panel — developer/support-agent surface for settings state + trace.
 *
 * Exposes the full settings state and affordance trace for a given subject.
 * Visual stub at MVP — the full introspection-trace primitive is deferred to a
 * post-MVP substrate-spine design pass.
 *
 * @element elohim-imagodei-introspection-panel
 *
 * @prop {string} subjectId - The human whose view is being introspected
 * @prop {string} subjectTier - The capability tier of the subject
 * @prop {ProtectedTier} [subjectProtectedTier] - If the subject is in a protected tier
 * @prop {AffordanceTrace[]} sampleAffordances - Affordances with visibility + trace explanations
 * @prop {SubjectSetting[]} subjectSettings - Current settings values + source
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:supported
 */
export class ElohimImagodeiIntrospectionPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      font-size: 0.875rem;
      color: var(--elohim-color-fg-1, #222);
    }

    h2 {
      font-size: 1rem;
      font-weight: 700;
      margin-block: 0 0.5rem;
      margin-inline: 0;
    }

    .subject-line {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #555);
      margin-block-end: 0.5rem;
    }

    .tier-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-block-end: 1rem;
    }

    section {
      margin-block-end: 1.5rem;
    }

    h3 {
      font-size: 0.875rem;
      font-weight: 600;
      margin-block: 0 0.5rem;
      margin-inline: 0;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
      padding-block-end: 0.25rem;
    }

    table {
      inline-size: 100%;
      border-collapse: collapse;
      font-size: 0.8rem;
    }

    th {
      text-align: start;
      padding-block: 0.3rem;
      padding-inline: 0.5rem;
      font-weight: 600;
      background: var(--elohim-color-surface-1, #f7f7f5);
      border: 1px solid var(--elohim-color-border, #ddd);
    }

    td {
      padding-block: 0.3rem;
      padding-inline: 0.5rem;
      border: 1px solid var(--elohim-color-border, #ddd);
      vertical-align: top;
    }

    .visible-yes {
      color: var(--elohim-color-healthy, #2a7a2a);
      font-weight: 500;
    }

    .visible-no {
      color: var(--elohim-color-fg-2, #555);
    }

    .stub-note {
      font-size: 0.75rem;
      font-style: italic;
      color: var(--elohim-color-fg-2, #555);
      background: var(--elohim-color-surface-1, #f7f7f5);
      border: 1px dashed var(--elohim-color-border, #ccc);
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border-radius: 4px;
      margin-block-start: 1rem;
    }
  `;

  @property({ type: String, attribute: 'subject-id' }) subjectId = '';
  @property({ type: String, attribute: 'subject-tier' }) subjectTier = '';
  @property({ type: String, attribute: 'subject-protected-tier' })
  subjectProtectedTier: ProtectedTier | '' = '';
  @property({ type: Array }) sampleAffordances: AffordanceTrace[] = [];
  @property({ type: Array }) subjectSettings: SubjectSetting[] = [];

  override render() {
    return html`
      <h2>Introspection · ${this.subjectId}</h2>

      <div class="tier-row">
        <span class="subject-line">Current capability tier: ${this.subjectTier}</span>
        ${this.subjectProtectedTier
          ? html`
              <elohim-imagodei-protected-tier-marker
                tier=${this.subjectProtectedTier}
              ></elohim-imagodei-protected-tier-marker>
            `
          : ''}
      </div>

      <section>
        <h3>Settings palette state</h3>
        ${this.subjectSettings.length === 0
          ? html`
              <p class="subject-line">No settings to display.</p>
            `
          : html`
              <table aria-label="Settings palette state for ${this.subjectId}">
                <thead>
                  <tr>
                    <th scope="col">Setting</th>
                    <th scope="col">Value</th>
                    <th scope="col">Source</th>
                  </tr>
                </thead>
                <tbody>
                  ${this.subjectSettings.map(
                    s => html`
                      <tr>
                        <td>${s.name}</td>
                        <td>${s.value}</td>
                        <td>${s.source}</td>
                      </tr>
                    `
                  )}
                </tbody>
              </table>
            `}
      </section>

      <section>
        <h3>Affordance trace</h3>
        ${this.sampleAffordances.length === 0
          ? html`
              <p class="subject-line">No affordances to display.</p>
            `
          : html`
              <table aria-label="Affordance trace for ${this.subjectId}">
                <thead>
                  <tr>
                    <th scope="col">Affordance</th>
                    <th scope="col">Visible</th>
                    <th scope="col">Why</th>
                  </tr>
                </thead>
                <tbody>
                  ${this.sampleAffordances.map(
                    a => html`
                      <tr>
                        <td>${a.name}</td>
                        <td>
                          <span class=${a.visible ? 'visible-yes' : 'visible-no'}>
                            ${a.visible ? 'visible' : 'hidden'}
                          </span>
                        </td>
                        <td>${a.whyTrace}</td>
                      </tr>
                    `
                  )}
                </tbody>
              </table>
            `}
      </section>

      <p class="stub-note">
        Visual stub at MVP. Full introspection trace primitive is deferred to a post-MVP
        substrate-spine design pass (no new storage entities introduced).
      </p>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-introspection-panel': ElohimImagodeiIntrospectionPanel;
  }
}
