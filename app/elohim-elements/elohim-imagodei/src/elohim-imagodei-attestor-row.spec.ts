import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiAttestorRow as ElohimImagodeiAttestorRowClass } from './elohim-imagodei-attestor-row.js';
import type { AttestorRef, ElohimImagodeiAttestorRow } from './elohim-imagodei-attestor-row.js';

const SAMPLE: AttestorRef[] = [
  { eprRef: 'epr:human-susan', displayName: 'Susan', role: 'qahal-elder' },
  { eprRef: 'epr:human-james', displayName: 'James', role: 'intimate-circle' },
  { eprRef: 'epr:human-marta', displayName: 'Marta', role: 'recovery-witness' },
];

// ---------------------------------------------------------------------------
// Core contract tests
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-attestor-row>', () => {
  it('renders one avatar per attestor up to maxVisible', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    const avatars = el.shadowRoot!.querySelectorAll('[part="avatar"]');
    expect(avatars.length).to.equal(3);
  });

  it('renders +N overflow when attestors exceed maxVisible', async () => {
    const many: AttestorRef[] = Array.from({ length: 8 }, (_, i) => ({
      eprRef: `epr:human-${i}`,
      displayName: `Human ${i}`,
      role: 'qahal-elder',
    }));
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row
        .attestors=${many}
        .maxVisible=${5}
      ></elohim-imagodei-attestor-row>
    `);
    const avatars = el.shadowRoot!.querySelectorAll('[part="avatar"]');
    expect(avatars.length).to.equal(5);
    expect(el.shadowRoot!.querySelector('[part="overflow"]')?.textContent).to.include('+3');
  });

  it('renders no overflow when attestors equals maxVisible', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row
        .attestors=${SAMPLE}
        .maxVisible=${3}
      ></elohim-imagodei-attestor-row>
    `);
    expect(el.shadowRoot!.querySelector('[part="overflow"]')).to.be.null;
  });

  it('renders empty slot when attestors is empty', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${[]}>
        <span slot="empty">no witnesses yet</span>
      </elohim-imagodei-attestor-row>
    `);
    const emptySlot = el.shadowRoot!.querySelector('slot[name="empty"]') as HTMLSlotElement;
    expect(emptySlot).to.exist;
  });

  it('emits attestor-tap with eprRef on avatar click', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    let detail: { eprRef?: string } | null = null;
    el.addEventListener('attestor-tap', e => {
      detail = (e as CustomEvent).detail;
    });
    (el.shadowRoot!.querySelectorAll<HTMLElement>('[part="avatar"]')[1]).click();
    expect(detail?.eprRef).to.equal('epr:human-james');
  });

  it('emits attestor-tap with the correct eprRef for first and last avatars', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    const avatars = el.shadowRoot!.querySelectorAll<HTMLElement>('[part="avatar"]');

    let firstDetail: { eprRef?: string } | null = null;
    el.addEventListener('attestor-tap', e => {
      firstDetail = (e as CustomEvent).detail;
    });
    avatars[0].click();
    expect(firstDetail?.eprRef).to.equal('epr:human-susan');

    let lastDetail: { eprRef?: string } | null = null;
    el.addEventListener('attestor-tap', e => {
      lastDetail = (e as CustomEvent).detail;
    });
    avatars[2].click();
    expect(lastDetail?.eprRef).to.equal('epr:human-marta');
  });

  it('avatar title attribute carries displayName and role', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    const first = el.shadowRoot!.querySelectorAll<HTMLElement>('[part="avatar"]')[0];
    const title = first.getAttribute('title') ?? '';
    expect(title).to.include('Susan');
    expect(title).to.include('qahal-elder');
  });

  it('defaults maxVisible to 5', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row></elohim-imagodei-attestor-row>
    `);
    expect(el.maxVisible).to.equal(5);
  });

  it('renders each avatar as a button for keyboard interaction', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    const avatars = el.shadowRoot!.querySelectorAll('[part="avatar"]');
    for (const avatar of avatars) {
      expect(avatar.tagName.toLowerCase()).to.equal('button');
      expect(avatar.getAttribute('type')).to.equal('button');
    }
  });

  it('passes axe accessibility audit with attestors', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit in empty state', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${[]}>
        <span slot="empty">No witnesses yet</span>
      </elohim-imagodei-attestor-row>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-attestor-row> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (element is still; no motion to gate)', () => {
    const cssText = (
      ElohimImagodeiAttestorRowClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
    expect(cssText).to.not.contain('animation:');
  });

  it('CSS has forced-colors override block', () => {
    const cssText = (
      ElohimImagodeiAttestorRowClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-attestor-row> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiAttestorRow>(
      'he-IL',
      html`
        <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const row = el.shadowRoot!.querySelector('[part="row"]')!;
    const rect = row.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiAttestorRowClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
