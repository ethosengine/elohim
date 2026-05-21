import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalCareEconomyMarker as ElohimQahalCareEconomyMarkerClass } from './elohim-qahal-care-economy-marker.js';
import type { ElohimQahalCareEconomyMarker } from './elohim-qahal-care-economy-marker.js';

describe('<elohim-qahal-care-economy-marker>', () => {
  it('renders kind="care" with positive token count', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="care" tokens="5"></elohim-qahal-care-economy-marker>
    `);
    expect(el.shadowRoot!.textContent).to.match(/\+5.*care/);
  });

  it('supports kinds: care, presence, repair, growth, time', async () => {
    for (const k of ['care', 'presence', 'repair', 'growth', 'time']) {
      const el = await fixture(html`
        <elohim-qahal-care-economy-marker kind=${k} tokens="1"></elohim-qahal-care-economy-marker>
      `);
      expect(el.getAttribute('kind')).to.equal(k);
    }
  });

  it('renders the correct icon for each kind', async () => {
    const cases: [string, string][] = [
      ['care', '✨'],
      ['presence', '👁'],
      ['repair', '🛠'],
      ['growth', '🌱'],
      ['time', '⏰'],
    ];
    for (const [kind, icon] of cases) {
      const el = await fixture(html`
        <elohim-qahal-care-economy-marker
          kind=${kind}
          tokens="1"
        ></elohim-qahal-care-economy-marker>
      `);
      expect(el.shadowRoot!.textContent).to.include(icon);
    }
  });

  it('aria-label is exact: "<kind> contribution, <tokens> tokens"', async () => {
    const cases: [string, number, string][] = [
      ['care', 5, 'care contribution, 5 tokens'],
      ['presence', 1, 'presence contribution, 1 tokens'],
      ['repair', 3, 'repair contribution, 3 tokens'],
      ['growth', 2, 'growth contribution, 2 tokens'],
      ['time', 10, 'time contribution, 10 tokens'],
    ];
    for (const [kind, tokens, expectedLabel] of cases) {
      const el = await fixture<ElohimQahalCareEconomyMarker>(html`
        <elohim-qahal-care-economy-marker
          kind=${kind}
          tokens=${tokens}
        ></elohim-qahal-care-economy-marker>
      `);
      const label = el.shadowRoot!.querySelector('[aria-label]')?.getAttribute('aria-label');
      expect(label, `kind=${kind} tokens=${tokens}`).to.equal(expectedLabel);
    }
  });

  it('defaults tokens to 1 when not specified', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="growth"></elohim-qahal-care-economy-marker>
    `);
    expect(el.tokens).to.equal(1);
    expect(el.shadowRoot!.textContent).to.include('+1');
  });

  it('defaults kind to "care" when not specified', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker></elohim-qahal-care-economy-marker>
    `);
    expect(el.kind).to.equal('care');
  });

  it('updates rendered content when tokens property changes', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="care" tokens="1"></elohim-qahal-care-economy-marker>
    `);
    expect(el.shadowRoot!.textContent).to.include('+1');
    el.tokens = 7;
    await elementUpdated(el);
    expect(el.shadowRoot!.textContent).to.include('+7');
  });

  it('updates rendered content when kind property changes', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="care" tokens="2"></elohim-qahal-care-economy-marker>
    `);
    expect(el.shadowRoot!.textContent).to.include('✨');
    el.kind = 'growth';
    await elementUpdated(el);
    expect(el.shadowRoot!.textContent).to.include('🌱');
  });

  it('reflects kind attribute on host when kind property changes', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="care"></elohim-qahal-care-economy-marker>
    `);
    expect(el.getAttribute('kind')).to.equal('care');
    el.kind = 'repair';
    await elementUpdated(el);
    expect(el.getAttribute('kind')).to.equal('repair');
  });

  it('renders an unknown kind without crashing (empty icon, raw kind in label)', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker
        kind="unknown"
        tokens="1"
      ></elohim-qahal-care-economy-marker>
    `);
    // Must not throw; shadowRoot must exist
    expect(el.shadowRoot).to.exist;
    // Label falls back to "unknown contribution, 1 tokens"
    const label = el.shadowRoot!.querySelector('[aria-label]')?.getAttribute('aria-label');
    expect(label).to.equal('unknown contribution, 1 tokens');
    // The text content includes the raw kind and count
    expect(el.shadowRoot!.textContent).to.include('unknown');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="growth" tokens="3"></elohim-qahal-care-economy-marker>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-care-economy-marker> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (marker is still; no motion to gate)', () => {
    // The marker declares maxStimulus=still so it must not have any CSS transitions.
    const cssText = (
      ElohimQahalCareEconomyMarkerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="care" tokens="5"></elohim-qahal-care-economy-marker>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-care-economy-marker> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalCareEconomyMarker>(
      'he-IL',
      html`
        <elohim-qahal-care-economy-marker kind="time" tokens="2"></elohim-qahal-care-economy-marker>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    // Marker is textual+symbolic — bounding box has nonzero dimensions
    const span = el.shadowRoot!.querySelector('[aria-label]')!;
    const rect = span.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalCareEconomyMarkerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
