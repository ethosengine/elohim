import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiConsentCard as ElohimImagodeiConsentCardClass } from './elohim-imagodei-consent-card.js';
import type { ClaimRef, ElohimImagodeiConsentCard } from './elohim-imagodei-consent-card.js';

const CLAIMS: ClaimRef[] = [
  {
    id: 'imagodei.displayName',
    label: 'Your display name',
    description: 'How you appear in the app.',
  },
  {
    id: 'qahal.standing',
    label: 'Your qahal standing',
    description: 'Capability tier within your community.',
  },
];

// ---------------------------------------------------------------------------
// Core contract tests
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-consent-card>', () => {
  it('renders client brand strip', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'graphos-designer', displayName: 'Graphos Designer' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    expect(el.shadowRoot!.querySelector('[part="client-name"]')?.textContent).to.include(
      'Graphos Designer'
    );
  });

  it('renders each requested claim as a toggleable row', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    const rows = el.shadowRoot!.querySelectorAll('[part="claim-row"]');
    expect(rows.length).to.equal(2);
  });

  it('locks required claims on and disables their toggle', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    const required = el.shadowRoot!.querySelector(
      '[part="claim-row"][data-claim-id="imagodei.displayName"] input'
    ) as HTMLInputElement;
    expect(required.checked).to.equal(true);
    expect(required.disabled).to.equal(true);
  });

  it('emits approve with granted claim ids (all on by default)', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    let detail: { grantedClaims: string[] } | null = null;
    el.addEventListener('approve', (e) => {
      detail = (e as CustomEvent<{ grantedClaims: string[] }>).detail;
    });
    await el.updateComplete;
    (el.shadowRoot!.querySelector<HTMLElement>('[part="approve"]'))!.click();
    expect(detail!.grantedClaims).to.deep.equal(['imagodei.displayName', 'qahal.standing']);
  });

  it('emits decline with user-rejected reason on Decline click', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    let detail: { reason: string } | null = null;
    el.addEventListener('decline', (e) => {
      detail = (e as CustomEvent<{ reason: string }>).detail;
    });
    (el.shadowRoot!.querySelector<HTMLElement>('[part="decline"]'))!.click();
    expect(detail!.reason).to.equal('user-rejected');
  });

  it('toggling an optional claim off excludes it from grantedClaims', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    await el.updateComplete;
    const optionalToggle = el.shadowRoot!.querySelector(
      '[part="claim-row"][data-claim-id="qahal.standing"] input'
    ) as HTMLInputElement;
    optionalToggle.checked = false;
    optionalToggle.dispatchEvent(new Event('change'));
    await el.updateComplete;
    let detail: { grantedClaims: string[] } | null = null;
    el.addEventListener('approve', (e) => {
      detail = (e as CustomEvent<{ grantedClaims: string[] }>).detail;
    });
    (el.shadowRoot!.querySelector<HTMLElement>('[part="approve"]'))!.click();
    expect(detail!.grantedClaims).to.deep.equal(['imagodei.displayName']);
  });

  it('defense-in-depth: required claim toggled off converts approve to partial-decline-blocked', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    await el.updateComplete;
    // Directly manipulate internal state to simulate DOM bypass
    (el as unknown as { _granted: Set<string> })._granted = new Set(['qahal.standing']);
    let declineDetail: { reason: string } | null = null;
    let approveDetail: { grantedClaims: string[] } | null = null;
    el.addEventListener('decline', (e) => {
      declineDetail = (e as CustomEvent<{ reason: string }>).detail;
    });
    el.addEventListener('approve', (e) => {
      approveDetail = (e as CustomEvent<{ grantedClaims: string[] }>).detail;
    });
    (el.shadowRoot!.querySelector<HTMLElement>('[part="approve"]'))!.click();
    expect(approveDetail).to.be.null;
    expect(declineDetail!.reason).to.equal('partial-decline-blocked');
  });

  it('renders rp part with client displayName and RP-mark', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'myapp', displayName: 'My App', brandMark: 'https://example.com/logo.png' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    expect(el.shadowRoot!.querySelector('[part="rp"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="rp-mark"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="client-name"]')!.textContent).to.include('My App');
  });

  it('optional claims are checked by default', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    await el.updateComplete;
    const optionalToggle = el.shadowRoot!.querySelector(
      '[part="claim-row"][data-claim-id="qahal.standing"] input'
    ) as HTMLInputElement;
    expect(optionalToggle.checked).to.equal(true);
  });

  it('renders policy-link slot', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      >
        <a slot="policy-link" href="/policy">Privacy Policy</a>
      </elohim-imagodei-consent-card>
    `);
    const slot = el.shadowRoot!.querySelector('slot[name="policy-link"]') as HTMLSlotElement;
    expect(slot).to.exist;
    expect(slot.assignedElements().some((n) => (n as HTMLElement).tagName === 'A')).to.equal(true);
  });

  it('approve and decline buttons exist', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${[]}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    expect(el.shadowRoot!.querySelector('[part="approve"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="decline"]')).to.exist;
  });

  it('each claim row carries data-claim-id attribute', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    const rows = el.shadowRoot!.querySelectorAll('[part="claim-row"]');
    const ids = Array.from(rows).map((r) => r.getAttribute('data-claim-id'));
    expect(ids).to.deep.equal(['imagodei.displayName', 'qahal.standing']);
  });

  it('toggling claim ON re-adds it to grantedClaims', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    await el.updateComplete;
    // Toggle off
    const toggle = el.shadowRoot!.querySelector(
      '[part="claim-row"][data-claim-id="qahal.standing"] input'
    ) as HTMLInputElement;
    toggle.checked = false;
    toggle.dispatchEvent(new Event('change'));
    await el.updateComplete;
    // Toggle back on
    toggle.checked = true;
    toggle.dispatchEvent(new Event('change'));
    await el.updateComplete;
    let detail: { grantedClaims: string[] } | null = null;
    el.addEventListener('approve', (e) => {
      detail = (e as CustomEvent<{ grantedClaims: string[] }>).detail;
    });
    (el.shadowRoot!.querySelector<HTMLElement>('[part="approve"]'))!.click();
    expect(detail!.grantedClaims).to.include('qahal.standing');
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-consent-card> — a11y precondition gate', () => {
  it('passes axe accessibility audit with required + optional claims', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'Graphos Designer' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit with no claims', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${[]}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('approve and decline buttons are keyboard-focusable', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    const approve = el.shadowRoot!.querySelector<HTMLButtonElement>('[part="approve"]')!;
    const decline = el.shadowRoot!.querySelector<HTMLButtonElement>('[part="decline"]')!;
    expect(approve.tabIndex).to.be.greaterThanOrEqual(0);
    expect(decline.tabIndex).to.be.greaterThanOrEqual(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-consent-card> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions and animations (still by default)', () => {
    const cssText = (
      ElohimImagodeiConsentCardClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
    expect(cssText).to.not.contain('animation:');
  });

  it('CSS has a forced-colors override block', () => {
    const cssText = (
      ElohimImagodeiConsentCardClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-consent-card> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiConsentCard>(
      'he-IL',
      html`
        <elohim-imagodei-consent-card
          .requestingClient=${{ id: 'g', displayName: 'Graphos Designer' }}
          .requestedClaims=${CLAIMS}
          .requiredClaims=${['imagodei.displayName']}
        ></elohim-imagodei-consent-card>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const card = el.shadowRoot!.querySelector('.card')!;
    const rect = card.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiConsentCardClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
