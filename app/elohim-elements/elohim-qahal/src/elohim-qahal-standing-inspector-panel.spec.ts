import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalStandingInspectorPanel as ElohimQahalStandingInspectorPanelClass } from './elohim-qahal-standing-inspector-panel.js';
import type {
  ElohimQahalStandingInspectorPanel,
  StandingBreakdown,
} from './elohim-qahal-standing-inspector-panel.js';

const SAMPLE_STANDING: StandingBreakdown = {
  tier: 'contributor',
  bloomTier: 'apply',
  attestations: 12,
  affinity: 87,
  debits: 2,
};

describe('<elohim-qahal-standing-inspector-panel>', () => {
  it('renders an h2 title', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${SAMPLE_STANDING}
      ></elohim-qahal-standing-inspector-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent).to.include('Standing Inspector');
  });

  it('renders capability-tier-chip for the tier row', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${SAMPLE_STANDING}
      ></elohim-qahal-standing-inspector-panel>
    `);
    const chip = el.shadowRoot!.querySelector('elohim-qahal-capability-tier-chip');
    expect(chip).to.exist;
    expect(chip!.getAttribute('tier')).to.equal('contributor');
  });

  it('renders standing-ring for the Bloom tier row', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${SAMPLE_STANDING}
      ></elohim-qahal-standing-inspector-panel>
    `);
    const ring = el.shadowRoot!.querySelector('elohim-qahal-standing-ring');
    expect(ring).to.exist;
    expect(ring!.getAttribute('bloom-tier')).to.equal('apply');
  });

  it('renders attestations, affinity, and debits counts', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${SAMPLE_STANDING}
      ></elohim-qahal-standing-inspector-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('12');
    expect(text).to.include('87');
    expect(text).to.include('2');
  });

  it('renders a .stub-note explaining the MVP deferral', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${SAMPLE_STANDING}
      ></elohim-qahal-standing-inspector-panel>
    `);
    const note = el.shadowRoot!.querySelector('.stub-note');
    expect(note).to.exist;
    expect(note!.textContent).to.include('Visual stub at MVP');
    expect(note!.textContent).to.include('attestation-chain walk');
  });

  it('renders empty state without crash when standing is null', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel></elohim-qahal-standing-inspector-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent).to.include('Standing Inspector');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${SAMPLE_STANDING}
      ></elohim-qahal-standing-inspector-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-standing-inspector-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalStandingInspectorPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${SAMPLE_STANDING}
      ></elohim-qahal-standing-inspector-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-standing-inspector-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalStandingInspectorPanel>(
      'he-IL',
      html`
        <elohim-qahal-standing-inspector-panel
          .standing=${SAMPLE_STANDING}
        ></elohim-qahal-standing-inspector-panel>
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
      ElohimQahalStandingInspectorPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
