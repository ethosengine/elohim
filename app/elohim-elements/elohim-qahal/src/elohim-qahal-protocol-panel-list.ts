import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface ProtocolPanelDescriptor {
  id: string;
  label: string;
  active?: boolean;
}

/**
 * Protocol panel list — lists the protocol panels available in this Qahal.
 *
 * Clicking a panel item emits a `panel-changed` event with the panel id.
 * The active panel is marked with `aria-current="page"`.
 *
 * @element elohim-qahal-protocol-panel-list
 *
 * @prop {ProtocolPanelDescriptor[]} panels - Array of protocol panel descriptors
 *
 * @fires {CustomEvent<{id: string}>} panel-changed - When user clicks a panel item
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
export class ElohimQahalProtocolPanelList extends CapabilityAwareElement(LitElement) {
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

  private _handleClick(id: string) {
    this.dispatchEvent(
      new CustomEvent('panel-changed', { detail: { id }, bubbles: true, composed: true })
    );
  }

  override render() {
    return html`
      <nav aria-label="Protocol panels">
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
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-protocol-panel-list': ElohimQahalProtocolPanelList;
  }
}
