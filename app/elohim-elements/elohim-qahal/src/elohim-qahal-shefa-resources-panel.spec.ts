import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalShefaResourcesPanel as ElohimQahalShefaResourcesPanelClass } from './elohim-qahal-shefa-resources-panel.js';
import type {
  ElohimQahalShefaResourcesPanel,
  ShefaSummary,
} from './elohim-qahal-shefa-resources-panel.js';

const SAMPLE_SHEFA: ShefaSummary = {
  inflows: 34,
  outflows: 21,
  commonsShareBalance: 58,
  agreementClauses: 7,
};

describe('<elohim-qahal-shefa-resources-panel>', () => {
  it('renders an h2 title', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel
        .shefa=${SAMPLE_SHEFA}
      ></elohim-qahal-shefa-resources-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent).to.include('Shefa Resources');
  });

  it('renders inflows and outflows counts', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel
        .shefa=${SAMPLE_SHEFA}
      ></elohim-qahal-shefa-resources-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Inflows');
    expect(text).to.include('34');
    expect(text).to.include('Outflows');
    expect(text).to.include('21');
  });

  it('renders commons share balance and agreement clauses', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel
        .shefa=${SAMPLE_SHEFA}
      ></elohim-qahal-shefa-resources-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Commons share balance');
    expect(text).to.include('58');
    expect(text).to.include('Agreement clauses');
    expect(text).to.include('7');
  });

  it('renders a .stub-note explaining the MVP deferral', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel
        .shefa=${SAMPLE_SHEFA}
      ></elohim-qahal-shefa-resources-panel>
    `);
    const note = el.shadowRoot!.querySelector('.stub-note');
    expect(note).to.exist;
    expect(note!.textContent).to.include('Visual stub at MVP');
    expect(note!.textContent).to.include('REA flow visualization');
  });

  it('renders four summary items', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel
        .shefa=${SAMPLE_SHEFA}
      ></elohim-qahal-shefa-resources-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.summary-item');
    expect(items).to.have.lengthOf(4);
  });

  it('renders empty state without crash when shefa is null', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel></elohim-qahal-shefa-resources-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent).to.include('Shefa Resources');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel
        .shefa=${SAMPLE_SHEFA}
      ></elohim-qahal-shefa-resources-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-shefa-resources-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalShefaResourcesPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalShefaResourcesPanel>(html`
      <elohim-qahal-shefa-resources-panel
        .shefa=${SAMPLE_SHEFA}
      ></elohim-qahal-shefa-resources-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-shefa-resources-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalShefaResourcesPanel>(
      'he-IL',
      html`
        <elohim-qahal-shefa-resources-panel
          .shefa=${SAMPLE_SHEFA}
        ></elohim-qahal-shefa-resources-panel>
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
      ElohimQahalShefaResourcesPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
