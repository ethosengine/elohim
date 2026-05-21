import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalStandingRing as ElohimQahalStandingRingClass } from './elohim-qahal-standing-ring.js';
import type { ElohimQahalStandingRing } from './elohim-qahal-standing-ring.js';

describe('<elohim-qahal-standing-ring>', () => {
  it('renders with bloom-tier attribute', async () => {
    const el = await fixture<ElohimQahalStandingRing>(html`
      <elohim-qahal-standing-ring bloom-tier="apply"></elohim-qahal-standing-ring>
    `);
    expect(el.shadowRoot!.textContent).to.match(/●●●○○○/);
  });

  it('supports six bloom tiers', async () => {
    const tiers = ['remember', 'understand', 'apply', 'analyze', 'evaluate', 'create'] as const;
    for (const t of tiers) {
      const el = await fixture<ElohimQahalStandingRing>(html`
        <elohim-qahal-standing-ring bloom-tier=${t}></elohim-qahal-standing-ring>
      `);
      expect(el.bloomTier).to.equal(t);
    }
  });

  it('aria-labels the bloom tier', async () => {
    const el = await fixture<ElohimQahalStandingRing>(html`
      <elohim-qahal-standing-ring bloom-tier="apply"></elohim-qahal-standing-ring>
    `);
    expect(el.shadowRoot!.querySelector('[role="img"]')?.getAttribute('aria-label')).to.equal(
      'Bloom tier: apply (3 of 6)'
    );
  });

  it('falls back to 1 filled dot when bloom-tier is not a known tier', async () => {
    const el = await fixture<ElohimQahalStandingRing>(html`
      <elohim-qahal-standing-ring bloom-tier="mastery"></elohim-qahal-standing-ring>
    `);
    const trimmed = (el.shadowRoot!.querySelector('[role="img"]')?.textContent ?? '').trim();
    expect(trimmed).to.equal('●○○○○○');
  });

  it('renders correct filled/empty dot count for each tier', async () => {
    const cases: [string, RegExp][] = [
      ['remember', /^●○○○○○$/],
      ['understand', /^●●○○○○$/],
      ['apply', /^●●●○○○$/],
      ['analyze', /^●●●●○○$/],
      ['evaluate', /^●●●●●○$/],
      ['create', /^●●●●●●$/],
    ];
    for (const [tier, pattern] of cases) {
      const el = await fixture<ElohimQahalStandingRing>(html`
        <elohim-qahal-standing-ring bloom-tier=${tier}></elohim-qahal-standing-ring>
      `);
      const text = (el.shadowRoot!.querySelector('[role="img"]')?.textContent ?? '').trim();
      expect(text, `tier=${tier}`).to.match(pattern);
    }
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalStandingRing>(html`
      <elohim-qahal-standing-ring bloom-tier="evaluate"></elohim-qahal-standing-ring>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-standing-ring> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (ring is still; no motion to gate)', () => {
    // The ring declares maxStimulus=still so it must not have any CSS transitions.
    const cssText = (
      ElohimQahalStandingRingClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalStandingRing>(html`
      <elohim-qahal-standing-ring bloom-tier="apply"></elohim-qahal-standing-ring>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-standing-ring> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalStandingRing>(
      'he-IL',
      html`
        <elohim-qahal-standing-ring bloom-tier="understand"></elohim-qahal-standing-ring>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    // Dots are symbolic/visual — bounding box has nonzero dimensions
    const span = el.shadowRoot!.querySelector('[role="img"]')!;
    const rect = span.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalStandingRingClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
