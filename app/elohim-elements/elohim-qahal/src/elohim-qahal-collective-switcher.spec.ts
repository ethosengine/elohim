import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalCollectiveSwitcher as ElohimQahalCollectiveSwitcherClass } from './elohim-qahal-collective-switcher.js';
import type {
  CollectiveDescriptor,
  ElohimQahalCollectiveSwitcher,
} from './elohim-qahal-collective-switcher.js';

const SAMPLE_COLLECTIVES: CollectiveDescriptor[] = [
  { id: 'alpha', icon: '🌱', name: 'Alpha Collective' },
  { id: 'beta', icon: '🔥', name: 'Beta Collective' },
  { id: 'gamma', icon: '💧', name: 'Gamma Collective' },
];

describe('<elohim-qahal-collective-switcher>', () => {
  it('renders an icon button for each collective', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${SAMPLE_COLLECTIVES}
        active-collective-id="alpha"
      ></elohim-qahal-collective-switcher>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons).to.have.lengthOf(3);
  });

  it('renders no buttons when collectives is empty', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher></elohim-qahal-collective-switcher>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons).to.have.lengthOf(0);
  });

  it('marks the active collective with aria-pressed=true', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${SAMPLE_COLLECTIVES}
        active-collective-id="beta"
      ></elohim-qahal-collective-switcher>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons[0].getAttribute('aria-pressed')).to.equal('false');
    expect(buttons[1].getAttribute('aria-pressed')).to.equal('true');
    expect(buttons[2].getAttribute('aria-pressed')).to.equal('false');
  });

  it('gives each button an aria-label matching the collective name', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${SAMPLE_COLLECTIVES}
        active-collective-id="alpha"
      ></elohim-qahal-collective-switcher>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons[0].getAttribute('aria-label')).to.equal('Alpha Collective');
    expect(buttons[1].getAttribute('aria-label')).to.equal('Beta Collective');
    expect(buttons[2].getAttribute('aria-label')).to.equal('Gamma Collective');
  });

  it('emits collective-changed event with correct id when a button is clicked', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${SAMPLE_COLLECTIVES}
        active-collective-id="alpha"
      ></elohim-qahal-collective-switcher>
    `);
    const events: CustomEvent[] = [];
    el.addEventListener('collective-changed', e => events.push(e as CustomEvent));

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[1] as HTMLButtonElement).click();

    expect(events).to.have.lengthOf(1);
    expect(events[0].detail).to.deep.equal({ id: 'beta' });
  });

  it('collective-changed event bubbles and is composed', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${SAMPLE_COLLECTIVES}
        active-collective-id="alpha"
      ></elohim-qahal-collective-switcher>
    `);
    let captured: CustomEvent | null = null;
    el.addEventListener('collective-changed', e => {
      captured = e as CustomEvent;
    });

    const buttons = el.shadowRoot!.querySelectorAll('button');
    (buttons[2] as HTMLButtonElement).click();

    expect(captured).to.not.be.null;
    expect((captured as unknown as CustomEvent).bubbles).to.be.true;
    expect((captured as unknown as CustomEvent).composed).to.be.true;
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${SAMPLE_COLLECTIVES}
        active-collective-id="alpha"
      ></elohim-qahal-collective-switcher>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-collective-switcher> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (switcher is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalCollectiveSwitcherClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${SAMPLE_COLLECTIVES}
        active-collective-id="alpha"
      ></elohim-qahal-collective-switcher>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-collective-switcher> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalCollectiveSwitcher>(
      'he-IL',
      html`
        <elohim-qahal-collective-switcher
          .collectives=${SAMPLE_COLLECTIVES}
          active-collective-id="alpha"
        ></elohim-qahal-collective-switcher>
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
      ElohimQahalCollectiveSwitcherClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
