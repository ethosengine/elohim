import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

import './elohim-qahal-provenance-marker.js';

export interface CuratedEpr {
  id: string;
  label: string;
  iconPath?: string;
}

/**
 * Curated EPR list — lists curated EPR pointers with ◆ provenance markers.
 *
 * Each item composes an `<elohim-qahal-provenance-marker category="curated-epr">` for the ◆ icon
 * next to the label. Clicking an item emits `epr-selected` with the EPR id.
 *
 * @element elohim-qahal-curated-epr-list
 *
 * @prop {CuratedEpr[]} eprs - Array of curated EPR descriptors
 *
 * @fires {CustomEvent<{id: string}>} epr-selected - When user clicks an EPR item
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalCuratedEprList extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    ul {
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }

    li {
      padding-block: 0.25rem;
      padding-inline: 0;
    }

    button {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      inline-size: 100%;
      padding-block: 0.375rem;
      padding-inline: 0.5rem;
      border: 0;
      background: transparent;
      cursor: pointer;
      font-size: 0.875rem;
      text-align: start;
      border-radius: 0.375rem;
      color: var(--elohim-color-fg-1, #222);
    }

    button:hover {
      background: var(--elohim-color-surface-2, #ebebe8);
    }

    button:focus-visible {
      outline: 2px solid var(--elohim-color-focus, #6c9);
      outline-offset: 2px;
    }

    .epr-label {
      flex: 1 1 auto;
    }

    .empty-note {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
    }
  `;

  @property({ type: Array }) eprs: CuratedEpr[] = [];

  private _handleClick(id: string) {
    this.dispatchEvent(
      new CustomEvent('epr-selected', { detail: { id }, bubbles: true, composed: true })
    );
  }

  override render() {
    if (this.eprs.length === 0) {
      return html`
        <p class="empty-note">No curated EPRs pinned.</p>
      `;
    }
    return html`
      <ul>
        ${this.eprs.map(
          epr => html`
            <li>
              <button aria-label=${epr.label} @click=${() => this._handleClick(epr.id)}>
                <elohim-qahal-provenance-marker
                  category="curated-epr"
                ></elohim-qahal-provenance-marker>
                <span class="epr-label">${epr.label}</span>
              </button>
            </li>
          `
        )}
      </ul>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-curated-epr-list': ElohimQahalCuratedEprList;
  }
}
