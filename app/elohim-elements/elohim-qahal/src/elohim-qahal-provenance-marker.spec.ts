import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalProvenanceMarker as ElohimQahalProvenanceMarkerClass } from './elohim-qahal-provenance-marker.js';
import type { ElohimQahalProvenanceMarker } from './elohim-qahal-provenance-marker.js';

describe('<elohim-qahal-provenance-marker>', () => {
  it('renders the four provenance categories with distinct symbols', async () => {
    const cases: [string, string][] = [
      ['protocol-panel', '●'],
      ['curated-epr', '◆'],
      ['installed-applet', '⬢'],
      ['external-hyperlink', '⤤'],
    ];
    for (const [category, expected] of cases) {
      const el = await fixture(html`
        <elohim-qahal-provenance-marker category=${category}></elohim-qahal-provenance-marker>
      `);
      expect(el.shadowRoot!.textContent).to.include(expected);
    }
  });

  it('aria-labels the category', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker
        category="external-hyperlink"
      ></elohim-qahal-provenance-marker>
    `);
    const label = el.shadowRoot!.querySelector('[aria-label]')?.getAttribute('aria-label');
    expect(label).to.include('external hyperlink');
  });

  it('aria-label is exact for each known category', async () => {
    const cases: [string, string][] = [
      ['protocol-panel', 'protocol panel'],
      ['curated-epr', 'curated EPR'],
      ['installed-applet', 'installed applet'],
      ['external-hyperlink', 'external hyperlink (leaving the elohim network)'],
    ];
    for (const [category, expectedLabel] of cases) {
      const el = await fixture<ElohimQahalProvenanceMarker>(html`
        <elohim-qahal-provenance-marker category=${category}></elohim-qahal-provenance-marker>
      `);
      const label = el.shadowRoot!.querySelector('[aria-label]')?.getAttribute('aria-label');
      expect(label, `category=${category}`).to.equal(expectedLabel);
    }
  });

  it('marks external-hyperlink as offline when offline attribute is set', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker
        category="external-hyperlink"
        offline
      ></elohim-qahal-provenance-marker>
    `);
    expect(el.offline).to.be.true;
  });

  it('offline is false by default', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker
        category="external-hyperlink"
      ></elohim-qahal-provenance-marker>
    `);
    expect(el.offline).to.be.false;
  });

  it('reflects offline attribute on host when offline property is toggled', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker
        category="external-hyperlink"
      ></elohim-qahal-provenance-marker>
    `);
    expect(el.hasAttribute('offline')).to.be.false;
    el.offline = true;
    await elementUpdated(el);
    expect(el.hasAttribute('offline')).to.be.true;
    el.offline = false;
    await elementUpdated(el);
    expect(el.hasAttribute('offline')).to.be.false;
  });

  it('renders an unknown category without crashing (empty symbol, raw string as label)', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker category="unknown"></elohim-qahal-provenance-marker>
    `);
    // Must not throw; shadowRoot must exist
    expect(el.shadowRoot).to.exist;
    // Symbol falls back to empty string
    const symbol = el.shadowRoot!.querySelector('[aria-label]')?.textContent?.trim() ?? null;
    expect(symbol).to.equal('');
    // Label falls back to the raw category string
    const label = el.shadowRoot!.querySelector('[aria-label]')?.getAttribute('aria-label');
    expect(label).to.equal('unknown');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker category="curated-epr"></elohim-qahal-provenance-marker>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-provenance-marker> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (marker is still; no motion to gate)', () => {
    // The marker declares maxStimulus=still so it must not have any CSS transitions.
    const cssText = (
      ElohimQahalProvenanceMarkerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker category="protocol-panel"></elohim-qahal-provenance-marker>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-provenance-marker> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalProvenanceMarker>(
      'he-IL',
      html`
        <elohim-qahal-provenance-marker
          category="installed-applet"
        ></elohim-qahal-provenance-marker>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    // Marker is symbolic — bounding box has nonzero dimensions
    const span = el.shadowRoot!.querySelector('[aria-label]')!;
    const rect = span.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalProvenanceMarkerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
