import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalSidebar as ElohimQahalSidebarClass } from './elohim-qahal-sidebar.js';
import type { ElohimQahalSidebar } from './elohim-qahal-sidebar.js';

describe('<elohim-qahal-sidebar>', () => {
  it('renders the qahalName in a heading', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`
      <elohim-qahal-sidebar qahal-name="Dawn Runners"></elohim-qahal-sidebar>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent!.trim()).to.equal('Dawn Runners');
  });

  it('renders four named slots (panels, curated, external, power-user)', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`
      <elohim-qahal-sidebar qahal-name="Test Qahal"></elohim-qahal-sidebar>
    `);
    const slots = el.shadowRoot!.querySelectorAll('slot');
    const slotNames = Array.from(slots).map(s => s.getAttribute('name'));
    expect(slotNames).to.include('panels');
    expect(slotNames).to.include('curated');
    expect(slotNames).to.include('external');
    expect(slotNames).to.include('power-user');
  });

  it('slots assigned content appears in the correct slot', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`
      <elohim-qahal-sidebar qahal-name="Test Qahal">
        <div slot="panels" id="p1">Panels content</div>
        <div slot="curated" id="c1">Curated content</div>
        <div slot="external" id="e1">External content</div>
        <div slot="power-user" id="u1">Power user content</div>
      </elohim-qahal-sidebar>
    `);
    expect(el.querySelector('[slot="panels"]')!.textContent).to.equal('Panels content');
    expect(el.querySelector('[slot="curated"]')!.textContent).to.equal('Curated content');
    expect(el.querySelector('[slot="external"]')!.textContent).to.equal('External content');
    expect(el.querySelector('[slot="power-user"]')!.textContent).to.equal('Power user content');
  });

  it('renders section dividers between slot sections', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`
      <elohim-qahal-sidebar qahal-name="Test"></elohim-qahal-sidebar>
    `);
    const dividers = el.shadowRoot!.querySelectorAll('.section-divider');
    expect(dividers).to.have.lengthOf(3);
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`
      <elohim-qahal-sidebar qahal-name="Accessibility Test">
        <div slot="panels">Panel A</div>
        <div slot="curated">Curated A</div>
        <div slot="external">External A</div>
        <div slot="power-user">Power user A</div>
      </elohim-qahal-sidebar>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-sidebar> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (sidebar is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalSidebarClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`
      <elohim-qahal-sidebar qahal-name="Test"></elohim-qahal-sidebar>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-sidebar> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalSidebar>(
      'he-IL',
      html`
        <elohim-qahal-sidebar qahal-name="קהל"></elohim-qahal-sidebar>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const h2 = el.shadowRoot!.querySelector('h2')!;
    const rect = h2.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalSidebarClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
