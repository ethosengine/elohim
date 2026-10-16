/** @vitest-environment jsdom */
import '@analogjs/vitest-angular/setup-zone';
import '@angular/compiler';
import { getTestBed, TestBed } from '@angular/core/testing';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { AccountBindComponent, accountBindReturn } from './account-bind.component';
import { ACCOUNT_BIND_PORT } from './account-bind-port';
import { AppComponent } from '../app.component';
import { routes } from '../app.routes';
import * as helpers from 'elohim-imagodei';

const nonce = 'ab'.repeat(32);
const text = [
  'elohim:doorway-account-bind:v1',
  'audience=doorway.example',
  'identifier=élise@example.org',
  'human=human-1',
  'from=uhCAk-old',
  `nonce=${nonce}`,
  'expires=2099-10-16T20:15:00Z',
].join('\n');
const encode = (value: string) =>
  btoa(String.fromCharCode(...new TextEncoder().encode(value)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');

describe('account-bind portal', () => {
  beforeAll(() => getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting()));
  afterEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
  });
  const port = {
    standing: vi.fn(),
    hasSessionKey: vi.fn(),
    sign: vi.fn(),
    signIn: vi.fn(),
    navigate: vi.fn(),
  };
  beforeEach(() => {
    vi.resetAllMocks();
    port.standing.mockResolvedValue({ ok: true, body: {} });
    port.hasSessionKey.mockResolvedValue(true);
    port.sign.mockResolvedValue({
      ok: true,
      body: { agentPubKey: 'uhCAk-new', signature: '+/8=' },
    });
    setUrl();
  });
  function setUrl(returnTo = 'https://doorway.example/account#old', challenge = text) {
    const query = new URLSearchParams({
      challenge: encode(challenge),
      return: returnTo,
    });
    window.history.replaceState(null, '', '/auth/portal/consent/account-bind?' + query);
  }
  async function render() {
    TestBed.configureTestingModule({
      imports: [AccountBindComponent],
      providers: [{ provide: ACCOUNT_BIND_PORT, useValue: port }],
    });
    const fixture = TestBed.createComponent(AccountBindComponent);
    fixture.detectChanges();
    await new Promise(resolve => setTimeout(resolve, 0));
    await fixture.whenStable();
    fixture.detectChanges();
    return {
      fixture,
      root: fixture.nativeElement as HTMLElement,
      component: fixture.componentInstance,
    };
  }
  it('renders the audience, UTF-8 account, expiry and hosting consequence', async () => {
    const { root } = await render();
    expect(root.querySelector('[data-testid="account-bind-view"]')).not.toBeNull();
    expect(root.querySelector('[data-testid="account-bind-audience"]')?.textContent).toBe(
      'doorway.example'
    );
    expect(root.querySelector('[data-testid="account-bind-identifier"]')?.textContent).toBe(
      'élise@example.org'
    );
    expect(root.textContent).toContain('2099-10-16T20:15:00Z');
    expect(root.textContent).toContain('stop hosting a cell');
  });
  it('approves once, signs exact text and replaces the return fragment', async () => {
    const { root, fixture } = await render();
    (root.querySelector('[data-testid="account-bind-approve"]') as HTMLButtonElement).click();
    await new Promise(resolve => setTimeout(resolve, 0));
    await fixture.whenStable();
    expect(port.sign).toHaveBeenCalledExactlyOnceWith(text);
    expect(port.navigate).toHaveBeenCalledWith(
      `https://doorway.example/account#challengeId=${nonce}&agentPubKey=uhCAk-new&signature=-_8`
    );
  });
  it('declines without signing', async () => {
    const { root } = await render();
    (root.querySelector('[data-testid="account-bind-decline"]') as HTMLButtonElement).click();
    expect(port.sign).not.toHaveBeenCalled();
    expect(port.navigate).toHaveBeenCalledWith('https://doorway.example/account#refused=DECLINED');
  });
  it.each([
    'http://doorway.example/account',
    'https://evil.example/account',
    'javascript:alert(1)',
    '/account',
  ])('refuses unsafe return %s without navigating or signing', async url => {
    setUrl(url);
    const { root, component } = await render();
    expect(root.querySelector('[data-testid="account-bind-error"]')?.textContent).toContain(
      'INVALID_RETURN_URL'
    );
    await component.approve();
    component.decline();
    expect(port.sign).not.toHaveBeenCalled();
    expect(port.navigate).not.toHaveBeenCalled();
    expect(port.standing).not.toHaveBeenCalled();
  });
  it('sends a signed-out person through the portal sign-in first', async () => {
    port.standing.mockResolvedValue({ ok: false, status: 401, body: {} });
    await render();
    expect(port.signIn).toHaveBeenCalledOnce();
    expect(port.sign).not.toHaveBeenCalled();
  });
  it('asks for sign-in again when the browser lost its session key', async () => {
    port.hasSessionKey.mockResolvedValue(false);
    await render();
    expect(port.signIn).toHaveBeenCalledOnce();
    expect(port.sign).not.toHaveBeenCalled();
  });
  it.each([
    'NO_SESSION',
    'SESSION_NOT_BOUND',
    'PROOF_REFUSED',
    'NOT_A_BIND_CHALLENGE',
    'CHALLENGE_EXPIRED',
    'NO_CONDUCTOR',
  ])('returns the node refusal %s', async code => {
    port.sign.mockResolvedValue({
      ok: false,
      status: 403,
      body: { error: 'Refused', code },
    });
    const { component } = await render();
    await component.approve();
    expect(port.navigate).toHaveBeenCalledWith('https://doorway.example/account#refused=' + code);
  });
  it('returns expiry without signing', async () => {
    setUrl(undefined, text.replace('2099', '2000'));
    await render();
    expect(port.navigate).toHaveBeenCalledWith(
      'https://doorway.example/account#refused=CHALLENGE_EXPIRED'
    );
    expect(port.sign).not.toHaveBeenCalled();
  });
  it('rechecks expiry on approval', async () => {
    const { component } = await render();
    vi.spyOn(Date, 'now').mockReturnValue(Date.parse('2100-01-01T00:00:00Z'));
    await component.approve();
    expect(port.sign).not.toHaveBeenCalled();
    expect(port.navigate).toHaveBeenCalledWith(
      'https://doorway.example/account#refused=CHALLENGE_EXPIRED'
    );
  });
  it('returns malformed requests without signing', async () => {
    setUrl(undefined, text + '\n');
    const { root } = await render();
    expect(root.querySelector('[data-testid="account-bind-error"]')?.textContent).toContain(
      'NOT_A_BIND_CHALLENGE'
    );
    expect(port.standing).not.toHaveBeenCalled();
    expect(port.navigate).toHaveBeenCalledWith(
      'https://doorway.example/account#refused=NOT_A_BIND_CHALLENGE'
    );
  });
  it('selects the new mode from the path, including a trailing slash', async () => {
    window.history.replaceState(null, '', '/auth/portal/consent/account-bind/');
    const app = new AppComponent();
    await app.ngOnInit();
    expect(app.mode()).toBe('account-bind');
    expect(routes.some(route => route.path === 'consent/account-bind')).toBe(true);
  });
});

describe('account-bind production port', () => {
  afterEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
  });
  it('passes a session-key prover to the sign helper', async () => {
    const sign = vi.spyOn(helpers, 'signAccountBind').mockResolvedValue({
      ok: true,
      body: { agentPubKey: 'key', signature: 'sig' },
    });
    const port = TestBed.inject(ACCOUNT_BIND_PORT);
    await port.sign(text);
    expect(sign).toHaveBeenCalledWith({ prove: expect.any(Function) }, text);
  });
});

describe('account-bind return addresses', () => {
  it.each([
    'http://localhost/account',
    'http://127.0.0.1/account',
    'https://doorway.example/account',
  ])('allows %s', url => expect(accountBindReturn(url, new URL(url).host)).not.toBeNull());
  it('allows an opaque doorway id with a secure return', () => {
    expect(accountBindReturn('https://doorway.example/account', 'doorway-alpha')).not.toBeNull();
  });
  it('does not allow non-web protocols even on localhost', () => {
    expect(accountBindReturn('ftp://localhost/account', 'localhost')).toBeNull();
  });
  it('allows a port on the return when the audience names only the host', () => {
    expect(accountBindReturn('http://localhost:8888/threshold/account', 'localhost')).not.toBeNull();
    expect(accountBindReturn('https://alpha.elohim.host:8443/a', 'alpha.elohim.host')).not.toBeNull();
  });
  it('refuses a return to a different host than the audience', () => {
    expect(accountBindReturn('https://evil.example/a', 'alpha.elohim.host')).toBeNull();
    expect(accountBindReturn('http://localhost:8888/a', 'localhost:8890')).toBeNull();
  });
});
