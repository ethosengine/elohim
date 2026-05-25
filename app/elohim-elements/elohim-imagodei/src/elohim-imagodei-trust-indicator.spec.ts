import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiTrustIndicator as ElohimImagodeiTrustIndicatorClass } from './elohim-imagodei-trust-indicator.js';
import type { ElohimImagodeiTrustIndicator } from './elohim-imagodei-trust-indicator.js';

// ---------------------------------------------------------------------------
// Core contract tests
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-trust-indicator>', () => {
  it('renders doorway-host chrome with the authority label', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha.elohim.host"
      ></elohim-imagodei-trust-indicator>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('alpha.elohim.host');
    expect(
      el.shadowRoot!.querySelector('[part="mode"]')?.getAttribute('data-mode')
    ).to.equal('doorway-host');
  });

  it('renders peer-conductor chrome with a different mode marker', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="peer-conductor"
        authority-label="your conductor on this device"
      ></elohim-imagodei-trust-indicator>
    `);
    expect(
      el.shadowRoot!.querySelector('[part="mode"]')?.getAttribute('data-mode')
    ).to.equal('peer-conductor');
  });

  it('includes the authority label text in the peer-conductor render', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="peer-conductor"
        authority-label="your conductor on this device"
      ></elohim-imagodei-trust-indicator>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('your conductor on this device');
  });

  it('surfaces flywheel hint only when flywheelHint is true', async () => {
    const elNoHint = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha"
      ></elohim-imagodei-trust-indicator>
    `);
    expect(elNoHint.shadowRoot!.querySelector('[part="flywheel-hint"]')).to.be.null;

    const elHint = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha"
        ?flywheel-hint=${true}
      ></elohim-imagodei-trust-indicator>
    `);
    expect(elHint.shadowRoot!.querySelector('[part="flywheel-hint"]')).to.exist;
  });

  it('does NOT show flywheel hint for peer-conductor even when flywheelHint is true', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="peer-conductor"
        authority-label="local"
        ?flywheel-hint=${true}
      ></elohim-imagodei-trust-indicator>
    `);
    // flywheel hint is only meaningful in doorway-host mode
    expect(el.shadowRoot!.querySelector('[part="flywheel-hint"]')).to.be.null;
  });

  it('emits trust-indicator-tap on click', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha"
      ></elohim-imagodei-trust-indicator>
    `);
    let detail: unknown = null;
    el.addEventListener('trust-indicator-tap', e => {
      detail = (e as CustomEvent).detail;
    });
    (el.shadowRoot!.querySelector('button') as HTMLElement).click();
    expect(detail).to.deep.equal({ trustMode: 'doorway-host', authorityLabel: 'alpha' });
  });

  it('emits trust-indicator-tap with correct detail for peer-conductor', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="peer-conductor"
        authority-label="local-device"
      ></elohim-imagodei-trust-indicator>
    `);
    let detail: unknown = null;
    el.addEventListener('trust-indicator-tap', e => {
      detail = (e as CustomEvent).detail;
    });
    (el.shadowRoot!.querySelector('button') as HTMLElement).click();
    expect(detail).to.deep.equal({ trustMode: 'peer-conductor', authorityLabel: 'local-device' });
  });

  it('renders a button element for keyboard interaction', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha"
      ></elohim-imagodei-trust-indicator>
    `);
    const btn = el.shadowRoot!.querySelector('button');
    expect(btn).to.exist;
    expect(btn!.getAttribute('type')).to.equal('button');
  });

  it('defaults to doorway-host mode when no trust-mode attribute is set', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        authority-label="fallback"
      ></elohim-imagodei-trust-indicator>
    `);
    expect(el.trustMode).to.equal('doorway-host');
  });

  it('passes axe accessibility audit for doorway-host mode', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha.elohim.host"
      ></elohim-imagodei-trust-indicator>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit for peer-conductor mode', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="peer-conductor"
        authority-label="your conductor on this device"
      ></elohim-imagodei-trust-indicator>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-trust-indicator> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (chip is still; no motion to gate)', () => {
    const cssText = (
      ElohimImagodeiTrustIndicatorClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('CSS has forced-colors override block', () => {
    const cssText = (
      ElohimImagodeiTrustIndicatorClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha"
      ></elohim-imagodei-trust-indicator>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-trust-indicator> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiTrustIndicator>(
      'he-IL',
      html`
        <elohim-imagodei-trust-indicator
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
        ></elohim-imagodei-trust-indicator>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const btn = el.shadowRoot!.querySelector('button')!;
    const rect = btn.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiTrustIndicatorClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
