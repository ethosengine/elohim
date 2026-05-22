import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiStewardConfigureBanner as ElohimImagodeiStewardConfigureBannerClass } from './elohim-imagodei-steward-configure-banner.js';
import type { ElohimImagodeiStewardConfigureBanner } from './elohim-imagodei-steward-configure-banner.js';

describe('<elohim-imagodei-steward-configure-banner>', () => {
  it('renders the steward name', async () => {
    const el = await fixture<ElohimImagodeiStewardConfigureBanner>(html`
      <elohim-imagodei-steward-configure-banner
        steward-name="Guardian Alice"
        stewardee-name="Young Bob"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-steward-configure-banner>
    `);
    expect(el.shadowRoot!.textContent).to.include('Guardian Alice');
  });

  it('renders the stewardee name', async () => {
    const el = await fixture<ElohimImagodeiStewardConfigureBanner>(html`
      <elohim-imagodei-steward-configure-banner
        steward-name="Guardian Alice"
        stewardee-name="Young Bob"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-steward-configure-banner>
    `);
    expect(el.shadowRoot!.textContent).to.include('Young Bob');
  });

  it('renders the witness qahal name', async () => {
    const el = await fixture<ElohimImagodeiStewardConfigureBanner>(html`
      <elohim-imagodei-steward-configure-banner
        steward-name="Guardian Alice"
        stewardee-name="Young Bob"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-steward-configure-banner>
    `);
    expect(el.shadowRoot!.textContent).to.include('Dawn Runners');
  });

  it('renders a region with an appropriate aria-label', async () => {
    const el = await fixture<ElohimImagodeiStewardConfigureBanner>(html`
      <elohim-imagodei-steward-configure-banner
        steward-name="Guardian Alice"
        stewardee-name="Young Bob"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-steward-configure-banner>
    `);
    const region = el.shadowRoot!.querySelector('[role="region"]')!;
    expect(region).to.exist;
    expect(region.getAttribute('aria-label')).to.include('Guardian Alice');
    expect(region.getAttribute('aria-label')).to.include('Young Bob');
  });

  it('renders the Continue button', async () => {
    const el = await fixture<ElohimImagodeiStewardConfigureBanner>(html`
      <elohim-imagodei-steward-configure-banner
        steward-name="Guardian Alice"
        stewardee-name="Young Bob"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-steward-configure-banner>
    `);
    const btn = el.shadowRoot!.querySelector('.continue-btn');
    expect(btn).to.exist;
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimImagodeiStewardConfigureBanner>(html`
      <elohim-imagodei-steward-configure-banner
        steward-name="Guardian Alice"
        stewardee-name="Young Bob"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-steward-configure-banner>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-imagodei-steward-configure-banner> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (banner is still; no motion to gate)', () => {
    const cssText = (
      ElohimImagodeiStewardConfigureBannerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiStewardConfigureBanner>(html`
      <elohim-imagodei-steward-configure-banner
        steward-name="Guardian Alice"
        stewardee-name="Young Bob"
        witness-qahal-name="Dawn Runners"
      ></elohim-imagodei-steward-configure-banner>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-imagodei-steward-configure-banner> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiStewardConfigureBanner>(
      'he-IL',
      html`
        <elohim-imagodei-steward-configure-banner
          steward-name="Guardian Alice"
          stewardee-name="Young Bob"
          witness-qahal-name="Dawn Runners"
        ></elohim-imagodei-steward-configure-banner>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const banner = el.shadowRoot!.querySelector('.banner')!;
    const rect = banner.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiStewardConfigureBannerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
