import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalPowerUserExpandable as ElohimQahalPowerUserExpandableClass } from './elohim-qahal-power-user-expandable.js';
import type {
  ElohimQahalPowerUserExpandable,
  ProtocolPanelDescriptor,
} from './elohim-qahal-power-user-expandable.js';

const SAMPLE_PANELS: ProtocolPanelDescriptor[] = [
  { id: 'debug', label: 'Debug Console', active: false },
  { id: 'raw-eprs', label: 'Raw EPRs', active: false },
  { id: 'substrate', label: 'Substrate Inspector', active: true },
];

describe('<elohim-qahal-power-user-expandable>', () => {
  it('renders panels when powerUserEnabled is true', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${true}
      ></elohim-qahal-power-user-expandable>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons).to.have.lengthOf(3);
  });

  it('renders nothing (no buttons) when powerUserEnabled is false', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${false}
      ></elohim-qahal-power-user-expandable>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons).to.have.lengthOf(0);
  });

  it('renders nothing by default (powerUserEnabled defaults to false)', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
      ></elohim-qahal-power-user-expandable>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons).to.have.lengthOf(0);
  });

  it('emits panel-changed event when enabled and panel is clicked', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${true}
      ></elohim-qahal-power-user-expandable>
    `);
    const events: CustomEvent[] = [];
    el.addEventListener('panel-changed', e => events.push(e as CustomEvent));

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[0] as HTMLButtonElement).click();

    expect(events).to.have.lengthOf(1);
    expect(events[0].detail).to.deep.equal({ id: 'debug' });
  });

  it('panel-changed event bubbles and is composed', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${true}
      ></elohim-qahal-power-user-expandable>
    `);
    let captured: CustomEvent | null = null;
    el.addEventListener('panel-changed', e => {
      captured = e as CustomEvent;
    });

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[1] as HTMLButtonElement).click();

    expect(captured).to.not.be.null;
    expect((captured as unknown as CustomEvent).bubbles).to.be.true;
    expect((captured as unknown as CustomEvent).composed).to.be.true;
  });

  it('marks the active panel with aria-current="page" when enabled', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${true}
      ></elohim-qahal-power-user-expandable>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons[0].getAttribute('aria-current')).to.equal('false');
    expect(buttons[1].getAttribute('aria-current')).to.equal('false');
    expect(buttons[2].getAttribute('aria-current')).to.equal('page');
  });

  it('passes axe accessibility audit when enabled', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${true}
      ></elohim-qahal-power-user-expandable>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit when disabled', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${false}
      ></elohim-qahal-power-user-expandable>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-power-user-expandable> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (element is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalPowerUserExpandableClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalPowerUserExpandable>(html`
      <elohim-qahal-power-user-expandable
        .panels=${SAMPLE_PANELS}
        ?power-user-enabled=${true}
      ></elohim-qahal-power-user-expandable>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-power-user-expandable> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalPowerUserExpandable>(
      'he-IL',
      html`
        <elohim-qahal-power-user-expandable
          .panels=${SAMPLE_PANELS}
          ?power-user-enabled=${true}
        ></elohim-qahal-power-user-expandable>
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
      ElohimQahalPowerUserExpandableClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
