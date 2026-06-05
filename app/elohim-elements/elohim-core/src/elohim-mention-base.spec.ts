import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimMentionBase } from './elohim-mention-base.js';
import { ElohimMentionBase as ElohimMentionBaseClass } from './elohim-mention-base.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

describe('<elohim-mention-base>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-mention-base')).to.equal(ElohimMentionBaseClass);
  });

  it('renders with default (empty) prop values', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base></elohim-mention-base>
    `);
    expect(el).to.exist;
    expect(el.epr).to.equal('');
    expect(el.label).to.equal('');
    expect(el.pillar).to.equal('');
  });

  it('renders the epr as the visible label when no label is provided', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base epr="epr:content:abc123"></elohim-mention-base>
    `);
    const labelSpan = el.shadowRoot!.querySelector('.label');
    expect(labelSpan).to.exist;
    expect(labelSpan!.textContent!.trim()).to.equal('epr:content:abc123');
  });

  it('renders the label when provided', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base
        epr="epr:content:abc123"
        label="Introduction to Lamad"
      ></elohim-mention-base>
    `);
    const labelSpan = el.shadowRoot!.querySelector('.label');
    expect(labelSpan!.textContent!.trim()).to.equal('Introduction to Lamad');
  });

  it('renders the epr id span alongside label when label is provided', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base
        epr="epr:content:abc123"
        label="Introduction to Lamad"
      ></elohim-mention-base>
    `);
    const idSpan = el.shadowRoot!.querySelector('.id');
    expect(idSpan).to.exist;
    expect(idSpan!.textContent!.trim()).to.equal('epr:content:abc123');
  });

  it('omits the epr id span when no label is provided', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base epr="epr:content:abc123"></elohim-mention-base>
    `);
    const idSpan = el.shadowRoot!.querySelector('.id');
    expect(idSpan).to.be.null;
  });

  it('renders the fallback-mark dot', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base epr="epr:content:abc123"></elohim-mention-base>
    `);
    const mark = el.shadowRoot!.querySelector('.fallback-mark');
    expect(mark).to.exist;
  });

  it('updates label display reactively', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base epr="epr:content:abc123"></elohim-mention-base>
    `);
    el.label = 'Updated label';
    await elementUpdated(el);
    const labelSpan = el.shadowRoot!.querySelector('.label');
    expect(labelSpan!.textContent!.trim()).to.equal('Updated label');
  });

  it('renders empty chip gracefully when both epr and title are empty (empty state)', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base></elohim-mention-base>
    `);
    // The chip container should still render, title span shows empty string
    const chip = el.shadowRoot!.querySelector('.chip');
    expect(chip).to.exist;
  });

  it('exposes chip, fallback-mark, label, id as named parts', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base epr="epr:content:abc123" label="Test"></elohim-mention-base>
    `);
    expect(el.shadowRoot!.querySelector('[part="chip"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="fallback-mark"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="label"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="id"]')).to.exist;
  });

  it('passes axe-core a11y scan with epr only', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base epr="epr:content:abc123"></elohim-mention-base>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core a11y scan with label and epr', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base
        epr="epr:content:abc123"
        label="Introduction to Lamad"
      ></elohim-mention-base>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core a11y scan in empty state', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base></elohim-mention-base>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-mention-base> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('forced-colors overrides use CSS system colors (Canvas, CanvasText, ButtonText)', () => {
    const cssText = (
      ElohimMentionBaseClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    // eslint-disable-next-line unicorn/prefer-set-has -- string scan, not membership lookup
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('Canvas') ||
      afterForced.includes('CanvasText') ||
      afterForced.includes('ButtonText');
    expect(hasSystemColor).to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimMentionBase>(html`
      <elohim-mention-base epr="epr:content:abc123" label="Test"></elohim-mention-base>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-mention-base> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimMentionBase>(
      'he-IL',
      html`
        <elohim-mention-base epr="epr:content:abc123" label="מבוא ללמד"></elohim-mention-base>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const chip = el.shadowRoot!.querySelector('.chip')!;
    const rect = chip.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimMentionBaseClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-mention-base> — theme-contrast gate', () => {
  // tokens cells: no shipped binding for this element yet — system cells only
  // (theme-authority spec §4.1). The blank-slate contract per scheme.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimMentionBase>(
        html`
          <elohim-mention-base
            epr="epr:content:abc123"
            label="Introduction to Lamad"
          ></elohim-mention-base>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});
