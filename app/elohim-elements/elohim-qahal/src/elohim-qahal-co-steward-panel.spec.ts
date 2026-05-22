import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalCoStewardPanel as ElohimQahalCoStewardPanelClass } from './elohim-qahal-co-steward-panel.js';
import type { ElohimQahalCoStewardPanel } from './elohim-qahal-co-steward-panel.js';

const PENDING = [
  'Alice shared 3 care tokens after Thursday gathering',
  'Bob offered repair support for the website',
  'Carol contributed 2 hours of presence',
];

describe('<elohim-qahal-co-steward-panel>', () => {
  it('renders the primary observation prominently', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
      ></elohim-qahal-co-steward-panel>
    `);
    const p = el.shadowRoot!.querySelector('.primary-observation');
    expect(p).to.exist;
    expect(p!.textContent!.trim()).to.equal('The household is steady.');
  });

  it('renders pending acknowledgments list when pendingAcknowledgments is non-empty', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.pending-list li');
    expect(items).to.have.lengthOf(3);
    expect(items[0].textContent!.trim()).to.equal(
      'Alice shared 3 care tokens after Thursday gathering'
    );
  });

  it('renders pending-acknowledgments heading when pending list is non-empty', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const heading = el.shadowRoot!.querySelector('.pending-heading');
    expect(heading).to.exist;
    expect(heading!.textContent).to.include('pending acknowledgment');
  });

  it('renders the no-urgency footer when pending list is non-empty', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const footer = el.shadowRoot!.querySelector('.no-urgency-footer');
    expect(footer).to.exist;
    expect(footer!.textContent).to.include('No urgency');
    expect(footer!.textContent).to.include('acknowledge them when you next sit down');
  });

  it('does not render pending section when pendingAcknowledgments is empty', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${[]}
      ></elohim-qahal-co-steward-panel>
    `);
    expect(el.shadowRoot!.querySelector('.pending-list')).to.not.exist;
    expect(el.shadowRoot!.querySelector('.pending-heading')).to.not.exist;
    expect(el.shadowRoot!.querySelector('.no-urgency-footer')).to.not.exist;
  });

  it('does not render pending section by default', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
      ></elohim-qahal-co-steward-panel>
    `);
    expect(el.shadowRoot!.querySelector('.pending-list')).to.not.exist;
  });

  it('tone discipline: no "alert" in rendered content', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const text = (el.shadowRoot!.textContent ?? '').toLowerCase();
    expect(text).to.not.include('alert');
  });

  it('tone discipline: no "warning" in rendered content', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.not.include('warning');
  });

  it('tone discipline: no exclamation marks in rendered content', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.not.include('!');
  });

  it('updates pending list when pendingAcknowledgments property changes', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${[]}
      ></elohim-qahal-co-steward-panel>
    `);
    expect(el.shadowRoot!.querySelector('.pending-list')).to.not.exist;
    el.pendingAcknowledgments = PENDING;
    await elementUpdated(el);
    const items = el.shadowRoot!.querySelectorAll('.pending-list li');
    expect(items).to.have.lengthOf(3);
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-co-steward-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalCoStewardPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
        .pendingAcknowledgments=${PENDING}
      ></elohim-qahal-co-steward-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-co-steward-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalCoStewardPanel>(
      'he-IL',
      html`
        <elohim-qahal-co-steward-panel
          primary-observation="The household is steady."
          .pendingAcknowledgments=${PENDING}
        ></elohim-qahal-co-steward-panel>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const div = el.shadowRoot!.querySelector('div')!;
    const rect = div.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalCoStewardPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
