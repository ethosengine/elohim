import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiLoginCard as ElohimImagodeiLoginCardClass } from './elohim-imagodei-login-card.js';
import type { ElohimImagodeiLoginCard, OAuthProviderRef } from './elohim-imagodei-login-card.js';

const PROVIDERS: OAuthProviderRef[] = [
  { id: 'google', displayName: 'Google' },
  { id: 'github', displayName: 'GitHub' },
];

// ---------------------------------------------------------------------------
// Core contract tests — password form
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-login-card>', () => {
  it('renders password form when allowPassword is true', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    expect(el.shadowRoot!.querySelector('input[type="password"]')).to.exist;
  });

  it('does NOT render password form when allowPassword is false', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${false}></elohim-imagodei-login-card>
    `);
    expect(el.shadowRoot!.querySelector('input[type="password"]')).to.not.exist;
  });

  it('renders OAuth buttons when providers are present', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .oauthProviders=${PROVIDERS}></elohim-imagodei-login-card>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('[part="oauth-button"]');
    expect(buttons.length).to.equal(2);
  });

  it('each OAuth button carries the correct data-provider attribute', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .oauthProviders=${PROVIDERS}></elohim-imagodei-login-card>
    `);
    const google = el.shadowRoot!.querySelector('[part="oauth-button"][data-provider="google"]');
    const github = el.shadowRoot!.querySelector('[part="oauth-button"][data-provider="github"]');
    expect(google).to.exist;
    expect(github).to.exist;
  });

  it('renders no OAuth buttons when oauthProviders is empty', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .oauthProviders=${[]}></elohim-imagodei-login-card>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('[part="oauth-button"]');
    expect(buttons.length).to.equal(0);
  });

  it('emits password-submit on form submission', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .rememberedIdentifier=${'matthew@alpha'}
      ></elohim-imagodei-login-card>
    `);
    let detail: any = null;
    el.addEventListener('password-submit', e => {
      detail = (e as CustomEvent).detail;
    });
    const pw = el.shadowRoot!.querySelector('input[type="password"]') as HTMLInputElement;
    pw.value = 'secret';
    pw.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise(r => setTimeout(r, 10));
    expect(detail.identifier).to.equal('matthew@alpha');
    expect(detail.password).to.equal('secret');
  });

  it('password-submit detail includes remember flag', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .remember=${true}
      ></elohim-imagodei-login-card>
    `);
    let detail: any = null;
    el.addEventListener('password-submit', e => {
      detail = (e as CustomEvent).detail;
    });
    const pw = el.shadowRoot!.querySelector('input[type="password"]') as HTMLInputElement;
    pw.value = 'pass1';
    pw.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise(r => setTimeout(r, 10));
    expect(detail.remember).to.equal(true);
  });

  it('submit button is disabled when password field is empty', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    const submit = el.shadowRoot!.querySelector('button[type="submit"]') as HTMLButtonElement;
    expect(submit.disabled).to.equal(true);
  });

  it('submit button becomes enabled when password is entered', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    const pw = el.shadowRoot!.querySelector('input[type="password"]') as HTMLInputElement;
    pw.value = 'somepass';
    pw.dispatchEvent(new Event('input'));
    await el.updateComplete;
    const submit = el.shadowRoot!.querySelector('button[type="submit"]') as HTMLButtonElement;
    expect(submit.disabled).to.equal(false);
  });

  it('emits oauth-start with providerId on OAuth button click', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .oauthProviders=${PROVIDERS}></elohim-imagodei-login-card>
    `);
    let detail: any = null;
    el.addEventListener('oauth-start', e => {
      detail = (e as CustomEvent).detail;
    });
    el.shadowRoot!.querySelector<HTMLElement>(
      '[part="oauth-button"][data-provider="github"]'
    )!.click();
    expect(detail.providerId).to.equal('github');
  });

  it('emits oauth-start with correct providerId for each provider', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .oauthProviders=${PROVIDERS}></elohim-imagodei-login-card>
    `);
    const events: string[] = [];
    el.addEventListener('oauth-start', e => {
      events.push((e as CustomEvent).detail.providerId);
    });
    el.shadowRoot!.querySelector<HTMLElement>(
      '[part="oauth-button"][data-provider="google"]'
    )!.click();
    el.shadowRoot!.querySelector<HTMLElement>(
      '[part="oauth-button"][data-provider="github"]'
    )!.click();
    expect(events).to.deep.equal(['google', 'github']);
  });

  it('renders unlock-prompt slot when provided (Tauri path)', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}>
        <div slot="unlock-prompt" id="tauri-unlock">unlock your local key</div>
      </elohim-imagodei-login-card>
    `);
    const slot = el.shadowRoot!.querySelector('slot[name="unlock-prompt"]') as HTMLSlotElement;
    expect(slot).to.exist;
    expect(slot.assignedElements().some(n => (n as HTMLElement).id === 'tauri-unlock')).to.equal(
      true
    );
  });

  it('adapts copy based on trustMode — doorway-host vs peer-conductor', async () => {
    const elHost = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .trustMode=${'doorway-host'}
      ></elohim-imagodei-login-card>
    `);
    const elPeer = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .trustMode=${'peer-conductor'}
      ></elohim-imagodei-login-card>
    `);
    const hostLabel = elHost.shadowRoot!.querySelector('label[for="pw"]')?.textContent ?? '';
    const peerLabel = elPeer.shadowRoot!.querySelector('label[for="pw"]')?.textContent ?? '';
    expect(hostLabel).to.not.equal(peerLabel);
    // peer copy must reference unlock or conductor
    expect(peerLabel.toLowerCase()).to.match(/unlock|conductor/);
  });

  it('doorway-host label references sign in', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .trustMode=${'doorway-host'}
      ></elohim-imagodei-login-card>
    `);
    const label = el.shadowRoot!.querySelector('label[for="pw"]')?.textContent ?? '';
    expect(label.toLowerCase()).to.match(/sign in|password/);
  });

  it('setError() surfaces error message via aria-live region', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    el.setError('credentials did not match');
    await el.updateComplete;
    const err = el.shadowRoot!.querySelector('[part="error"]');
    expect(err).to.exist;
    expect(err?.textContent).to.include('credentials did not match');
  });

  it('setError() region has aria-live="polite"', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    el.setError('something went wrong');
    await el.updateComplete;
    const err = el.shadowRoot!.querySelector('[part="error"]');
    expect(err?.getAttribute('aria-live')).to.equal('polite');
  });

  it('setError(null) clears the error region', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    el.setError('temporary error');
    await el.updateComplete;
    el.setError(null);
    await el.updateComplete;
    expect(el.shadowRoot!.querySelector('[part="error"]')).to.not.exist;
  });

  it('password input has aria-invalid=false in default state', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    const pw = el.shadowRoot!.querySelector('input[type="password"]') as HTMLInputElement;
    expect(pw.getAttribute('aria-invalid')).to.equal('false');
  });

  it('password input has aria-invalid=true after setError', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    el.setError('bad credentials');
    await el.updateComplete;
    const pw = el.shadowRoot!.querySelector('input[type="password"]') as HTMLInputElement;
    expect(pw.getAttribute('aria-invalid')).to.equal('true');
  });

  it('password input has autocomplete=current-password', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    const pw = el.shadowRoot!.querySelector('input[type="password"]') as HTMLInputElement;
    expect(pw.getAttribute('autocomplete')).to.equal('current-password');
  });

  it('renders a divider when both allowPassword and oauthProviders are set', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .oauthProviders=${PROVIDERS}
      ></elohim-imagodei-login-card>
    `);
    const divider = el.shadowRoot!.querySelector('.divider');
    expect(divider).to.exist;
  });

  it('does NOT render a divider when allowPassword is false', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${false}
        .oauthProviders=${PROVIDERS}
      ></elohim-imagodei-login-card>
    `);
    const divider = el.shadowRoot!.querySelector('.divider');
    expect(divider).to.not.exist;
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-login-card> — a11y precondition gate', () => {
  it('passes axe accessibility audit in default state (password form)', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit with OAuth providers', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .oauthProviders=${PROVIDERS}
      ></elohim-imagodei-login-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit in error state', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    el.setError('credentials did not match');
    await el.updateComplete;
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit in peer-conductor mode', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card
        .allowPassword=${true}
        .trustMode=${'peer-conductor'}
      ></elohim-imagodei-login-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-login-card> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions and animations (still by default)', () => {
    const cssText = (ElohimImagodeiLoginCardClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).to.not.contain('transition:');
    expect(cssText).to.not.contain('animation:');
  });

  it('CSS has a forced-colors override block', () => {
    const cssText = (ElohimImagodeiLoginCardClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-login-card> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiLoginCard>(
      'he-IL',
      html`
        <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const form = el.shadowRoot!.querySelector('form')!;
    const rect = form.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (ElohimImagodeiLoginCardClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
