import { CapabilityAwareElement } from 'elohim-core';
import { css, html, type TemplateResult, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

import './elohim-imagodei-protected-tier-marker.js';
import './elohim-imagodei-setting-control.js';
import './elohim-imagodei-steward-configure-banner.js';

import type { ProtectedTier } from './elohim-imagodei-protected-tier-marker.js';
import type { SettingControl } from './elohim-imagodei-setting-control.js';

// 13 settings from UX spec §8.2.
// Placeholder values and source indicators — actual backends are post-MVP.
const SETTINGS_CATALOG: SettingControl[] = [
  {
    name: 'Power-user view',
    description: 'Visual density toggle for advanced controls',
    value: 'simple',
    configurableBy: ['self'],
    source: 'self',
  },
  {
    name: 'External link visibility',
    description: 'Controls which external links are shown',
    value: 'full',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Notification volume/style',
    description: 'How and how often you receive notifications',
    value: 'gentle',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Content reach gating',
    description: 'Bounds for incoming and outgoing content reach',
    value: 'qahal-bounded',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Standing visibility',
    description: 'Who can see your standing score',
    value: 'visible',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Co-steward voice register',
    description: 'How your co-steward speaks on your behalf',
    value: 'observational',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Recovery authority delegation',
    description: 'Who can help you recover account access',
    value: 'self',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Compute-stewardship visibility',
    description: 'Whether your compute contribution is visible to others',
    value: 'visible',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Data export visibility',
    description: 'Who can initiate a data export on your behalf',
    value: 'stewarded',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Language / accessibility',
    description: 'Language preference and accessibility overrides',
    value: 'system',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Web2.0 link click confirmation',
    description: 'Confirmation step before following external links',
    value: 'one-tap-warning',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
  {
    name: 'Imagodei lens defaults',
    description: 'Default lens used when viewing your own imagodei',
    value: 'most-recent-shared-Qahal',
    configurableBy: ['self'],
    source: 'self',
  },
  {
    name: 'Onboarding pace',
    description: 'How the protocol introduces new features to you',
    value: 'natural',
    configurableBy: ['self', 'steward'],
    source: 'self',
  },
];

/**
 * Settings palette — full composite settings surface for imagodei configuration.
 *
 * Renders the 13 settings from UX spec §8.2 as a list of setting-control rows.
 * When the viewer is configuring a stewardee's settings (viewerId !== targetId),
 * renders the steward-configure-banner at the top. When the target is in a
 * protected tier, renders the protected-tier-marker alongside the banner.
 *
 * Setting values are MVP placeholders — actual setting backends are post-MVP.
 *
 * @element elohim-imagodei-settings-palette
 *
 * @prop {string} viewerId - The ID of the viewer (the person making changes)
 * @prop {string} targetId - The ID of the imagodei being configured (equals viewerId for self-configuration)
 * @prop {ProtectedTier} [targetTier] - If the target is in a protected tier
 * @prop {string} [witnessQahalName] - Required when viewer !== target; the Qahal whose commons-elohim will co-sign
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
export class ElohimImagodeiSettingsPalette extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    .palette-header {
      margin-block-end: 1.25rem;
    }

    h2 {
      font-size: 1rem;
      font-weight: 700;
      margin-block: 0 0.75rem;
      margin-inline: 0;
      color: var(--elohim-color-fg-1, #222);
    }

    .tier-row {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      margin-block-end: 1rem;
    }

    .settings-list {
      display: block;
    }
  `;

  @property({ type: String, attribute: 'viewer-id' }) viewerId = '';
  @property({ type: String, attribute: 'target-id' }) targetId = '';
  @property({ type: String, attribute: 'target-tier' }) targetTier: ProtectedTier | '' = '';
  @property({ type: String, attribute: 'witness-qahal-name' }) witnessQahalName = '';

  private get isStewardMode(): boolean {
    return this.viewerId !== this.targetId && this.targetId !== '';
  }

  private get viewerCanEdit(): boolean {
    // In self-mode the viewer can always edit. In steward mode the steward can edit.
    return this.viewerId !== '' && (this.viewerId === this.targetId || this.isStewardMode);
  }

  private _renderProtectedTierMarker(): TemplateResult {
    if (!this.targetTier) return html``;
    return html`
      <elohim-imagodei-protected-tier-marker
        tier=${this.targetTier}
      ></elohim-imagodei-protected-tier-marker>
    `;
  }

  override render() {
    return html`
      <div class="palette-header">
        <h2>Settings</h2>
        ${this.isStewardMode
          ? html`
              <div class="tier-row">
                <elohim-imagodei-steward-configure-banner
                  steward-name=${this.viewerId}
                  stewardee-name=${this.targetId}
                  witness-qahal-name=${this.witnessQahalName}
                ></elohim-imagodei-steward-configure-banner>
                ${this._renderProtectedTierMarker()}
              </div>
            `
          : ''}
      </div>

      <div class="settings-list">
        ${SETTINGS_CATALOG.map(
          setting => html`
            <elohim-imagodei-setting-control
              .setting=${setting}
              ?viewer-can-edit=${this.viewerCanEdit}
            ></elohim-imagodei-setting-control>
          `
        )}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-settings-palette': ElohimImagodeiSettingsPalette;
  }
}
