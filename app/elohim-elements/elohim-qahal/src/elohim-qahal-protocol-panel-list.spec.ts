import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalProtocolPanelList as ElohimQahalProtocolPanelListClass } from './elohim-qahal-protocol-panel-list.js';
import type {
  ElohimQahalProtocolPanelList,
  ProtocolPanelDescriptor,
} from './elohim-qahal-protocol-panel-list.js';

const SAMPLE_PANELS: ProtocolPanelDescriptor[] = [
  { id: 'overview', label: 'Overview', active: true },
  { id: 'resources', label: 'Resources', active: false },
  { id: 'members', label: 'Members', active: false },
];

describe('<elohim-qahal-protocol-panel-list>', () => {
  it('renders a button for each panel', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list .panels=${SAMPLE_PANELS}></elohim-qahal-protocol-panel-list>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons).to.have.lengthOf(3);
  });

  it('renders no buttons when panels is empty', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list></elohim-qahal-protocol-panel-list>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons).to.have.lengthOf(0);
  });

  it('marks the active panel with aria-current="page"', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list .panels=${SAMPLE_PANELS}></elohim-qahal-protocol-panel-list>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons[0].getAttribute('aria-current')).to.equal('page');
    expect(buttons[1].getAttribute('aria-current')).to.equal('false');
    expect(buttons[2].getAttribute('aria-current')).to.equal('false');
  });

  it('emits panel-changed event with correct id when a button is clicked', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list .panels=${SAMPLE_PANELS}></elohim-qahal-protocol-panel-list>
    `);
    const events: CustomEvent[] = [];
    el.addEventListener('panel-changed', e => events.push(e as CustomEvent));

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[1] as HTMLButtonElement).click();

    expect(events).to.have.lengthOf(1);
    expect(events[0].detail).to.deep.equal({ id: 'resources' });
  });

  it('panel-changed event bubbles and is composed', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list .panels=${SAMPLE_PANELS}></elohim-qahal-protocol-panel-list>
    `);
    let captured: CustomEvent | null = null;
    el.addEventListener('panel-changed', e => {
      captured = e as CustomEvent;
    });

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[0] as HTMLButtonElement).click();

    expect(captured).to.not.be.null;
    expect((captured as unknown as CustomEvent).bubbles).to.be.true;
    expect((captured as unknown as CustomEvent).composed).to.be.true;
  });

  it('panel-changed event carries the panel id', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list .panels=${SAMPLE_PANELS}></elohim-qahal-protocol-panel-list>
    `);
    const events: CustomEvent[] = [];
    el.addEventListener('panel-changed', e => events.push(e as CustomEvent));

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[2] as HTMLButtonElement).click();

    expect(events[0].detail.id).to.equal('members');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list .panels=${SAMPLE_PANELS}></elohim-qahal-protocol-panel-list>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-protocol-panel-list> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel list is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalProtocolPanelListClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalProtocolPanelList>(html`
      <elohim-qahal-protocol-panel-list .panels=${SAMPLE_PANELS}></elohim-qahal-protocol-panel-list>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-protocol-panel-list> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalProtocolPanelList>(
      'he-IL',
      html`
        <elohim-qahal-protocol-panel-list
          .panels=${SAMPLE_PANELS}
        ></elohim-qahal-protocol-panel-list>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const nav = el.shadowRoot!.querySelector('nav')!;
    const rect = nav.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalProtocolPanelListClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
