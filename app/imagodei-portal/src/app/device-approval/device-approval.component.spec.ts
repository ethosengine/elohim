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
});
