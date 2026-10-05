/** @vitest-environment jsdom */

import '@analogjs/vitest-angular/setup-zone';
import '@angular/compiler';

import { getTestBed, TestBed } from '@angular/core/testing';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { AppComponent } from './app.component.js';

const settle = () => new Promise<void>(resolve => setTimeout(resolve, 0));

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const ALONE = {
  identityRoot: 'uhCAkroot',
  authority: 'uhCEkauthority',
  networkDna: 'uhC0kdna',
  controllers: ['uhCAkworkspace'],
  controllerCount: 1,
  required: 1,
  thisNodeIsController: true,
  restsOnThisNodeAlone: true,
};

/** The native portal's root: what an identity rests on, beginning it, and the header. */
describe('AppComponent — the root, for a person on their own device', () => {
  let routes: Record<string, () => Response>;
  let calls: { url: string; init?: RequestInit }[];

  beforeAll(() => {
    getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting());
  });

  beforeEach(() => {
    calls = [];
    routes = {};
    window.history.replaceState(null, '', '/auth/portal/');
    globalThis.fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      calls.push({ url, init });
      const key = Object.keys(routes).find(k => url.includes(k));
      return key ? routes[key]!() : new Response('not found', { status: 404 });
    }) as unknown as typeof fetch;
  });

  afterEach(() => TestBed.resetTestingModule());

  async function render() {
    TestBed.configureTestingModule({ imports: [AppComponent] });
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    await fixture.componentInstance.ngOnInit();
    await settle();
    fixture.detectChanges();
    return fixture;
  }

  const header = (root: HTMLElement) =>
    root.querySelector('[data-testid="portal-trust"]') as
      | (HTMLElement & { strings?: { ownNodeLabel?: string } })
      | null;

  it('offers to begin an identity here when this node has none, asking only for a name', async () => {
    routes['/auth/identity/standing'] = () =>
      json(409, { error: 'none', code: 'consent_identity_unbootstrapped' });
    const fixture = await render();
    const root = fixture.nativeElement as HTMLElement;

    expect(fixture.componentInstance.mode()).toBe('identity');
    const begin = root.querySelector('[data-testid="identity-begin"]')!;
    expect(begin.textContent).toContain('Begin your identity on this device');
    expect(begin.textContent).toContain(
      'Your key is made and kept on this device. Nothing is sent to any host.'
    );
    expect(begin.textContent).toContain(
      'Until you add another device, this device alone speaks for you.'
    );
    // One field asked for, and an optional sign-in secret said to be optional.
    expect(begin.querySelectorAll('input')).toHaveLength(2);
    expect(begin.textContent).toContain('Sign-in secret (optional)');
    expect(begin.textContent).not.toMatch(/doorway/i);
    expect(root.querySelector('elohim-imagodei-federated-resolver')).toBeNull();
    expect(header(root)?.strings?.ownNodeLabel).toBe('Your own device will make and keep your key');
  });

  it('begins with the name, then says what the identity rests on', async () => {
    routes['/auth/identity/standing'] = () =>
      json(409, { error: 'none', code: 'consent_identity_unbootstrapped' });
    routes['/auth/identity/begin'] = () =>
      json(201, {
        standing: ALONE,
        session: { id: 's', humanId: 'h', identifier: 'h' },
        created: { human: true, authority: true, session: true },
      });
    const fixture = await render();
    await fixture.componentInstance.onIdentityBegin({ displayName: 'Matthew' });
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;

    const begin = calls.find(c => c.url.includes('/auth/identity/begin'))!;
    expect(begin.init?.method).toBe('POST');
    expect(JSON.parse(begin.init!.body as string)).toEqual({ displayName: 'Matthew' });
    expect(root.querySelector('[data-testid="identity-standing"]')?.textContent).toBe(
      'Your identity rests on this device alone.'
    );
    expect(header(root)?.strings?.ownNodeLabel).toBe('Your own device holds your key');
    // Nothing asked of any other server.
    expect(calls.every(c => c.url.startsWith('/'))).toBe(true);
  });

  it('says what the identity rests on for a person already signed in here', async () => {
    routes['/auth/identity/standing'] = () =>
      json(200, {
        ...ALONE,
        controllerCount: 2,
        controllers: ['a', 'b'],
        restsOnThisNodeAlone: false,
      });
    const fixture = await render();
    expect(
      (fixture.nativeElement as HTMLElement).querySelector('[data-testid="identity-standing"]')
        ?.textContent
    ).toBe(
      '2 of your own devices speak for you, this device among them. Any one of them can approve a new device for you.'
    );
  });

  it('signs in with the word and secret when a secret is set, naming no doorway', async () => {
    routes['/auth/identity/standing'] = () =>
      json(401, {
        error: 'no session',
        code: 'consent_not_signed_in',
        hasIdentity: true,
        signInSecretSet: true,
      });
    const fixture = await render();
    const root = fixture.nativeElement as HTMLElement;
    expect(fixture.componentInstance.mode()).toBe('sign-in');
    const form = root.querySelector('[data-testid="sign-in"]')!;
    expect(form.textContent).toContain('Sign-in word');
    expect(form.textContent).toContain('Sign-in secret');
    expect(form.textContent).not.toMatch(/doorway|federated/i);
    expect(root.querySelector('elohim-imagodei-federated-resolver')).toBeNull();
    expect(header(root)?.strings?.ownNodeLabel).toBe('Your own device holds your key');
    expect(root.querySelector('elohim-imagodei-attestor-row')).toBeNull();
  });

  it('says where a sign-in secret is set when none is, and how a forgotten one is replaced', async () => {
    routes['/auth/identity/standing'] = () =>
      json(401, {
        error: 'no session',
        code: 'consent_not_signed_in',
        hasIdentity: true,
        signInSecretSet: false,
      });
    const fixture = await render();
    const root = fixture.nativeElement as HTMLElement;
    expect(fixture.componentInstance.mode()).toBe('secret-unset');
    const panel = root.querySelector('[data-testid="sign-in-secret-unset"]')!;
    expect(panel.textContent).toContain('Signing in from a browser needs a sign-in secret.');
    expect(root.querySelector('[data-testid="sign-in-secret-command"]')?.textContent?.trim()).toBe(
      'epr identity secret'
    );
    expect(panel.textContent).toContain('This is also how a forgotten secret is replaced.');
  });

  it('posts the sign-in to this node, then shows what the identity rests on', async () => {
    let signedIn = false;
    routes['/auth/identity/standing'] = () =>
      signedIn
        ? json(200, { ...ALONE, identifier: 'matthew' })
        : json(401, {
            error: 'no session',
            code: 'consent_not_signed_in',
            hasIdentity: true,
            signInSecretSet: true,
          });
    routes['/auth/login'] = () => {
      signedIn = true;
      return json(200, {
        humanId: 'h',
        agentPubKey: 'a',
        identifier: 'matthew',
        expiresAt: 1,
        isSteward: true,
        redirect: '/auth/portal/',
        sessionKeyBound: false,
      });
    };
    const fixture = await render();
    await fixture.componentInstance.onSignIn({ identifier: 'matthew', password: 'a-long-secret' });
    fixture.detectChanges();

    const login = calls.find(c => c.url.includes('/auth/login'))!;
    expect(JSON.parse(login.init!.body as string)).toMatchObject({
      identifier: 'matthew',
      password: 'a-long-secret',
      remember: true,
    });
    expect(fixture.componentInstance.mode()).toBe('identity');
    expect(
      (fixture.nativeElement as HTMLElement).querySelector('[data-testid="identity-standing"]')
        ?.textContent
    ).toContain('matthew');
  });

  it('says a slowed sign-in is a wait, and for how long', async () => {
    routes['/auth/identity/standing'] = () =>
      json(401, { code: 'consent_not_signed_in', hasIdentity: true, signInSecretSet: true });
    routes['/auth/login'] = () =>
      json(429, { error: 'slow', code: 'signin_slowed', retryAfter: 30 });
    const fixture = await render();
    await fixture.componentInstance.onSignIn({ identifier: 'matthew', password: 'wrong-secret' });
    fixture.detectChanges();
    expect(
      (fixture.nativeElement as HTMLElement).querySelector('[data-testid="sign-in-refusal"]')
        ?.textContent
    ).toContain('This is a wait: try again in 30 seconds.');
  });

  it('shows the sign-in form when sent back to sign in again, though a session is open', async () => {
    window.history.replaceState(
      null,
      '',
      '/auth/portal/?sign_in=1&return_to=%2Fauth%2Fportal%2Fconsent%2Fdevice'
    );
    routes['/auth/identity/standing'] = () => json(200, ALONE);
    const fixture = await render();
    expect(fixture.componentInstance.mode()).toBe('sign-in');
    expect(
      (fixture.nativeElement as HTMLElement).querySelector('[data-testid="sign-in"]')?.textContent
    ).toContain('Then you’ll go straight back to where you were.');
  });

  it('signs out and returns to the sign-in', async () => {
    let signedIn = true;
    routes['/auth/identity/standing'] = () =>
      signedIn
        ? json(200, ALONE)
        : json(401, { code: 'consent_not_signed_in', hasIdentity: true, signInSecretSet: true });
    routes['/auth/logout'] = () => {
      signedIn = false;
      return json(200, {});
    };
    const fixture = await render();
    expect(
      (fixture.nativeElement as HTMLElement).querySelector('[data-testid="identity-sign-out"]')
    ).not.toBeNull();
    await fixture.componentInstance.onSignOut();
    expect(calls.some(c => c.url.includes('/auth/logout'))).toBe(true);
    expect(fixture.componentInstance.mode()).toBe('sign-in');
  });

  it('says nothing in the header when nothing is known, rather than “Hosted via” no one', async () => {
    const fixture = await render();
    const root = fixture.nativeElement as HTMLElement;
    expect(fixture.componentInstance.mode()).toBe('login');
    expect(header(root)).toBeNull();
    expect(root.querySelector('[data-testid="portal-header-empty"]')?.getAttribute('slot')).toBe(
      'header'
    );
  });

  it('leaves the shell’s own header for a doorway that hosts the person', async () => {
    routes['/auth/me'] = () =>
      json(200, { trustMode: 'doorway-host', authority: { label: 'alpha.elohim.host' } });
    const fixture = await render();
    await settle();
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;
    expect(header(root)).toBeNull();
    expect(root.querySelector('[slot="header"]')).toBeNull();
  });
});
