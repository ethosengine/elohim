import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiSettingsPalette as ElohimImagodeiSettingsPaletteClass } from './elohim-imagodei-settings-palette.js';
import type { ElohimImagodeiSettingsPalette } from './elohim-imagodei-settings-palette.js';

// The 13 setting names from UX spec §8.2
const SETTING_NAMES = [
  'Power-user view',
  'External link visibility',
  'Notification volume/style',
  'Content reach gating',
  'Standing visibility',
  'Co-steward voice register',
  'Recovery authority delegation',
  'Compute-stewardship visibility',
  'Data export visibility',
  'Language / accessibility',
  'Web2.0 link click confirmation',
  'Imagodei lens defaults',
  'Onboarding pace',
];

describe('<elohim-imagodei-settings-palette>', () => {
  it('renders all 13 settings names in self-configuration mode', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="user-alice"
        target-id="user-alice"
      ></elohim-imagodei-settings-palette>
    `);
    // Each setting-control is a custom element — check its shadow DOM for the name
    const controls = el.shadowRoot!.querySelectorAll('elohim-imagodei-setting-control');
    expect(controls).to.have.lengthOf(13);
    const allText = Array.from(controls)
      .map(ctrl => ctrl.shadowRoot?.textContent ?? '')
      .join(' ');
    for (const name of SETTING_NAMES) {
      expect(allText, `Expected "${name}" to be present`).to.include(name);
    }
  });

  it('renders all 13 settings in steward-configuration mode', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="steward-guardian"
        target-id="user-alice"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-settings-palette>
    `);
    // Dive into shadow DOM and inner custom elements for full text
    const settingControls = el.shadowRoot!.querySelectorAll('elohim-imagodei-setting-control');
    expect(settingControls).to.have.lengthOf(13);
  });

  it('does NOT show the steward banner in self-configuration mode', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="user-alice"
        target-id="user-alice"
      ></elohim-imagodei-settings-palette>
    `);
    const banner = el.shadowRoot!.querySelector('elohim-imagodei-steward-configure-banner');
    expect(banner).to.be.null;
  });

  it('shows the steward banner when viewer !== target', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="steward-guardian"
        target-id="user-alice"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-settings-palette>
    `);
    const banner = el.shadowRoot!.querySelector('elohim-imagodei-steward-configure-banner');
    expect(banner).to.exist;
  });

  it('does NOT show the protected-tier marker when targetTier is not set', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="steward-guardian"
        target-id="user-alice"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-settings-palette>
    `);
    const marker = el.shadowRoot!.querySelector('elohim-imagodei-protected-tier-marker');
    expect(marker).to.be.null;
  });

  it('shows the protected-tier marker when targetTier is set', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="steward-guardian"
        target-id="user-alice"
        target-tier="child"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-settings-palette>
    `);
    const marker = el.shadowRoot!.querySelector('elohim-imagodei-protected-tier-marker');
    expect(marker).to.exist;
  });

  it('dynamic transition: banner appears when targetId changes from self to other', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="user-alice"
        target-id="user-alice"
      ></elohim-imagodei-settings-palette>
    `);
    expect(el.shadowRoot!.querySelector('elohim-imagodei-steward-configure-banner')).to.be.null;

    el.targetId = 'user-bob';
    el.witnessQahalName = 'Dawn Runners';
    await elementUpdated(el);

    const banner = el.shadowRoot!.querySelector('elohim-imagodei-steward-configure-banner');
    expect(banner).to.exist;
  });

  it('passes axe accessibility audit in self-configuration mode', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="user-alice"
        target-id="user-alice"
      ></elohim-imagodei-settings-palette>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit in steward-configuration mode', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="steward-guardian"
        target-id="user-alice"
        target-tier="child"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-settings-palette>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-imagodei-settings-palette> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (palette is still; no motion to gate)', () => {
    const cssText = (
      ElohimImagodeiSettingsPaletteClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="user-alice"
        target-id="user-alice"
      ></elohim-imagodei-settings-palette>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-imagodei-settings-palette> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiSettingsPalette>(
      'he-IL',
      html`
        <elohim-imagodei-settings-palette
          viewer-id="user-alice"
          target-id="user-alice"
        ></elohim-imagodei-settings-palette>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const list = el.shadowRoot!.querySelector('.settings-list')!;
    const rect = list.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiSettingsPaletteClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
