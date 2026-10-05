import { HttpErrorResponse } from '@angular/common/http';
import { TestBed, type ComponentFixture } from '@angular/core/testing';
import { ActivatedRoute, Router, convertToParamMap } from '@angular/router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthStateService } from '../../services/auth-state.service';
import {
  DeviceConsentService,
  type ConsentAgreeResponse,
  type ConsentViewResponse,
} from '../../services/device-consent.service';

import { DeviceConsentComponent } from './device-consent.component';

/** Let pending promise continuations run. */
const settle = () => new Promise<void>(resolve => setTimeout(resolve, 0));

function encode(value: unknown): string {
  return btoa(JSON.stringify(value)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

const GRANT_REQUEST = { clientId: 'epr-cli', label: 'workspace', acts: ['device.enroll'] };
const PARAM = encode(GRANT_REQUEST);
const PAGE_URL = `/consent/device?request=${PARAM}`;

const VIEW: ConsentViewResponse = {
  clientId: 'epr-cli',
  label: 'workspace',
  deviceFingerprint: 'uhCAk…3FOt',
  deviceRootFingerprint: 'uhCkk…PrcW',
  askedActs: ['device.enroll', 'device.bind-root'],
};

/** Sign-in-first, the phases, and the hand-back rule of the device approval page. */
describe('DeviceConsentComponent', () => {
  let signedIn: boolean;
  let consent: {
    view: ReturnType<typeof vi.fn>;
    agree: ReturnType<typeof vi.fn>;
    handBack: ReturnType<typeof vi.fn>;
  };
  let router: {
    url: string;
    createUrlTree: ReturnType<typeof vi.fn>;
    navigateByUrl: ReturnType<typeof vi.fn>;
  };

  function create(param: string | null = PARAM): ComponentFixture<DeviceConsentComponent> {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [DeviceConsentComponent],
      providers: [
        {
          provide: ActivatedRoute,
          useValue: {
            snapshot: {
              queryParamMap: convertToParamMap(param === null ? {} : { request: param }),
            },
          },
        },
        { provide: Router, useValue: router },
        {
          provide: AuthStateService,
          useValue: {
            isAuthenticated: () => signedIn,
            account: () => (signedIn ? { identifier: 'matthew' } : null),
          },
        },
        { provide: DeviceConsentService, useValue: consent },
      ],
    });
    const fixture = TestBed.createComponent(DeviceConsentComponent);
    fixture.detectChanges();
    return fixture;
  }

  async function ready(fixture: ComponentFixture<DeviceConsentComponent>) {
    await settle();
    fixture.detectChanges();
  }

  function card(fixture: ComponentFixture<DeviceConsentComponent>): HTMLElement & {
    phase?: string;
    request?: ConsentViewResponse;
    code?: string;
    expiresAt?: number;
    refusalCode?: string;
    personLabel?: string;
    signer?: string;
  } {
    return fixture.nativeElement.querySelector('[data-testid="device-consent-card"]');
  }

  function approve(fixture: ComponentFixture<DeviceConsentComponent>, agreedActs: string[]) {
    card(fixture).dispatchEvent(
      new CustomEvent('approve', {
        detail: { agreedActs, declinedActs: VIEW.askedActs.filter(a => !agreedActs.includes(a)) },
      })
    );
  }

  beforeEach(() => {
    sessionStorage.clear();
    signedIn = true;
    consent = {
      view: vi.fn().mockResolvedValue(VIEW),
      agree: vi.fn(),
      handBack: vi.fn(),
    };
    router = {
      url: PAGE_URL,
      createUrlTree: vi.fn((commands: unknown[], extras: unknown) => ({ commands, extras })),
      navigateByUrl: vi.fn().mockResolvedValue(true),
    };
  });

  afterEach(() => sessionStorage.clear());

  describe('sign-in first', () => {
    it('sends a signed-out person to sign in and back to this exact page', () => {
      signedIn = false;
      const fixture = create();

      expect(router.createUrlTree).toHaveBeenCalledWith(['/login'], {
        queryParams: { returnUrl: PAGE_URL },
      });
      expect(router.navigateByUrl).toHaveBeenCalledWith({
        commands: ['/login'],
        extras: { queryParams: { returnUrl: PAGE_URL } },
      });
      // Nothing about the request is fetched or shown before sign-in.
      expect(consent.view).not.toHaveBeenCalled();
      expect(card(fixture)).toBeNull();
    });

    it('fetches the request only once signed in, and shows it for review', async () => {
      const fixture = create();
      expect(consent.view).toHaveBeenCalledWith(GRANT_REQUEST);
      await ready(fixture);

      const el = card(fixture);
      expect(el.phase).toBe('review');
      expect(el.request).toEqual(VIEW);
      expect(el.personLabel).toBe('matthew');
      expect(el.getAttribute('signer')).toBe('doorway-host');
    });

    it('goes back to sign-in when the session has run out', async () => {
      consent.view.mockRejectedValue(new HttpErrorResponse({ status: 401 }));
      const fixture = create();
      await ready(fixture);

      expect(router.navigateByUrl).toHaveBeenCalledTimes(1);
      expect(card(fixture)).toBeNull();
    });
  });

  describe('reading the request', () => {
    it.each([
      ['absent', null],
      ['not base64url', '!!!'],
      ['not JSON', 'bm90IGpzb24'],
    ])('refuses an %s request without calling the doorway', async (_label, param) => {
      const fixture = create(param);
      await ready(fixture);

      expect(consent.view).not.toHaveBeenCalled();
      expect(card(fixture).phase).toBe('refused');
      expect(card(fixture).refusalCode).toBe('request_unreadable');
    });

    it('shows the runtime’s refusal code', async () => {
      consent.view.mockRejectedValue(
        new HttpErrorResponse({
          status: 400,
          error: { error: 'cannot be shown', code: 'request_acts_incoherent' },
        })
      );
      const fixture = create();
      await ready(fixture);

      expect(card(fixture).phase).toBe('refused');
      expect(card(fixture).refusalCode).toBe('request_acts_incoherent');
    });

    it('says approvals are unavailable when this doorway lacks the endpoint', async () => {
      consent.view.mockRejectedValue(new HttpErrorResponse({ status: 404 }));
      const fixture = create();
      await ready(fixture);

      expect(card(fixture).refusalCode).toBe('consent_unavailable');
    });
  });

  describe('approving', () => {
    it('signs only what was agreed and shows the code to paste', async () => {
      const answer: ConsentAgreeResponse = {
        returnTarget: { kind: 'display', value: 'K7QF-2MXD' },
        expiresAt: 1_791_000_300_000,
      };
      consent.agree.mockResolvedValue(answer);
      const fixture = create();
      await ready(fixture);

      approve(fixture, ['device.enroll']);
      expect(fixture.componentInstance.phase()).toBe('signing');
      await ready(fixture);

      expect(consent.agree).toHaveBeenCalledWith({
        request: GRANT_REQUEST,
        agreedActs: ['device.enroll'],
      });
      expect(card(fixture).phase).toBe('code');
      expect(card(fixture).code).toBe('K7QF-2MXD');
      expect(card(fixture).expiresAt).toBe(1_791_000_300_000);
      expect(consent.handBack).not.toHaveBeenCalled();
    });

    it('hands the code to a terminal on this machine and says so', async () => {
      const url = 'http://127.0.0.1:53682/callback?code=c0de&state=s';
      consent.agree.mockResolvedValue({ returnTarget: { kind: 'redirect', url }, expiresAt: 1 });
      const fixture = create();
      await ready(fixture);

      approve(fixture, ['device.enroll', 'device.bind-root']);
      await ready(fixture);

      expect(card(fixture).phase).toBe('handed-back');
      expect(consent.handBack).toHaveBeenCalledWith(url);
    });

    it('never follows a redirect to anywhere but this machine’s terminal', async () => {
      consent.agree.mockResolvedValue({
        returnTarget: { kind: 'redirect', url: 'https://evil.example/collect' },
        expiresAt: 1,
      });
      const fixture = create();
      await ready(fixture);

      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(consent.handBack).not.toHaveBeenCalled();
      expect(card(fixture).phase).toBe('refused');
      expect(card(fixture).refusalCode).toBe('return_path_refused');
    });

    it('sends one approval even if approve fires twice', async () => {
      consent.agree.mockResolvedValue({
        returnTarget: { kind: 'display', value: 'K7QF' },
        expiresAt: 1,
      });
      const fixture = create();
      await ready(fixture);

      approve(fixture, ['device.enroll']);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(consent.agree).toHaveBeenCalledTimes(1);
    });
  });

  it('declining sends nothing', async () => {
    const fixture = create();
    await ready(fixture);

    card(fixture).dispatchEvent(
      new CustomEvent('decline', { detail: { reason: 'user-rejected' } })
    );
    fixture.detectChanges();

    expect(card(fixture).phase).toBe('declined');
    expect(consent.agree).not.toHaveBeenCalled();
  });

  describe('leaving and coming back', () => {
    it('shows the same code again without asking or approving again', async () => {
      consent.agree.mockResolvedValue({
        returnTarget: { kind: 'display', value: 'K7QF' },
        expiresAt: Date.now() + 60_000,
      });
      const first = create();
      await ready(first);
      approve(first, ['device.enroll']);
      await ready(first);
      first.destroy();

      consent.view.mockClear();
      const again = create();
      await ready(again);

      expect(consent.view).not.toHaveBeenCalled();
      expect(consent.agree).toHaveBeenCalledTimes(1);
      expect(card(again).phase).toBe('code');
      expect(card(again).code).toBe('K7QF');
    });

    it('does not re-submit an approval that was still being signed when the person left', async () => {
      consent.agree.mockReturnValue(new Promise(() => undefined)); // never answers
      const first = create();
      await ready(first);
      approve(first, ['device.enroll']);
      first.destroy();

      const again = create();
      await ready(again);

      expect(consent.agree).toHaveBeenCalledTimes(1);
      expect(card(again).phase).toBe('refused');
      expect(card(again).refusalCode).toBe('approval_interrupted');

      // The card is no longer in review, so an approve intent is ignored.
      approve(again, ['device.enroll']);
      await ready(again);
      expect(consent.agree).toHaveBeenCalledTimes(1);
    });

    it('remembers a decline', async () => {
      const first = create();
      await ready(first);
      card(first).dispatchEvent(new CustomEvent('decline'));
      first.destroy();

      const again = create();
      await ready(again);
      expect(card(again).phase).toBe('declined');
    });
  });

  describe('witness trail', () => {
    const WITNESSES = [
      {
        id: 'device-sign',
        act: 'signed',
        relation: 'your-device',
        label: 'workspace',
        state: 'done',
      },
      { id: 'peers', act: 'recorded', relation: 'others', count: 3, state: 'done' },
    ];

    function trail(fixture: ComponentFixture<DeviceConsentComponent>) {
      return fixture.nativeElement.querySelector('[data-testid="device-consent-witness-trail"]') as
        | (HTMLElement & { steps?: unknown })
        | null;
    }

    function doorwayStep(fixture: ComponentFixture<DeviceConsentComponent>, state: string) {
      return {
        id: 'doorway-sign',
        act: 'signed',
        relation: 'your-doorway',
        label: fixture.componentInstance.hostLabel,
        state,
      };
    }

    it('is not shown while the person reviews', async () => {
      const fixture = create();
      await ready(fixture);
      expect(trail(fixture)).toBeNull();
    });

    it('shows the doorway signing, live, while signing', async () => {
      consent.agree.mockReturnValue(new Promise(() => undefined));
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      fixture.detectChanges();

      expect(trail(fixture)?.steps).toEqual([doorwayStep(fixture, 'working')]);
      expect(trail(fixture)?.getAttribute('mode')).toBeNull();
    });

    it('passes reported witnesses through after the doorway step, settled under the code', async () => {
      consent.agree.mockResolvedValue({
        returnTarget: { kind: 'display', value: 'K7QF' },
        expiresAt: Date.now() + 60_000,
        witnesses: WITNESSES,
      });
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(trail(fixture)?.steps).toEqual([doorwayStep(fixture, 'done'), ...WITNESSES]);
      expect(trail(fixture)?.getAttribute('mode')).toBe('settled');
    });

    it('settles under handed-back too, and hands back without waiting on it', async () => {
      const url = 'http://127.0.0.1:53682/cb';
      consent.agree.mockResolvedValue({ returnTarget: { kind: 'redirect', url }, expiresAt: 1 });
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(consent.handBack).toHaveBeenCalledWith(url);
      expect(trail(fixture)?.steps).toEqual([doorwayStep(fixture, 'done')]);
      expect(trail(fixture)?.getAttribute('mode')).toBe('settled');
    });

    it('marks the doorway step failed on a refusal', async () => {
      consent.agree.mockRejectedValue(
        new HttpErrorResponse({ status: 400, error: { error: 'no', code: 'act_unknown' } })
      );
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(card(fixture).phase).toBe('refused');
      expect(trail(fixture)?.steps).toEqual([doorwayStep(fixture, 'failed')]);
    });

    it('restores the settled trail from this tab without re-running anything', async () => {
      consent.agree.mockResolvedValue({
        returnTarget: { kind: 'display', value: 'K7QF' },
        expiresAt: Date.now() + 60_000,
        witnesses: WITNESSES,
      });
      const first = create();
      await ready(first);
      approve(first, ['device.enroll']);
      await ready(first);
      first.destroy();

      consent.view.mockClear();
      const again = create();
      await ready(again);

      expect(consent.view).not.toHaveBeenCalled();
      expect(consent.agree).toHaveBeenCalledTimes(1);
      expect(trail(again)?.steps).toEqual([doorwayStep(again, 'done'), ...WITNESSES]);
      expect(trail(again)?.getAttribute('mode')).toBe('settled');
    });
  });
});
