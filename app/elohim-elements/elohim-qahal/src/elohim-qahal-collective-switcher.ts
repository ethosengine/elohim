import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface CollectiveDescriptor {
  id: string;
  icon: string;
  name: string;
}

/**
 * Collective switcher — far-left column listing Qahals the operator participates in.
 *
 * Used in the Qahal homepage 4-column chrome as the leftmost icon strip.
 * Each button represents one Qahal; clicking one emits `collective-changed`.
 *
 * @element elohim-qahal-collective-switcher
 *
 * @prop {CollectiveDescriptor[]} collectives - Array of Qahal descriptors
 * @prop {string} activeCollectiveId - The currently-active collective's id
 *
 * @fires {CustomEvent<{id: string}>} collective-changed - When user clicks a different collective
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:n/a, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalCollectiveSwitcher extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      padding-block: 0.5rem;
      background: var(--elohim-color-surface-1, #f7f7f5);
      border-inline-end: 1px solid var(--elohim-color-border, #ddd);
      block-size: 100%;
    }

    nav {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    button {
      inline-size: 3rem;
      block-size: 3rem;
      border: 0;
      background: transparent;
      cursor: pointer;
      font-size: 1.5rem;
      border-radius: 0.5rem;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    button[aria-pressed='true'] {
      background: var(--elohim-color-surface-2, #ebebe8);
    }

    button:focus-visible {
      outline: 2px solid var(--elohim-color-focus, #6c9);
      outline-offset: 2px;
    }
  `;

  @property({ type: Array }) collectives: CollectiveDescriptor[] = [];

  @property({ type: String, attribute: 'active-collective-id' }) activeCollectiveId = '';

  private handleClick(id: string) {
    this.dispatchEvent(
      new CustomEvent('collective-changed', { detail: { id }, bubbles: true, composed: true })
    );
  }

  override render() {
    return html`
      <nav aria-label="Collective switcher">
        ${this.collectives.map(
          c => html`
            <button
              aria-label=${c.name}
              aria-pressed=${c.id === this.activeCollectiveId ? 'true' : 'false'}
              @click=${() => this.handleClick(c.id)}
            >
              ${c.icon}
            </button>
          `
        )}
      </nav>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-collective-switcher': ElohimQahalCollectiveSwitcher;
  }
}
