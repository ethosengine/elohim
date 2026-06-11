import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiOauthCallback as ElohimImagodeiOauthCallbackClass } from './elohim-imagodei-oauth-callback.js';
import type {
  ElohimImagodeiOauthCallback,
  ExchangeOutcome,
} from './elohim-imagodei-oauth-callback.js';

// ---------------------------------------------------------------------------
// Core contract tests
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-oauth-callback>', () => {
  it('renders skeleton during code exchange', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () =>
          new Promise(() => {
            /* never resolves */
          })}
      ></elohim-imagodei-oauth-callback>
    `);
    await el.updateComplete;
    expect(el.shadowRoot!.querySelector('[part="skeleton"]')).to.exist;
  });

  it('emits success when exchange resolves', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async (): Promise<ExchangeOutcome> => ({ session: { humanId: 'matthew' } })}
      ></elohim-imagodei-oauth-callback>
    `);
    let detail: any = null;
    el.addEventListener('success', e => {
      detail = (e as CustomEvent).detail;
    });
    await new Promise(r => setTimeout(r, 20));
    expect(detail.session.humanId).to.equal('matthew');
  });

  it('emits error when exchange rejects', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () => {
          throw new Error('invalid_grant');
        }}
      ></elohim-imagodei-oauth-callback>
    `);
    let detail: any = null;
    el.addEventListener('error', e => {
      detail = (e as CustomEvent).detail;
    });
    await new Promise(r => setTimeout(r, 20));
    expect(detail.reason).to.include('invalid_grant');
  });

  it('does NOT auto-exchange when code is missing', async () => {
    let called = false;
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        state="xyz"
        .exchangeCode=${async (): Promise<ExchangeOutcome> => {
          called = true;
          return { session: {} };
        }}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    expect(called).to.equal(false);
  });

  it('does NOT auto-exchange when state is missing', async () => {
    let called = false;
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        .exchangeCode=${async (): Promise<ExchangeOutcome> => {
          called = true;
          return { session: {} };
        }}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    expect(called).to.equal(false);
  });

  it('runs exchange exactly once even if code/state set multiple times', async () => {
    let callCount = 0;
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async (): Promise<ExchangeOutcome> => {
          callCount += 1;
          return { session: {} };
        }}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    // Re-set the props to the same values; should NOT trigger another exchange
    el.code = 'abc';
    el.state = 'xyz';
    await el.updateComplete;
    await new Promise(r => setTimeout(r, 20));
    expect(callCount).to.equal(1);
  });

  it('renders error-detail slot when error state', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () => {
          throw new Error('invalid_grant');
        }}
      >
        <div slot="error-detail" id="err-detail">extra detail</div>
        <a slot="recovery-cta" id="recovery" href="/identity/recovery">recover</a>
      </elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    const errSlot = el.shadowRoot!.querySelector('slot[name="error-detail"]') as HTMLSlotElement;
    const recSlot = el.shadowRoot!.querySelector('slot[name="recovery-cta"]') as HTMLSlotElement;
    expect(errSlot?.assignedElements().some(n => (n as HTMLElement).id === 'err-detail')).to.equal(
      true
    );
    expect(recSlot?.assignedElements().some(n => (n as HTMLElement).id === 'recovery')).to.equal(
      true
    );
  });

  it('renders idle state when neither code nor state provided', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        .exchangeCode=${async (): Promise<ExchangeOutcome> => ({ session: {} })}
      ></elohim-imagodei-oauth-callback>
    `);
    await el.updateComplete;
    expect(el.shadowRoot!.querySelector('[part="idle"]')).to.exist;
  });

  it('renders done state after successful exchange', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async (): Promise<ExchangeOutcome> => ({ session: {} })}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    await el.updateComplete;
    expect(el.shadowRoot!.querySelector('[part="done"]')).to.exist;
  });

  it('includes providerLabel in exchanging message when set', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        provider-label="GitHub"
        .exchangeCode=${async () =>
          new Promise(() => {
            /* never resolves */
          })}
      ></elohim-imagodei-oauth-callback>
    `);
    await el.updateComplete;
    const message = el.shadowRoot!.querySelector('[part="exchanging-message"]');
    expect(message?.textContent).to.include('GitHub');
  });

  it('error state renders error part with role=alert', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () => {
          throw new Error('token_expired');
        }}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    await el.updateComplete;
    const errorEl = el.shadowRoot!.querySelector('[part="error"]');
    expect(errorEl).to.exist;
    expect(errorEl?.getAttribute('role')).to.equal('alert');
  });

  it('skeleton has aria-busy=true during exchange', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () =>
          new Promise(() => {
            /* never resolves */
          })}
      ></elohim-imagodei-oauth-callback>
    `);
    await el.updateComplete;
    const skeleton = el.shadowRoot!.querySelector('[part="skeleton"]');
    expect(skeleton?.getAttribute('aria-busy')).to.equal('true');
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-oauth-callback> — a11y precondition gate', () => {
  it('passes axe audit in idle state (no code/state)', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        .exchangeCode=${async (): Promise<ExchangeOutcome> => ({ session: {} })}
      ></elohim-imagodei-oauth-callback>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe audit in exchanging state', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () =>
          new Promise(() => {
            /* never resolves */
          })}
      ></elohim-imagodei-oauth-callback>
    `);
    await el.updateComplete;
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe audit in error state', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () => {
          throw new Error('invalid_grant');
        }}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe audit in done state', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async (): Promise<ExchangeOutcome> => ({ session: {} })}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise(r => setTimeout(r, 20));
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-oauth-callback> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS wraps skeleton animation in prefers-reduced-motion media query', () => {
    const cssText = (ElohimImagodeiOauthCallbackClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).to.contain('prefers-reduced-motion');
  });

  it('CSS has a forced-colors override block', () => {
    const cssText = (ElohimImagodeiOauthCallbackClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        .exchangeCode=${async (): Promise<ExchangeOutcome> => ({ session: {} })}
      ></elohim-imagodei-oauth-callback>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-oauth-callback> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiOauthCallback>(
      'he-IL',
      html`
        <elohim-imagodei-oauth-callback
          .exchangeCode=${async (): Promise<ExchangeOutcome> => ({ session: {} })}
        ></elohim-imagodei-oauth-callback>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const idle = el.shadowRoot!.querySelector('[part="idle"]')!;
    const rect = idle.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (ElohimImagodeiOauthCallbackClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
