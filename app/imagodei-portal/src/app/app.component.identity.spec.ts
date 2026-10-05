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
    expect(begin.querySelectorAll('input')).toHaveLength(1);
    expect(root.querySelector('input[type="password"]')).toBeNull();
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
    await fixture.componentInstance.onIdentityBegin('Matthew');
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
      'Your identity rests on 2 of your own devices, this device among them. Any one of them can approve a new device.'
    );
  });

  it('keeps the sign-in for no one signed in, with this host’s header and no witness line', async () => {
    routes['/auth/identity/standing'] = () =>
      json(401, { error: 'no session', code: 'consent_not_signed_in' });
    const fixture = await render();
    const root = fixture.nativeElement as HTMLElement;
    expect(fixture.componentInstance.mode()).toBe('login');
    expect(root.querySelector('elohim-imagodei-federated-resolver')).not.toBeNull();
    expect(header(root)?.strings?.ownNodeLabel).toBe('Your own device holds your key');
    expect(root.querySelector('elohim-imagodei-attestor-row')).toBeNull();
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
