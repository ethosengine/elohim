import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalRulesPanel as ElohimQahalRulesPanelClass } from './elohim-qahal-rules-panel.js';
import type { ElohimQahalRulesPanel, Rubric } from './elohim-qahal-rules-panel.js';

const SAMPLE_RUBRIC: Rubric = {
  name: 'Dawn Runners Standing Rubric',
  standingHonors: [
    'showing up consistently for the group',
    'contributing to the commons with care',
    'receiving recognition from peers',
  ],
  bloomMapping: [
    { tier: 'remember', standingLabel: 'Visitor — learning the space' },
    { tier: 'understand', standingLabel: 'Engaged — grasping the patterns' },
    { tier: 'apply', standingLabel: 'Contributor — putting it into practice' },
    { tier: 'analyze', standingLabel: 'Steward — discerning the whole' },
    { tier: 'evaluate', standingLabel: 'Elder — holding the arc' },
    { tier: 'create', standingLabel: 'Founder — shaping the future' },
  ],
  cadenceLabel: 'Monthly recalibration',
  frictionGradientNote: 'Accumulation beyond steward-tier triggers increasing friction',
  configuredBy: ['alice', 'bob'],
  lastRevised: '2026-05-01',
};

describe('<elohim-qahal-rules-panel>', () => {
  it('renders the rubric name as an h2', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent!.trim()).to.equal('Dawn Runners Standing Rubric');
  });

  it('renders all three standing honors as list items', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('showing up consistently for the group');
    expect(text).to.include('contributing to the commons with care');
    expect(text).to.include('receiving recognition from peers');
  });

  it('renders all six Bloom-tier mappings', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const dts = el.shadowRoot!.querySelectorAll('dt');
    const dds = el.shadowRoot!.querySelectorAll('dd');
    expect(dts).to.have.lengthOf(6);
    expect(dds).to.have.lengthOf(6);
    const tierNames = Array.from(dts).map(dt => dt.textContent!.trim());
    expect(tierNames).to.include('remember');
    expect(tierNames).to.include('understand');
    expect(tierNames).to.include('apply');
    expect(tierNames).to.include('analyze');
    expect(tierNames).to.include('evaluate');
    expect(tierNames).to.include('create');
  });

  it('renders Bloom-tier standing labels in dd elements', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Visitor — learning the space');
    expect(text).to.include('Steward — discerning the whole');
  });

  it('renders cadence label', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Monthly recalibration');
  });

  it('renders friction-gradient note', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Accumulation beyond steward-tier triggers increasing friction');
  });

  it('renders configured-by ids', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('alice');
    expect(text).to.include('bob');
  });

  it('renders last-revised date', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('2026-05-01');
  });

  it('renders empty state when rubric is null', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel></elohim-qahal-rules-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('No rubric configured');
    expect(el.shadowRoot!.querySelector('h2')).to.not.exist;
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-rules-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalRulesPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-rules-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalRulesPanel>(
      'he-IL',
      html`
        <elohim-qahal-rules-panel .rubric=${SAMPLE_RUBRIC}></elohim-qahal-rules-panel>
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
      ElohimQahalRulesPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
