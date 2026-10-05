import { TestBed, type ComponentFixture } from '@angular/core/testing';
import { ActivatedRoute, Router, convertToParamMap } from '@angular/router';
import type {
  ConsentAgreeResponse,
  ConsentViewResponse,
  ConsentWireResult,
} from 'elohim-imagodei/device-consent';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthStateService } from '../../services/auth-state.service';
import { DEVICE_CONSENT_PORT } from '../../services/device-consent-port';

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

const ok = <T>(body: T): ConsentWireResult<T> => ({ ok: true, body });
const refusal = (status: number, code?: string): ConsentWireResult<never> => ({
  ok: false,
  status,
  body: code ? { error: 'no', code } : null,
});

/** The node's answer to an approval, with its count and witnesses. */
const agreed = (partial: Partial<ConsentAgreeResponse>): ConsentAgreeResponse => ({
  returnTarget: { kind: 'display', value: 'K7QF' },
  expiresAt: Date.now() + 60_000,
  consentCid: 'bafyreiconsent',
  controllers: { required: 1, signed: 1 },
  witnesses: [],
  ...partial,
});

/** The doorway's mount of the shared approval page: sign-in first, words, selectors. */
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
        {
          provide: DEVICE_CONSENT_PORT,
          useValue: {
            client: { view: consent.view, agree: consent.agree },
            handBack: consent.handBack,
          },
        },
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
      view: vi.fn().mockResolvedValue(ok(VIEW)),
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

    it('goes back to sign-in when the session has run out, saying so', async () => {
      consent.view.mockResolvedValue(refusal(401));
      const fixture = create();
      await ready(fixture);

      expect(router.navigateByUrl).toHaveBeenCalledTimes(1);
      expect(card(fixture).phase).toBe('refused');
      expect(card(fixture).refusalCode).toBe('consent_not_signed_in');
    });
  });

  describe('reading the request', () => {
    it.each([
      ['absent', null],
      ['not base64url', '!!!'],
      ['not JSON', 'bm90IGpzb24'],
    ])('refuses an %s request without calling anything', async (_label, param) => {
      const fixture = create(param);
      await ready(fixture);

      expect(consent.view).not.toHaveBeenCalled();
      expect(card(fixture).phase).toBe('refused');
      expect(card(fixture).refusalCode).toBe('request_unreadable');
    });

    it('shows the runtime’s refusal code', async () => {
      consent.view.mockResolvedValue(refusal(400, 'request_acts_incoherent'));
      const fixture = create();
      await ready(fixture);

      expect(card(fixture).phase).toBe('refused');
      expect(card(fixture).refusalCode).toBe('request_acts_incoherent');
    });

    it('says approvals are unavailable when this doorway lacks the endpoint', async () => {
      consent.view.mockResolvedValue(refusal(404));
      const fixture = create();
      await ready(fixture);

      expect(card(fixture).refusalCode).toBe('consent_unavailable');
    });
  });

  describe('approving', () => {
    it('signs only what was agreed and shows the code to paste', async () => {
      consent.agree.mockResolvedValue(
        ok(
          agreed({
            returnTarget: { kind: 'display', value: 'K7QF-2MXD' },
            expiresAt: 1_791_000_300_000,
          })
        )
      );
      const fixture = create();
      await ready(fixture);

      approve(fixture, ['device.enroll']);
      expect(fixture.componentInstance.state()?.phase).toBe('signing');
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
      consent.agree.mockResolvedValue(ok(agreed({ returnTarget: { kind: 'redirect', url } })));
      const fixture = create();
      await ready(fixture);

      approve(fixture, ['device.enroll', 'device.bind-root']);
      await ready(fixture);

      expect(card(fixture).phase).toBe('handed-back');
      expect(consent.handBack).toHaveBeenCalledWith(url);
    });

    it('never follows a redirect to anywhere but this machine’s terminal', async () => {
      consent.agree.mockResolvedValue(
        ok(agreed({ returnTarget: { kind: 'redirect', url: 'https://evil.example/collect' } }))
      );
      const fixture = create();
      await ready(fixture);

      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(consent.handBack).not.toHaveBeenCalled();
      expect(card(fixture).phase).toBe('refused');
      expect(card(fixture).refusalCode).toBe('return_path_refused');
    });

    it('sends one approval even if approve fires twice', async () => {
      consent.agree.mockResolvedValue(ok(agreed({})));
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
      consent.agree.mockResolvedValue(ok(agreed({})));
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
        id: 'doorway-sign',
        act: 'signed',
        relation: 'your-doorway',
        label: 'alpha.elohim.host',
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
        id: 'key-holder-sign',
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

    it('shows the doorway signing, live, while signing — a wait, and on what', async () => {
      consent.agree.mockReturnValue(new Promise(() => undefined));
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      fixture.detectChanges();

      expect(trail(fixture)?.steps).toEqual([doorwayStep(fixture, 'working')]);
      expect(trail(fixture)?.getAttribute('mode')).toBeNull();
    });

    it('shows the witnesses the node reported instead of its own step, settled under the code', async () => {
      consent.agree.mockResolvedValue(ok(agreed({ witnesses: WITNESSES as never })));
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(trail(fixture)?.steps).toEqual(WITNESSES);
      expect(trail(fixture)?.getAttribute('mode')).toBe('settled');
    });

    it('settles under handed-back too, and hands back without waiting on it', async () => {
      const url = 'http://127.0.0.1:53682/cb';
      consent.agree.mockResolvedValue(ok(agreed({ returnTarget: { kind: 'redirect', url } })));
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(consent.handBack).toHaveBeenCalledWith(url);
      expect(trail(fixture)?.steps).toEqual([doorwayStep(fixture, 'done')]);
      expect(trail(fixture)?.getAttribute('mode')).toBe('settled');
    });

    it('marks the doorway step failed on a refusal', async () => {
      consent.agree.mockResolvedValue(refusal(400, 'act_unknown'));
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(card(fixture).phase).toBe('refused');
      expect(trail(fixture)?.steps).toEqual([doorwayStep(fixture, 'failed')]);
    });

    it('restores the settled trail from this tab without re-running anything', async () => {
      consent.agree.mockResolvedValue(ok(agreed({ witnesses: WITNESSES as never })));
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
      expect(trail(again)?.steps).toEqual(WITNESSES);
      expect(trail(again)?.getAttribute('mode')).toBe('settled');
    });
  });

  describe('what the doorway says', () => {
    function standing(fixture: ComponentFixture<DeviceConsentComponent>) {
      return fixture.nativeElement.querySelector(
        '[data-testid="device-consent-standing"]'
      ) as HTMLElement | null;
    }

    it('gives the card the doorway’s words: it holds the key and signs as the person', async () => {
      const fixture = create();
      await ready(fixture);
      const strings = (
        card(fixture) as HTMLElement & {
          strings?: { signerHosted?: (host?: string) => string };
        }
      ).strings;
      expect(strings?.signerHosted?.('alpha.elohim.host')).toBe(
        'Your doorway (alpha.elohim.host) holds your key, and will sign this as you when you approve.'
      );
    });

    it('says plainly that one of one is a complete approval', async () => {
      consent.agree.mockResolvedValue(ok(agreed({})));
      const fixture = create();
      await ready(fixture);
      expect(standing(fixture)).toBeNull();
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(standing(fixture)?.textContent).toContain(
        'Your doorway signed as you, and that is enough: this approval is complete.'
      );
    });

    it('names how many more of the person’s own devices must agree, only for their quorum', async () => {
      consent.agree.mockResolvedValue(ok(agreed({ controllers: { required: 3, signed: 1 } })));
      const fixture = create();
      await ready(fixture);
      approve(fixture, ['device.enroll']);
      await ready(fixture);

      expect(card(fixture).phase).toBe('code');
      expect(standing(fixture)?.textContent).toContain('2 more of your own devices must agree');
    });

    it('declining shows no standing line and no code', async () => {
      const fixture = create();
      await ready(fixture);
      card(fixture).dispatchEvent(new CustomEvent('decline'));
      fixture.detectChanges();
      expect(standing(fixture)).toBeNull();
      expect(card(fixture).code).toBeUndefined();
      expect(card(fixture).refusalCode).toBeUndefined();
    });
  });
});
