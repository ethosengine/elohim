import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiSettingControl as ElohimImagodeiSettingControlClass } from './elohim-imagodei-setting-control.js';
import type {
  ElohimImagodeiSettingControl,
  SettingControl,
} from './elohim-imagodei-setting-control.js';

const SAMPLE_SETTING: SettingControl = {
  name: 'Notification volume/style',
  description: 'How and how often you receive notifications',
  value: 'gentle',
  configurableBy: ['self', 'steward'],
  source: 'self',
};

describe('<elohim-imagodei-setting-control>', () => {
  it('renders the setting name', async () => {
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control .setting=${SAMPLE_SETTING}></elohim-imagodei-setting-control>
    `);
    expect(el.shadowRoot!.textContent).to.include('Notification volume/style');
  });

  it('renders the setting value', async () => {
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control .setting=${SAMPLE_SETTING}></elohim-imagodei-setting-control>
    `);
    expect(el.shadowRoot!.textContent).to.include('gentle');
  });

  it('renders the source label', async () => {
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control .setting=${SAMPLE_SETTING}></elohim-imagodei-setting-control>
    `);
    const badge = el.shadowRoot!.querySelector('.source-badge');
    expect(badge).to.exist;
    expect(badge!.textContent).to.include('self');
  });

  it('shows steward source badge styling for steward source', async () => {
    const stewardSetting: SettingControl = { ...SAMPLE_SETTING, source: 'steward' };
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control .setting=${stewardSetting}></elohim-imagodei-setting-control>
    `);
    const badge = el.shadowRoot!.querySelector('.source-badge');
    expect(badge).to.exist;
    expect(badge!.getAttribute('data-source')).to.equal('steward');
  });

  it('edit button is visible only when viewerCanEdit is true', async () => {
    const editable = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control
        .setting=${SAMPLE_SETTING}
        viewer-can-edit
      ></elohim-imagodei-setting-control>
    `);
    expect(editable.shadowRoot!.querySelector('.edit-btn')).to.exist;

    const readOnly = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control .setting=${SAMPLE_SETTING}></elohim-imagodei-setting-control>
    `);
    expect(readOnly.shadowRoot!.querySelector('.edit-btn')).to.be.null;
  });

  it('emits edit-requested with the setting name when edit button is clicked', async () => {
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control
        .setting=${SAMPLE_SETTING}
        viewer-can-edit
      ></elohim-imagodei-setting-control>
    `);
    const events: CustomEvent[] = [];
    el.addEventListener('edit-requested', e => events.push(e as CustomEvent));

    const btn = el.shadowRoot!.querySelector<HTMLButtonElement>('.edit-btn')!;
    btn.click();

    expect(events).to.have.lengthOf(1);
    expect(events[0].detail).to.deep.equal({ name: 'Notification volume/style' });
  });

  it('edit-requested event bubbles and is composed', async () => {
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control
        .setting=${SAMPLE_SETTING}
        viewer-can-edit
      ></elohim-imagodei-setting-control>
    `);
    let captured: CustomEvent | null = null;
    el.addEventListener('edit-requested', e => {
      captured = e as CustomEvent;
    });

    const btn = el.shadowRoot!.querySelector<HTMLButtonElement>('.edit-btn')!;
    btn.click();

    expect(captured).to.not.be.null;
    expect((captured as unknown as CustomEvent).bubbles).to.be.true;
    expect((captured as unknown as CustomEvent).composed).to.be.true;
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control
        .setting=${SAMPLE_SETTING}
        viewer-can-edit
      ></elohim-imagodei-setting-control>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-imagodei-setting-control> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (setting-control is still; no motion to gate)', () => {
    const cssText = (
      ElohimImagodeiSettingControlClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiSettingControl>(html`
      <elohim-imagodei-setting-control .setting=${SAMPLE_SETTING}></elohim-imagodei-setting-control>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-imagodei-setting-control> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiSettingControl>(
      'he-IL',
      html`
        <elohim-imagodei-setting-control
          .setting=${SAMPLE_SETTING}
        ></elohim-imagodei-setting-control>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const row = el.shadowRoot!.querySelector('.row')!;
    const rect = row.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiSettingControlClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
