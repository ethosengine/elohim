import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiFederatedResolver as ElohimImagodeiFederatedResolverClass } from './elohim-imagodei-federated-resolver.js';
import type {
  ElohimImagodeiFederatedResolver,
  FederatedResolveOutcome,
} from './elohim-imagodei-federated-resolver.js';

// ---------------------------------------------------------------------------
// Core contract tests — localStorage + async resolver callback
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-federated-resolver>', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('emits resolved with identifier + doorwayUrl on successful resolve', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (id: string): Promise<FederatedResolveOutcome> => ({
          ok: true,
          doorwayUrl: `https://${id.split('@')[1]}`,
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    let detail: unknown = null;
    el.addEventListener('resolved', (e) => {
      detail = (e as CustomEvent).detail;
    });
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'matthew@alpha.elohim.host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(detail).to.deep.equal({
      identifier: 'matthew@alpha.elohim.host',
      doorwayUrl: 'https://alpha.elohim.host',
    });
  });

  it('emits resolve-error with reason on failed resolve', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (): Promise<FederatedResolveOutcome> => ({
          ok: false,
          reason: 'unknown-host',
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    let detail: unknown = null;
    el.addEventListener('resolve-error', (e) => {
      detail = (e as CustomEvent).detail;
    });
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'someone@nowhere';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(detail).to.deep.equal({ identifier: 'someone@nowhere', reason: 'unknown-host' });
  });

  it('persists identifier to localStorage on successful resolve', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        remember-key="test-id-key"
        .resolveIdentifier=${async (): Promise<FederatedResolveOutcome> => ({
          ok: true,
          doorwayUrl: 'https://x',
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'matthew@alpha.elohim.host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(localStorage.getItem('test-id-key')).to.equal('matthew@alpha.elohim.host');
  });

  it('pre-fills input from localStorage on mount', async () => {
    localStorage.setItem('test-id-key', 'previously@alpha.elohim.host');
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        remember-key="test-id-key"
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    expect(input.value).to.equal('previously@alpha.elohim.host');
  });

  it('does NOT persist identifier when resolve fails', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        remember-key="test-id-key"
        .resolveIdentifier=${async (): Promise<FederatedResolveOutcome> => ({
          ok: false,
          reason: 'failed',
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'bad@host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(localStorage.getItem('test-id-key')).to.be.null;
  });

  it('disables submit while busy', async () => {
    let resolveFn: ((v: FederatedResolveOutcome) => void) | null = null;
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${() =>
          new Promise<FederatedResolveOutcome>((r) => {
            resolveFn = r;
          })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'matthew@alpha.elohim.host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await el.updateComplete;
    const submit = el.shadowRoot!.querySelector('button[type="submit"]') as HTMLButtonElement;
    expect(submit.disabled).to.equal(true);
    resolveFn!({ ok: true, doorwayUrl: 'https://x' });
    await new Promise((r) => setTimeout(r, 10));
    expect(submit.disabled).to.equal(false);
  });

  it('submit button is disabled when input is empty', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver></elohim-imagodei-federated-resolver>
    `);
    const submit = el.shadowRoot!.querySelector('button[type="submit"]') as HTMLButtonElement;
    expect(submit.disabled).to.equal(true);
  });

  it('input has aria-invalid=false in default state', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    expect(input.getAttribute('aria-invalid')).to.equal('false');
  });

  it('input has aria-invalid=true after a failed resolve', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (): Promise<FederatedResolveOutcome> => ({
          ok: false,
          reason: 'unknown-host',
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'bad@host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    await el.updateComplete;
    expect(input.getAttribute('aria-invalid')).to.equal('true');
  });

  it('shows error message in an alert role element after failed resolve', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (): Promise<FederatedResolveOutcome> => ({
          ok: false,
          reason: 'not-found',
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'unknown@host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    await el.updateComplete;
    const alert = el.shadowRoot!.querySelector('[role="alert"]');
    expect(alert).to.exist;
    expect(alert!.textContent).to.include('not-found');
  });

  it('uses default rememberKey=elohim_auth_identifier when not specified', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (): Promise<FederatedResolveOutcome> => ({
          ok: true,
          doorwayUrl: 'https://alpha.elohim.host',
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'matthew@alpha.elohim.host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(localStorage.getItem('elohim_auth_identifier')).to.equal('matthew@alpha.elohim.host');
  });

  it('renders a form element as the root', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver></elohim-imagodei-federated-resolver>
    `);
    expect(el.shadowRoot!.querySelector('form')).to.exist;
  });

  it('renders an input with autocomplete=username', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    expect(input.getAttribute('autocomplete')).to.equal('username');
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-federated-resolver> — a11y precondition gate', () => {
  it('passes axe accessibility audit in default (empty) state', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver></elohim-imagodei-federated-resolver>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit in error state', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (): Promise<FederatedResolveOutcome> => ({
          ok: false,
          reason: 'host-unreachable',
        })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'matthew@unreachable.host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    await el.updateComplete;
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-federated-resolver> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions and animations (still by default)', () => {
    const cssText = (
      ElohimImagodeiFederatedResolverClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
    expect(cssText).to.not.contain('animation:');
  });

  it('CSS has a forced-colors override block', () => {
    const cssText = (
      ElohimImagodeiFederatedResolverClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver></elohim-imagodei-federated-resolver>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-federated-resolver> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiFederatedResolver>(
      'he-IL',
      html`
        <elohim-imagodei-federated-resolver></elohim-imagodei-federated-resolver>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const form = el.shadowRoot!.querySelector('form')!;
    const rect = form.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiFederatedResolverClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
