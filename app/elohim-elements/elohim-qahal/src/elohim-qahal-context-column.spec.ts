import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalContextColumn as ElohimQahalContextColumnClass } from './elohim-qahal-context-column.js';
import type { ElohimQahalContextColumn } from './elohim-qahal-context-column.js';

describe('<elohim-qahal-context-column>', () => {
  it('renders an <aside> with aria-label="Context"', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column></elohim-qahal-context-column>
    `);
    const aside = el.shadowRoot!.querySelector('aside');
    expect(aside).to.exist;
    expect(aside!.getAttribute('aria-label')).to.equal('Context');
  });

  it('renders three named slots: co-steward, rules, discovery', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column></elohim-qahal-context-column>
    `);
    const slots = el.shadowRoot!.querySelectorAll('slot');
    const slotNames = Array.from(slots).map(s => s.getAttribute('name'));
    expect(slotNames).to.include('co-steward');
    expect(slotNames).to.include('rules');
    expect(slotNames).to.include('discovery');
  });

  it('renders a "Co-steward" section heading', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column></elohim-qahal-context-column>
    `);
    const headings = el.shadowRoot!.querySelectorAll('h3');
    const texts = Array.from(headings).map(h => h.textContent!.trim());
    expect(texts).to.include('Co-steward');
  });

  it('renders a "Rules" section heading', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column></elohim-qahal-context-column>
    `);
    const headings = el.shadowRoot!.querySelectorAll('h3');
    const texts = Array.from(headings).map(h => h.textContent!.trim());
    expect(texts).to.include('Rules');
  });

  it('renders a "Discovery" section heading', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column></elohim-qahal-context-column>
    `);
    const headings = el.shadowRoot!.querySelectorAll('h3');
    const texts = Array.from(headings).map(h => h.textContent!.trim());
    expect(texts).to.include('Discovery');
  });

  it('slots assigned content appears in the correct slot', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column>
        <div slot="co-steward">Elohim present</div>
        <div slot="rules">Be kind</div>
        <div slot="discovery">Suggested connections</div>
      </elohim-qahal-context-column>
    `);
    expect(el.querySelector('[slot="co-steward"]')!.textContent).to.equal('Elohim present');
    expect(el.querySelector('[slot="rules"]')!.textContent).to.equal('Be kind');
    expect(el.querySelector('[slot="discovery"]')!.textContent).to.equal('Suggested connections');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column>
        <div slot="co-steward">Co-steward panel</div>
        <div slot="rules">Rules panel</div>
        <div slot="discovery">Discovery panel</div>
      </elohim-qahal-context-column>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-context-column> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (context column is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalContextColumnClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalContextColumn>(html`
      <elohim-qahal-context-column></elohim-qahal-context-column>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-context-column> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalContextColumn>(
      'he-IL',
      html`
        <elohim-qahal-context-column></elohim-qahal-context-column>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const aside = el.shadowRoot!.querySelector('aside')!;
    const rect = aside.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalContextColumnClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
