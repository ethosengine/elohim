/** @vitest-environment jsdom */

import '@analogjs/vitest-angular/setup-zone';
import '@angular/compiler';

import { getTestBed, TestBed, type ComponentFixture } from '@angular/core/testing';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';
import type {
  ConsentAgreeResponse,
  ConsentViewResponse,
  ConsentWireResult,
} from 'elohim-imagodei/device-consent';
import type { IdentityStandingView } from 'elohim-imagodei/identity-standing';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { devicePageFor } from '../app.component.js';

import { DEVICE_APPROVAL_PORT } from './device-approval-port.js';
import { DeviceApprovalComponent } from './device-approval.component.js';

const settle = () => new Promise<void>(resolve => setTimeout(resolve, 0));

const encode = (value: unknown) =>
  btoa(JSON.stringify(value)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');

const GRANT_REQUEST = { clientId: 'epr-cli', label: 'workspace', acts: ['device.enroll'] };

const VIEW: ConsentViewResponse = {
  clientId: 'epr-cli',
  label: 'workspace',
  deviceFingerprint: 'uhCAk…3FOt',
  askedActs: ['device.enroll'],
};

const ok = <T>(body: T): ConsentWireResult<T> => ({ ok: true, body });

const agreed = (partial: Partial<ConsentAgreeResponse> = {}): ConsentAgreeResponse => ({
  returnTarget: { kind: 'display', value: 'K7QF-2MXD' },
  expiresAt: Date.now() + 60_000,
  consentCid: 'bafyreiconsent',
  controllers: { required: 1, signed: 1 },
  witnesses: [{ id: 'this-device-sign', act: 'signed', relation: 'this-device', state: 'done' }],
  ...partial,
});

const ALONE: IdentityStandingView = {
  identityRoot: 'uhCAkroot',
  authority: 'uhCEkauthority',
  networkDna: 'uhC0kdna',
  controllers: ['uhCAkworkspace'],
  controllerCount: 1,
  required: 1,
  thisNodeIsController: true,
  restsOnThisNodeAlone: true,
};

const BEGUN = {
  standing: ALONE,
  session: { id: 'sess-1', humanId: 'h-1', identifier: 'h-1' },
  created: { human: true, authority: true, session: true },
};

const unbootstrapped = {
  ok: false,
  status: 409,
  body: { error: 'no identity here', code: 'consent_identity_unbootstrapped' },
};

type Card = HTMLElement & {
  phase?: string;
  refusalCode?: string;
  strings?: { signerOwnNode?: (host?: string) => string; refusal?: Record<string, string> };
};

describe('devicePageFor — which paths are the device approval page', () => {
  it.each([
    ['/auth/portal/consent/device', 'live'],
    ['/auth/portal/consent/device/', 'live'],
    ['/auth/portal/consent/device/preview', 'preview'],
    ['/auth/portal/', null],
    ['/auth/portal/consent', null],
  ])('%s → %s', (path, page) => {
    expect(devicePageFor(path)).toBe(page);
  });
});

describe('DeviceApprovalComponent — the native portal’s mount of the shared page', () => {
  let port: {
    client: { view: ReturnType<typeof vi.fn>; agree: ReturnType<typeof vi.fn> };
    identity: { standing: ReturnType<typeof vi.fn>; begin: ReturnType<typeof vi.fn> };
    signIn: ReturnType<typeof vi.fn>;
    handBack: ReturnType<typeof vi.fn>;
  };

  beforeAll(() => {
    getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting());
  });

  beforeEach(() => {
    sessionStorage.clear();
    window.history.replaceState(
      null,
      '',
      `/auth/portal/consent/device?request=${encode(GRANT_REQUEST)}`
    );
    port = {
      client: { view: vi.fn().mockResolvedValue(ok(VIEW)), agree: vi.fn() },
      identity: {
        standing: vi.fn().mockResolvedValue(ok(ALONE)),
        begin: vi.fn().mockResolvedValue(ok(BEGUN)),
      },
      signIn: vi.fn(),
      handBack: vi.fn(),
    };
  });

  afterEach(() => {
    sessionStorage.clear();
    TestBed.resetTestingModule();
  });

  async function create(): Promise<ComponentFixture<DeviceApprovalComponent>> {
    TestBed.configureTestingModule({
      imports: [DeviceApprovalComponent],
      providers: [{ provide: DEVICE_APPROVAL_PORT, useValue: port }],
    });
    const fixture = TestBed.createComponent(DeviceApprovalComponent);
    fixture.detectChanges();
    await settle();
    fixture.detectChanges();
    return fixture;
  }

  const q = (fixture: ComponentFixture<unknown>, testid: string) =>
    (fixture.nativeElement as HTMLElement).querySelector(`[data-testid="${testid}"]`);

  it('asks only this node, with the request as the link carried it', async () => {
    const fixture = await create();
    expect(port.client.view).toHaveBeenCalledWith(GRANT_REQUEST);
    const card = q(fixture, 'device-consent-card') as Card;
    expect(card.phase).toBe('review');
    expect(card.getAttribute('signer')).toBe('peer-conductor');
  });

  it('tells the person plainly that this device holds the key and signs, and never names a doorway', async () => {
    const fixture = await create();
    const card = q(fixture, 'device-consent-card') as Card;
    const signer = card.strings?.signerOwnNode?.();
    expect(signer).toBe('This device holds your key, and will sign this as you when you approve.');
    expect(JSON.stringify(card.strings?.refusal)).not.toMatch(/doorway/i);
  });

  it('leads the trail with this device and says the approval is complete on it', async () => {
    port.client.agree.mockResolvedValue(ok(agreed()));
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'], declinedActs: [] } })
    );
    await settle();
    fixture.detectChanges();

    const trail = q(fixture, 'device-consent-witness-trail') as HTMLElement & { steps?: unknown[] };
    expect(trail.steps).toEqual(agreed().witnesses);
    expect(q(fixture, 'device-consent-standing')?.textContent).toContain(
      'This device signed as you, and that is enough: this approval is complete.'
    );
  });

  it('sends the person to sign in and back when this node says no one is signed in', async () => {
    port.client.agree.mockResolvedValue({
      ok: false,
      status: 403,
      body: { error: 'no session', code: 'consent_not_signed_in' },
    });
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    fixture.detectChanges();

    expect(port.signIn).toHaveBeenCalledTimes(1);
    expect((q(fixture, 'device-consent-card') as Card).refusalCode).toBe('consent_not_signed_in');
  });

  it('declining calls nothing', async () => {
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(new CustomEvent('decline'));
    fixture.detectChanges();
    expect((q(fixture, 'device-consent-card') as Card).phase).toBe('declined');
    expect(port.client.agree).not.toHaveBeenCalled();
  });

  it('says what the identity rests on while the person decides', async () => {
    const fixture = await create();
    expect(q(fixture, 'identity-standing')?.textContent).toContain(
      'Your identity rests on this device alone.'
    );
  });

  it('offers the begin step in place when this node has no identity, and asks nothing else', async () => {
    port.identity.standing.mockResolvedValue(unbootstrapped);
    const fixture = await create();
    expect(q(fixture, 'device-consent-card')).toBeNull();
    const begin = q(fixture, 'identity-begin')!;
    expect(begin.textContent).toContain('Begin your identity on this device');
    expect(begin.textContent).toContain('Nothing is sent to any host.');
    expect(begin.textContent).toContain('Then you’ll come back here to approve “workspace”.');
    // One field asked for, and an optional sign-in secret said to be optional.
    expect(begin.querySelectorAll('input')).toHaveLength(2);
    expect(begin.textContent).toContain('Sign-in secret (optional)');
    expect(begin.textContent).not.toMatch(/doorway/i);
    expect(port.client.agree).not.toHaveBeenCalled();
  });

  it('after an approval the node could not sign for want of an identity: begin, then back to review, sending nothing twice', async () => {
    port.identity.standing.mockResolvedValue({ ok: false, status: 404, body: null });
    port.client.agree.mockResolvedValue(unbootstrapped);
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    fixture.detectChanges();
    expect(q(fixture, 'identity-begin')).not.toBeNull();

    (q(fixture, 'identity-name') as HTMLInputElement).value = 'Matthew';
    (q(fixture, 'identity-begin')!.querySelector('form') as HTMLFormElement).dispatchEvent(
      new Event('submit')
    );
    await settle();
    await settle();
    fixture.detectChanges();

    expect(port.identity.begin).toHaveBeenCalledWith({ displayName: 'Matthew' });
    expect((q(fixture, 'device-consent-card') as Card).phase).toBe('review');
    expect(port.client.agree).toHaveBeenCalledTimes(1);
    expect(port.client.view).toHaveBeenCalledTimes(1);
  });

  it('on a page open on another machine, shows the same link as a terminal command to copy', async () => {
    port.client.agree.mockResolvedValue({
      ok: false,
      status: 403,
      body: { error: 'not local', code: 'consent_caller_not_local' },
    });
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    fixture.detectChanges();

    expect((q(fixture, 'device-consent-card') as Card).refusalCode).toBe(
      'consent_caller_not_local'
    );
    expect(q(fixture, 'device-consent-command')?.textContent?.trim()).toBe(
      `epr device approve '${window.location.href}'`
    );
    expect(q(fixture, 'device-consent-witness-trail')).toBeNull();
  });

  it('asked to sign in again: shows the reason, and signs in only when the person chooses', async () => {
    port.client.agree.mockResolvedValue({
      ok: false,
      status: 401,
      body: {
        error: 'again',
        code: 'consent_reauthentication_asked',
        reason: 'You signed in on this device more than a day ago.',
      },
    });
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    fixture.detectChanges();

    expect(q(fixture, 'device-consent-reauth-reason')?.textContent?.trim()).toBe(
      'You signed in on this device more than a day ago.'
    );
    expect(port.signIn).not.toHaveBeenCalled();
    (q(fixture, 'device-consent-sign-in-again') as HTMLButtonElement).click();
    expect(port.signIn).toHaveBeenCalledTimes(1);
    expect(port.client.agree).toHaveBeenCalledTimes(1);
  });

  it('names the person by their sign-in word, never by the record id', async () => {
    port.identity.standing.mockResolvedValue(ok({ ...ALONE, identifier: 'matthew' }));
    const fixture = await create();
    expect((q(fixture, 'device-consent-card') as Card & { personLabel?: string }).personLabel).toBe(
      'matthew'
    );
    expect(q(fixture, 'identity-standing')?.textContent).toContain('Your identity,');
    expect(q(fixture, 'identity-standing')?.textContent).toContain('matthew');
    expect((fixture.nativeElement as HTMLElement).textContent).not.toContain('uhCAkroot');
  });

  it('when this browser’s sign-in can no longer be confirmed: says so plainly, and signs in only when asked', async () => {
    port.client.agree.mockResolvedValue({
      ok: false,
      status: 401,
      body: { error: 'proof', code: 'session_proof_invalid' },
    });
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    fixture.detectChanges();
    expect((q(fixture, 'device-consent-card') as Card).refusalCode).toBe('session_proof_invalid');
    expect(port.signIn).not.toHaveBeenCalled();
    (q(fixture, 'device-consent-sign-in-again') as HTMLButtonElement).click();
    expect(port.signIn).toHaveBeenCalledTimes(1);
    expect(port.client.agree).toHaveBeenCalledTimes(1);
  });

  it('a page neither signed in nor on the node’s machine: offers sign-in, and the command for that machine', async () => {
    port.client.agree.mockResolvedValue({
      ok: false,
      status: 403,
      body: { error: 'x', code: 'consent_caller_not_local' },
    });
    const fixture = await create();
    q(fixture, 'device-consent-card')!.dispatchEvent(
      new CustomEvent('approve', { detail: { agreedActs: ['device.enroll'] } })
    );
    await settle();
    fixture.detectChanges();
    (q(fixture, 'device-consent-sign-in') as HTMLButtonElement).click();
    expect(port.signIn).toHaveBeenCalledTimes(1);
    expect(q(fixture, 'device-consent-command')).not.toBeNull();
  });
});
