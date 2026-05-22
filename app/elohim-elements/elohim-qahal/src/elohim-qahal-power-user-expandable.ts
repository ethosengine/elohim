import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, nothing } from 'lit';
import { property } from 'lit/decorators.js';

import type { ProtocolPanelDescriptor } from './elohim-qahal-protocol-panel-list.js';

export type { ProtocolPanelDescriptor } from './elohim-qahal-protocol-panel-list.js';

/**
 * Power-user expandable — renders the protocol panels list only when the imagodei
 * power-user-enabled setting is active.
 *
 * The element honors the `powerUserEnabled` setting silently — no toggle is exposed
 * on this element itself. The Qahal sidebar is responsible for sourcing the setting
 * from the imagodei settings palette and passing it here.
 *
 * When `powerUserEnabled` is false the element renders nothing (an empty nav with no
 * visible content).
 *
 * @element elohim-qahal-power-user-expandable
 *
 * @prop {ProtocolPanelDescriptor[]} panels - Array of protocol panel descriptors
 * @prop {boolean} powerUserEnabled - Sourced from imagodei settings palette; when false renders nothing
 *
 * @fires {CustomEvent<{id: string}>} panel-changed - When user clicks a panel item (only fires when enabled)
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
export class ElohimQahalPowerUserExpandable extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    nav {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    button {
      display: block;
      inline-size: 100%;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
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

    button[aria-current='page'] {
      background: var(--elohim-color-surface-2, #ebebe8);
      font-weight: 600;
    }

    button:focus-visible {
      outline: 2px solid var(--elohim-color-focus, #6c9);
      outline-offset: 2px;
    }
  `;

  @property({ type: Array }) panels: ProtocolPanelDescriptor[] = [];
  @property({ type: Boolean, attribute: 'power-user-enabled' }) powerUserEnabled = false;

  private _handleClick(id: string) {
    this.dispatchEvent(
      new CustomEvent('panel-changed', { detail: { id }, bubbles: true, composed: true })
    );
  }

  override render() {
    if (!this.powerUserEnabled) {
      return html`
        <nav aria-label="Power-user panels"></nav>
      `;
    }
    return html`
      <nav aria-label="Power-user panels">
        ${this.panels.map(
          p => html`
            <button
              aria-label=${p.label}
              aria-current=${p.active ? 'page' : 'false'}
              @click=${() => this._handleClick(p.id)}
            >
              ${p.label}
            </button>
          `
        )}
      </nav>
      ${nothing}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-power-user-expandable': ElohimQahalPowerUserExpandable;
  }
}
