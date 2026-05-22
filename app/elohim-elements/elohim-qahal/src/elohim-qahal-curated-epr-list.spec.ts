import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalCuratedEprList as ElohimQahalCuratedEprListClass } from './elohim-qahal-curated-epr-list.js';
import type { CuratedEpr, ElohimQahalCuratedEprList } from './elohim-qahal-curated-epr-list.js';

const SAMPLE_EPRS: CuratedEpr[] = [
  { id: 'epr-alpha', label: 'Alpha EPR' },
  { id: 'epr-beta', label: 'Beta EPR' },
  { id: 'epr-gamma', label: 'Gamma EPR', iconPath: '/icons/gamma.svg' },
];

describe('<elohim-qahal-curated-epr-list>', () => {
  it('renders a list item for each EPR', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    const items = el.shadowRoot!.querySelectorAll('li');
    expect(items).to.have.lengthOf(3);
  });

  it('renders an empty state when eprs is empty', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list></elohim-qahal-curated-epr-list>
    `);
    const emptyNote = el.shadowRoot!.querySelector('.empty-note');
    expect(emptyNote).to.exist;
    expect(emptyNote!.textContent).to.include('No curated EPRs');
  });

  it('each item has a ◆ provenance marker with category="curated-epr"', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    const markers = el.shadowRoot!.querySelectorAll(
      'elohim-qahal-provenance-marker[category="curated-epr"]'
    );
    expect(markers).to.have.lengthOf(3);
  });

  it('each item has a ◆ symbol in its marker', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    const markers = el.shadowRoot!.querySelectorAll('elohim-qahal-provenance-marker');
    expect(markers.length).to.be.greaterThan(0);
    for (const marker of Array.from(markers)) {
      expect(marker.shadowRoot!.textContent).to.include('◆');
    }
  });

  it('renders label text for each EPR', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Alpha EPR');
    expect(text).to.include('Beta EPR');
    expect(text).to.include('Gamma EPR');
  });

  it('emits epr-selected event with correct id on click', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    const events: CustomEvent[] = [];
    el.addEventListener('epr-selected', e => events.push(e as CustomEvent));

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[1] as HTMLButtonElement).click();

    expect(events).to.have.lengthOf(1);
    expect(events[0].detail).to.deep.equal({ id: 'epr-beta' });
  });

  it('epr-selected event bubbles and is composed', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    let captured: CustomEvent | null = null;
    el.addEventListener('epr-selected', e => {
      captured = e as CustomEvent;
    });

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[0] as HTMLButtonElement).click();

    expect(captured).to.not.be.null;
    expect((captured as unknown as CustomEvent).bubbles).to.be.true;
    expect((captured as unknown as CustomEvent).composed).to.be.true;
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-curated-epr-list> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (list is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalCuratedEprListClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalCuratedEprList>(html`
      <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-curated-epr-list> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalCuratedEprList>(
      'he-IL',
      html`
        <elohim-qahal-curated-epr-list .eprs=${SAMPLE_EPRS}></elohim-qahal-curated-epr-list>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const ul = el.shadowRoot!.querySelector('ul')!;
    const rect = ul.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalCuratedEprListClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
