/** @vitest-environment jsdom */

import '@analogjs/vitest-angular/setup-zone';
import '@angular/compiler';

import { getTestBed, TestBed, type ComponentFixture } from '@angular/core/testing';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';
import type { PendingAsksClient, PendingAsksView } from 'elohim-imagodei/pending-asks';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { rootPageFor } from '../app.component.js';

import { PendingAsksComponent } from './pending-asks.component.js';

const settle = () => new Promise<void>(resolve => setTimeout(resolve, 0));

const PERSON = {
  identifier: 'matthew',
  humanId: 'h-4c1d',
  displayName: 'Matthew',
  identityRoot: 'uhCkkroot',
  identityFingerprint: 'uhCkk…root',
};

const LISTED: PendingAsksView = {
  carrier: 'private-network',
  approver: 'uhCAkme',
  speaksFor: { kind: 'person', ...PERSON },
  speaksForWords: 'An approval here is for matthew (Matthew), identity uhCkk…root.',
  asks: [
    {
      number: 1,
      label: 'laptop',
      deviceKey: 'uhCAklaptop',
      deviceFingerprint: 'uhCAk…7Lq2',
      askedActs: ['device.enroll'],
      secondsLeft: 250,
      state: { kind: 'unassigned' },
      stateWords: 'has no identity of its own',
      addressedHere: true,
      forIdentity: PERSON,
    },
  ],
};

describe('rootPageFor — what the root shows from what the node said', () => {
  const standing = { phase: 'standing' as const, standing: null };
  const flags = { signInAsked: false, keyLost: false };
  it.each([
    [standing, flags, 'identity'],
    [standing, { ...flags, signInAsked: true }, 'sign-in'],
    [standing, { ...flags, keyLost: true }, 'sign-in'],
    [{ phase: 'begin' as const, standing: null }, flags, 'identity'],
    [
      {
        phase: 'not-signed-in' as const,
        standing: null,
        signedOut: { hasIdentity: true, signInSecretSet: true },
      },
      flags,
      'sign-in',
    ],
    [
      {
        phase: 'not-signed-in' as const,
        standing: null,
        signedOut: { hasIdentity: true, signInSecretSet: false },
      },
      flags,
      'secret-unset',
    ],
    [{ phase: 'unavailable' as const, standing: null }, flags, 'legacy'],
  ])('%o with %o → %s', (identity, f, page) => {
    expect(rootPageFor(identity, f)).toBe(page);
  });
});

describe('PendingAsksComponent — devices asking over the private network', () => {
  let client: { list: ReturnType<typeof vi.fn>; decide: ReturnType<typeof vi.fn> };

  beforeAll(() => {
    getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting());
  });

  beforeEach(() => {
    sessionStorage.clear();
    client = { list: vi.fn().mockResolvedValue({ ok: true, body: LISTED }), decide: vi.fn() };
  });

  afterEach(() => TestBed.resetTestingModule());

  async function create(): Promise<ComponentFixture<PendingAsksComponent>> {
    TestBed.configureTestingModule({ imports: [PendingAsksComponent] });
    const fixture = TestBed.createComponent(PendingAsksComponent);
    fixture.componentInstance.client = client as unknown as PendingAsksClient;
    fixture.detectChanges();
    await settle();
    fixture.detectChanges();
    return fixture;
  }

  const q = (f: ComponentFixture<unknown>, id: string) =>
    (f.nativeElement as HTMLElement).querySelector(`[data-testid="${id}"]`);

  it('lists each ask with its key, what it asks, its time left, and whose identity it is for', async () => {
    const f = await create();
    const ask = q(f, 'pending-ask-1')!;
    expect(ask.textContent).toContain('laptop');
    expect(ask.textContent).toContain('uhCAk…7Lq2');
    expect(ask.textContent).toContain('Asks to enroll this device.');
    expect(ask.textContent).toContain('About 4 minutes left');
    expect(ask.textContent).toContain('The asking device has no identity of its own.');
    expect(q(f, 'pending-for')?.textContent).toContain('matthew (Matthew)');
    expect((f.nativeElement as HTMLElement).textContent).not.toContain('h-4c1d');
  });

  it('opens the same review card; declining tells the node, and the ask leaves the list', async () => {
    client.decide.mockResolvedValue({
      ok: true,
      body: { number: 1, decidedBy: 'answer', declined: true, handedBack: { taken: true } },
    });
    const f = await create();
    (q(f, 'pending-review-1') as HTMLButtonElement).click();
    await settle();
    f.detectChanges();
    const card = q(f, 'device-consent-card') as HTMLElement & { phase?: string };
    expect(card.phase).toBe('review');

    client.list.mockResolvedValue({ ok: true, body: { ...LISTED, asks: [] } });
    card.dispatchEvent(new CustomEvent('decline'));
    await settle();
    await settle();
    f.detectChanges();

    expect(client.decide).toHaveBeenCalledWith({ ask: '1', answer: { agreedActs: [] } });
    expect(q(f, 'device-consent-card')).toBeNull();
    expect(q(f, 'pending-ask-1')).toBeNull();
    expect(q(f, 'pending-notice')?.textContent).toContain('Nothing was approved.');
    expect(q(f, 'pending-notice')?.textContent).toContain(
      'was told, so its terminal stops waiting.'
    );
  });

  it('a decline the node refused stays on the card and says why', async () => {
    client.decide.mockResolvedValue({
      ok: false,
      status: 401,
      body: { code: 'session_proof_stale' },
    });
    const f = await create();
    (q(f, 'pending-review-1') as HTMLButtonElement).click();
    await settle();
    f.detectChanges();
    q(f, 'device-consent-card')!.dispatchEvent(new CustomEvent('decline'));
    await settle();
    f.detectChanges();
    expect((q(f, 'device-consent-card') as HTMLElement & { phase?: string }).phase).toBe('review');
    expect(q(f, 'pending-notice')?.textContent).toContain('Nothing was declined');
    expect(q(f, 'pending-sign-in-again')).not.toBeNull();
  });

  it('decides with the acts agreed, and goes back to the list', async () => {
    client.decide.mockResolvedValue({
      ok: true,
      body: {
        number: 1,
        decidedBy: 'answer',
        agreed: {
          returnTarget: { kind: 'display', value: 'K7QF' },
          expiresAt: Date.now() + 60_000,
          consentCid: 'c',
          controllers: { required: 1, signed: 1 },
          witnesses: [],
        },
        handedBack: { taken: true },
      },
    });
    const f = await create();
    (q(f, 'pending-review-1') as HTMLButtonElement).click();
    await settle();
    f.detectChanges();
    q(f, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    f.detectChanges();
    expect(client.decide).toHaveBeenCalledWith({
      ask: '1',
      answer: { agreedActs: ['device.enroll'] },
    });
    const handed = q(f, 'device-consent-card') as HTMLElement & { phase?: string };
    expect(handed.phase).toBe('handed-back');
    // Over the private network, not to a terminal on this machine.
    expect(handed.getAttribute('handed-back-to')).toBe('network');
    (q(f, 'pending-back') as HTMLButtonElement).click();
    await settle();
    f.detectChanges();
    expect(q(f, 'pending-asks')).not.toBeNull();
    expect(client.list).toHaveBeenCalledTimes(2);
  });

  it('a decide refused for a page not signed in names the ask’s number in the terminal command', async () => {
    client.decide.mockResolvedValue({
      ok: false,
      status: 403,
      body: { error: 'x', code: 'consent_caller_not_local' },
    });
    const f = await create();
    (q(f, 'pending-review-1') as HTMLButtonElement).click();
    await settle();
    f.detectChanges();
    q(f, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    f.detectChanges();
    expect(q(f, 'device-consent-command')?.textContent?.trim()).toBe('epr device approve 1');
    expect(q(f, 'device-consent-elsewhere')?.textContent).not.toContain('open this same link');
  });

  it('gives a node that speaks for nobody one line and no list; nothing without a network', async () => {
    client.list.mockResolvedValue({
      ok: true,
      body: {
        ...LISTED,
        speaksFor: { kind: 'nobody' },
        speaksForWords: 'This device does not speak for anyone, so it lists no requests.',
        asks: [],
      },
    });
    let f = await create();
    expect(q(f, 'pending-lists-nothing')?.textContent).toBe(
      'This device does not speak for anyone, so it lists no requests.'
    );
    expect(q(f, 'pending-asks')).toBeNull();
    TestBed.resetTestingModule();

    client.list.mockResolvedValue({ ok: true, body: { ...LISTED, carrier: 'absent' } });
    f = await create();
    expect((f.nativeElement as HTMLElement).textContent?.trim()).toBe('');
  });
});
