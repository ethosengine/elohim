import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalMainViewer as ElohimQahalMainViewerClass } from './elohim-qahal-main-viewer.js';
import type { ElohimQahalMainViewer } from './elohim-qahal-main-viewer.js';

describe('<elohim-qahal-main-viewer>', () => {
  it('renders slotted content in the default slot', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer active-panel-name="Members">
        <div id="panel-content">Hello panel</div>
      </elohim-qahal-main-viewer>
    `);
    expect(el.querySelector('#panel-content')).to.exist;
    expect(el.querySelector('#panel-content')!.textContent).to.equal('Hello panel');
  });

  it('exposes the active-panel-name attribute as activePanelName property', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer active-panel-name="Learning Path"></elohim-qahal-main-viewer>
    `);
    expect(el.activePanelName).to.equal('Learning Path');
  });

  it('sets aria-label on <main> from active-panel-name', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer active-panel-name="Governance"></elohim-qahal-main-viewer>
    `);
    const main = el.shadowRoot!.querySelector('main');
    expect(main).to.exist;
    expect(main!.getAttribute('aria-label')).to.equal('Governance');
  });

  it('falls back to "Active panel" aria-label when activePanelName is empty', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer></elohim-qahal-main-viewer>
    `);
    const main = el.shadowRoot!.querySelector('main');
    expect(main!.getAttribute('aria-label')).to.equal('Active panel');
  });

  it('<main> element has role="main"', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer active-panel-name="Members"></elohim-qahal-main-viewer>
    `);
    const main = el.shadowRoot!.querySelector('main');
    expect(main!.getAttribute('role')).to.equal('main');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer active-panel-name="Members">
        <p>Content here</p>
      </elohim-qahal-main-viewer>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-main-viewer> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (viewer is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalMainViewerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer active-panel-name="Test"></elohim-qahal-main-viewer>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-main-viewer> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalMainViewer>(
      'he-IL',
      html`
        <elohim-qahal-main-viewer active-panel-name="חברים"></elohim-qahal-main-viewer>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const main = el.shadowRoot!.querySelector('main')!;
    const rect = main.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalMainViewerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
