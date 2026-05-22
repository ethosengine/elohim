import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalAttestationsPanel as ElohimQahalAttestationsPanelClass } from './elohim-qahal-attestations-panel.js';
import type {
  AttestationSummary,
  ElohimQahalAttestationsPanel,
} from './elohim-qahal-attestations-panel.js';

const SAMPLE_ATTESTATIONS: AttestationSummary = {
  recentCount: 12,
  byKindCounts: { quiz: 5, peer: 3, contribution: 4 },
  lastAttestationAt: '2026-05-20T14:30:00Z',
};

describe('<elohim-qahal-attestations-panel>', () => {
  it('renders an h2 title', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent).to.include('Attestations');
  });

  it('renders the recent count headline', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const count = el.shadowRoot!.querySelector('.recent-count');
    expect(count).to.exist;
    expect(count!.textContent!.trim()).to.equal('12');
  });

  it('renders by-kind breakdown table with each kind and count', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('quiz');
    expect(text).to.include('5');
    expect(text).to.include('peer');
    expect(text).to.include('3');
    expect(text).to.include('contribution');
    expect(text).to.include('4');
  });

  it('renders the last-attestation timestamp', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const lastAt = el.shadowRoot!.querySelector('.last-at');
    expect(lastAt).to.exist;
    expect(lastAt!.textContent).to.include('2026-05-20T14:30:00Z');
  });

  it('renders a .stub-note explaining the MVP deferral', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const note = el.shadowRoot!.querySelector('.stub-note');
    expect(note).to.exist;
    expect(note!.textContent).to.include('Visual stub at MVP');
    expect(note!.textContent).to.include('drill-down');
  });

  it('renders correct number of kind rows in the breakdown table', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const rows = el.shadowRoot!.querySelectorAll('.kind-table tbody tr');
    expect(rows).to.have.lengthOf(3);
  });

  it('renders empty state without crash when attestations is null', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel></elohim-qahal-attestations-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent).to.include('Attestations');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-attestations-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalAttestationsPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalAttestationsPanel>(html`
      <elohim-qahal-attestations-panel
        .attestations=${SAMPLE_ATTESTATIONS}
      ></elohim-qahal-attestations-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-attestations-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalAttestationsPanel>(
      'he-IL',
      html`
        <elohim-qahal-attestations-panel
          .attestations=${SAMPLE_ATTESTATIONS}
        ></elohim-qahal-attestations-panel>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const section = el.shadowRoot!.querySelector('section')!;
    const rect = section.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalAttestationsPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
