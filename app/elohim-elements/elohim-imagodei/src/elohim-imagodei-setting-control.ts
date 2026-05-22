import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface SettingControl {
  name: string;
  description: string;
  value: string;
  configurableBy: ('self' | 'steward' | 'system')[];
  source: 'self' | 'steward' | 'system';
}

/**
 * Setting control — individual setting row with name, value, source indicator, and edit affordance.
 *
 * Renders a single row of the settings palette: setting name + current value + source
 * indicator (self / steward / system) + an edit button gated by the viewerCanEdit capability flag.
 * Fires `edit-requested` when the user activates the edit affordance.
 *
 * @element elohim-imagodei-setting-control
 *
 * @prop {SettingControl} setting - The setting descriptor to render
 * @prop {boolean} viewerCanEdit - Whether the viewer has permission to edit this setting
 *
 * @fires {CustomEvent<{name: string}>} edit-requested - When the user requests to edit this setting
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimImagodeiSettingControl extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      padding-block: 0.75rem;
      padding-inline: 0;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
    }

    :host(:last-child) {
      border-block-end: none;
    }

    .row {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .meta {
      flex: 1;
      min-inline-size: 0;
    }

    .name {
      font-size: 0.875rem;
      font-weight: 600;
      color: var(--elohim-color-fg-1, #222);
    }

    .description {
      font-size: 0.75rem;
      color: var(--elohim-color-fg-2, #555);
      margin-block-start: 0.125rem;
    }

    .value-cell {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-1, #222);
      min-inline-size: 8rem;
      text-align: end;
    }

    .source-badge {
      display: inline-block;
      font-size: 0.7rem;
      font-weight: 500;
      padding-block: 0.1rem;
      padding-inline: 0.35rem;
      border-radius: 4px;
      background: var(--elohim-color-surface-2, #eee);
      color: var(--elohim-color-fg-2, #555);
      text-transform: capitalize;
    }

    .source-badge[data-source='steward'] {
      background: var(--elohim-color-protected-bg, #fef3c7);
      color: var(--elohim-color-protected-fg, #92400e);
    }

    .source-badge[data-source='system'] {
      background: var(--elohim-color-surface-3, #e0e0e0);
      color: var(--elohim-color-fg-3, #444);
    }

    .edit-btn {
      font-size: 0.8rem;
      padding-block: 0.3rem;
      padding-inline: 0.75rem;
      border: 1px solid var(--elohim-color-border, #ccc);
      background: var(--elohim-color-surface-0, #fff);
      border-radius: 4px;
      cursor: pointer;
      color: var(--elohim-color-fg-1, #222);
      white-space: nowrap;
    }

    .edit-btn:focus-visible {
      outline: 2px solid var(--elohim-color-focus, #6c9);
      outline-offset: 2px;
    }
  `;

  @property({ type: Object }) setting: SettingControl = {
    name: '',
    description: '',
    value: '',
    configurableBy: ['self'],
    source: 'self',
  };

  @property({ type: Boolean, attribute: 'viewer-can-edit' }) viewerCanEdit = false;

  private handleEdit() {
    this.dispatchEvent(
      new CustomEvent('edit-requested', {
        detail: { name: this.setting.name },
        bubbles: true,
        composed: true,
      })
    );
  }

  override render() {
    const { name, description, value, source } = this.setting;
    return html`
      <div class="row">
        <div class="meta">
          <div class="name">${name}</div>
          ${description
            ? html`
                <div class="description">${description}</div>
              `
            : ''}
        </div>
        <div class="value-cell">${value}</div>
        <span class="source-badge" data-source=${source}>${source}</span>
        ${this.viewerCanEdit
          ? html`
              <button class="edit-btn" aria-label="Edit ${name}" @click=${this.handleEdit}>
                Edit
              </button>
            `
          : ''}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-setting-control': ElohimImagodeiSettingControl;
  }
}
