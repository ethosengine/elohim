import { HttpErrorResponse, provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter, Router } from '@angular/router';
import { of } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { AccountResponse } from '../../models/doorway.model';
import { DoorwayAdminService } from '../../services/doorway-admin.service';
import { AUTH_TOKEN_KEY } from '../../services/doorway-session-token.store';

import { DoorwayAccountComponent } from './doorway-account.component';

/**
 * Agency-pipeline coherence (genesis/a2o/features/auth/agency-pipeline-coherence.feature,
 * "Matthew's pipeline shows hosted-steward as an in-between state").
 *
 * doorway/account's pipeline and the elohim-app agency badge must not tell a
 * human two different stories about how far along they are. A steward whose
 * cell is still doorway-hosted is a HOSTED STEWARD: the badge says so, and this
 * page used to tick "Steward" complete and show a banner that named neither the
 * state nor the host.
 */
describe('DoorwayAccountComponent — hosted-steward is an in-between state', () => {
  let fixture: ComponentFixture<DoorwayAccountComponent>;
  let component: DoorwayAccountComponent;
  let originalLocation: Location;

  const BASE_ACCOUNT: AccountResponse = {
    humanId: 'human-matthew',
    identifier: 'matthew@alpha.elohim.host',
    permissionLevel: 'AUTHENTICATED',
    storageBytes: 1024,
    storageLimit: 10_240,
    storagePercent: 10,
    projectionQueries: 5,
    dailyQueryLimit: 100,
    queriesPercent: 5,
    bandwidthBytes: 2048,
    dailyBandwidthLimit: 20_480,
    bandwidthPercent: 10,
    isSteward: false,
    keyExported: true,
    createdAt: '2026-01-01T00:00:00Z',
    lastLoginAt: '2026-09-01T00:00:00Z',
  };

  async function renderWith(account: AccountResponse): Promise<void> {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [DoorwayAccountComponent],
      providers: [
        provideRouter([]),
        provideHttpClient(),
        provideHttpClientTesting(),
        {
          provide: DoorwayAdminService,
          useValue: {
            getAccount: vi.fn().mockReturnValue(of(account)),
            getPortalHostUrl: vi.fn().mockResolvedValue({ hostUrl: null }),
            mintSessionToken: vi.fn(),
          },
        },
      ],
    });

    fixture = TestBed.createComponent(DoorwayAccountComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    originalLocation = globalThis.location;
    Object.defineProperty(globalThis, 'location', {
      value: {
        href: 'https://doorway-alpha.elohim.host/threshold/account',
        origin: 'https://doorway-alpha.elohim.host',
        hostname: 'doorway-alpha.elohim.host',
      },
      writable: true,
      configurable: true,
    });
  });

  afterEach(() => {
    Object.defineProperty(globalThis, 'location', {
      value: originalLocation,
      writable: true,
      configurable: true,
    });
  });

  describe('a steward whose cell is still doorway-hosted', () => {
    beforeEach(async () => {
      await renderWith({ ...BASE_ACCOUNT, isSteward: true, conductorId: 'conductor-alpha-1' });
    });

    it('does NOT mark the Steward step completed', () => {
      expect(component.stewardAccessingThroughDoorway()).toBe(true);
      expect(component.isStepCompleted('steward')).toBe(false);
      expect(component.isCurrentStep('steward')).toBe(true);
      // Hosted is still done — they did create an account on a doorway.
      expect(component.isStepCompleted('hosted')).toBe(true);
    });

    it('names the state "Hosted Steward" and the doorway host in the banner', () => {
      const banner: HTMLElement | null = fixture.nativeElement.querySelector('.context-banner');
      expect(banner).not.toBeNull();

      const text = (banner?.textContent ?? '').replaceAll(/\s+/g, ' ').trim();
      expect(text).toContain('Hosted Steward');
      expect(text).toContain('Accessing through alpha.elohim.host');
    });

    it('derives the gateway domain the way threshold-login does', () => {
      expect(component.gatewayDomain()).toBe('alpha.elohim.host');
    });
  });

  describe('a graduated steward running their own conductor', () => {
    beforeEach(async () => {
      await renderWith({ ...BASE_ACCOUNT, isSteward: true });
    });

    it('marks the Steward step completed and shows no in-between banner', () => {
      expect(component.stewardAccessingThroughDoorway()).toBe(false);
      expect(component.isStepCompleted('steward')).toBe(true);
      expect(fixture.nativeElement.querySelector('.context-banner')).toBeNull();
    });
  });

  describe('a hosted visitor (rendering unchanged)', () => {
    beforeEach(async () => {
      await renderWith({ ...BASE_ACCOUNT, conductorId: 'conductor-alpha-1', keyExported: false });
    });

    it('shows no hosted-steward affordance and keeps the graduation CTA', () => {
      expect(component.stewardAccessingThroughDoorway()).toBe(false);
      expect(component.isStepCompleted('steward')).toBe(false);
      expect(fixture.nativeElement.querySelector('.context-banner')).toBeNull();
      expect(fixture.nativeElement.querySelector('.graduation-cta')).not.toBeNull();
      // A human who has only ever been hosted is AT "Hosted". Key export is the
      // next GATE — the graduation CTA above names it — not where they are.
      // See `isCurrentStep`, and hosted-human/05-leaving.feature.
      expect(component.isCurrentStep('hosted')).toBe(true);
      expect(component.isCurrentStep('key_export')).toBe(false);
    });
  });
});

/**
 * Closing a hosted account (genesis/a2o/features/auth/hosted-human/05-leaving.feature,
 * "Closing the account asks the human to confirm, then signs them out") and the
 * hosting strip (07-hosted-by-a-household.feature, "The account page says which
 * household is hosting them").
 *
 * The `data-testid`s asserted here are the ones the committed a2o glue reads
 * (genesis/a2o/steps/ui/hosted-human.steps.ts, `TEST_ID`): account-close-begin,
 * account-close-confirm-input, account-close-confirm, account-close-error,
 * account-hosted-by-household, account-hosted-until.
 *
 * Confirmation is the DOORWAY's judgement, not the page's: the glue's
 * `submitClosure` waits for a `/auth/close-account` response and asserts a 400
 * with `code: CONFIRMATION_MISMATCH` on a wrong identifier. So a mismatch is
 * posted and refused, never pre-judged in TypeScript.
 */
describe('DoorwayAccountComponent — closing an account and the hosting strip', () => {
  let fixture: ComponentFixture<DoorwayAccountComponent>;
  let component: DoorwayAccountComponent;
  let closeAccount: ReturnType<typeof vi.fn>;
  let navigate: ReturnType<typeof vi.fn>;
  let originalLocation: Location;

  /**
   * The generated wire view itself (S2 Task 13b): `displayName`,
   * `hostedCellGrantCid`, `hostedCellValidUntil` and `hostedByHousehold` are
   * all schema-carried now, so the page reads them directly instead of through
   * a hand-written overlay.
   */
  const HOSTED: AccountResponse = {
    humanId: 'human-stranger',
    identifier: 'stranger@alpha.elohim.host',
    permissionLevel: 'AUTHENTICATED',
    storageBytes: 0,
    storageLimit: 10_240,
    storagePercent: 0,
    projectionQueries: 0,
    dailyQueryLimit: 100,
    queriesPercent: 0,
    bandwidthBytes: 0,
    dailyBandwidthLimit: 20_480,
    bandwidthPercent: 0,
    isSteward: false,
    keyExported: false,
  };

  function testId(id: string): HTMLElement | null {
    return fixture.nativeElement.querySelector(`[data-testid="${id}"]`);
  }

  async function renderAccount(account: AccountResponse): Promise<void> {
    TestBed.resetTestingModule();
    closeAccount = vi.fn().mockResolvedValue({
      closed: true,
      cellUninstalled: true,
      alreadyClosed: false,
    });
    TestBed.configureTestingModule({
      imports: [DoorwayAccountComponent],
      providers: [
        provideRouter([]),
        provideHttpClient(),
        provideHttpClientTesting(),
        {
          provide: DoorwayAdminService,
          useValue: {
            getAccount: vi.fn().mockReturnValue(of(account)),
            getPortalHostUrl: vi.fn().mockResolvedValue({ hostUrl: null }),
            mintSessionToken: vi.fn(),
            closeAccount,
          },
        },
      ],
    });
    navigate = vi.fn().mockResolvedValue(true);
    vi.spyOn(TestBed.inject(Router), 'navigate').mockImplementation(
      navigate as unknown as Router['navigate']
    );

    fixture = TestBed.createComponent(DoorwayAccountComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  async function settle(): Promise<void> {
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    originalLocation = globalThis.location;
    Object.defineProperty(globalThis, 'location', {
      value: {
        href: 'https://doorway-alpha.elohim.host/threshold/account',
        origin: 'https://doorway-alpha.elohim.host',
        hostname: 'doorway-alpha.elohim.host',
      },
      writable: true,
      configurable: true,
    });
    localStorage.setItem(AUTH_TOKEN_KEY, 'a-session-jwt');
  });

  afterEach(() => {
    localStorage.removeItem(AUTH_TOKEN_KEY);
    Object.defineProperty(globalThis, 'location', {
      value: originalLocation,
      writable: true,
      configurable: true,
    });
  });

  describe('the close-account section', () => {
    beforeEach(async () => {
      await renderAccount({ ...HOSTED, conductorId: 'conductor-alpha-1' });
    });

    it('is offered to every signed-in human, below the graduation CTA', () => {
      expect(testId('account-close-begin')).not.toBeNull();
      // The confirmation is a deliberate second step, not a stray input.
      expect(testId('account-close-confirm-input')).toBeNull();
      expect(testId('account-close-confirm')).toBeNull();
    });

    it('asks the human to type their identifier once they begin', async () => {
      testId('account-close-begin')?.click();
      await settle();

      expect(testId('account-close-confirm-input')).not.toBeNull();
      expect(testId('account-close-confirm')).not.toBeNull();
      expect(testId('account-close-error')).toBeNull();
    });

    it('lets the doorway refuse a wrong identifier, and stays signed in', async () => {
      closeAccount.mockRejectedValueOnce(
        new HttpErrorResponse({
          status: 400,
          error: { code: 'CONFIRMATION_MISMATCH', message: 'That is not your identifier.' },
        })
      );
      testId('account-close-begin')?.click();
      await settle();
      component.confirmIdentifier.set('wrong@alpha.elohim.host');
      testId('account-close-confirm')?.click();
      await settle();

      expect(closeAccount).toHaveBeenCalledExactlyOnceWith('wrong@alpha.elohim.host');
      expect(testId('account-close-error')).not.toBeNull();
      expect(navigate).not.toHaveBeenCalled();
      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('a-session-jwt');
    });

    it('closes, signs the human out locally, and returns to the doorway landing', async () => {
      testId('account-close-begin')?.click();
      await settle();
      component.confirmIdentifier.set('stranger@alpha.elohim.host');
      testId('account-close-confirm')?.click();
      await settle();

      expect(closeAccount).toHaveBeenCalledExactlyOnceWith('stranger@alpha.elohim.host');
      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
      expect(navigate).toHaveBeenCalledWith(['/']);
      expect(testId('account-close-error')).toBeNull();
    });
  });

  describe('the hosted-by-a-household strip', () => {
    it('names the hosting household and the date the hosting is promised until', async () => {
      await renderAccount({
        ...HOSTED,
        conductorId: 'conductor-alpha-1',
        hostedByHousehold: 'The Ellis Household',
        hostedCellGrantCid: 'uhCEkHostedCellGrantHandleThatIsNotAName0000',
        hostedCellValidUntil: '2026-10-11T00:00:00Z',
      });

      const household = testId('account-hosted-by-household');
      const until = testId('account-hosted-until');
      expect(household?.textContent?.trim()).toBe('The Ellis Household');
      expect(until?.textContent?.trim()).toBeTruthy();
      // Never the opaque notary handle for the promise.
      expect(household?.textContent?.trim()).not.toMatch(/^u[A-Za-z0-9_-]{40,}$/);
    });

    it('names the household, never the machine it runs on', async () => {
      await renderAccount({
        ...HOSTED,
        conductorId: 'conductor-alpha-1',
        hostedCellValidUntil: '2026-10-11T00:00:00Z',
      });

      // A conductor id is a MACHINE. The wire carries no household name here,
      // so the row is absent rather than standing in the machine's name —
      // `07-hosted-by-a-household.feature` asks for the name a person uses.
      expect(testId('account-hosted-by-household')).toBeNull();
      expect(testId('account-hosted-until')?.textContent?.trim()).toBeTruthy();
    });

    it('is absent — not blank — when the account response carries no hosting facts', async () => {
      await renderAccount(HOSTED);

      expect(testId('account-hosted-by-household')).toBeNull();
      expect(testId('account-hosted-until')).toBeNull();
    });

    it('shows only the promised-until row when the household is not yet named', async () => {
      await renderAccount({ ...HOSTED, hostedCellValidUntil: '2026-10-11T00:00:00Z' });

      expect(testId('account-hosted-by-household')).toBeNull();
      expect(testId('account-hosted-until')?.textContent?.trim()).toBeTruthy();
    });
  });
});

/**
 * The agency pipeline a hosted human actually sees
 * (genesis/a2o/features/auth/hosted-human/05-leaving.feature, "A newcomer creates
 * an account, is hosted as themselves, and closes it again").
 *
 * Two claims the household run 20260911T041337Z-faca0d95 measured RED at once:
 *
 *  1. ONE BAD INSTANT MUST NOT BLANK THE PAGE. The doorway serves `createdAt`
 *     as `2026-09-11 4:36:34.217 +00:00:00` (the time crate's `Display`, not
 *     RFC3339 — `doorway-service/src/routes/auth_routes.rs` builds it with
 *     `d.to_string()`). `DatePipe` THROWS `NG02100` on a string it cannot parse,
 *     and a throw part-way through this template abandons everything below it:
 *     the gauges, the stepper, the CTA. The a2o glue reads `.step` / `.step-label`
 *     and found NO steps at all — "Agency pipeline has no "Hosted" step" is what a
 *     crashed render looks like from outside.
 *
 *  2. "HOSTED" IS THE STEP A HOSTED HUMAN IS AT. The story calls it "the first
 *     stage of agency"; the pipeline marks the stage the human is IN, not the
 *     next gate they could pass. This is the axis `agency-pipeline-coherence.feature`
 *     left open — 05-leaving, which is not @wip, decides it for the stage.
 */
describe('DoorwayAccountComponent — the agency pipeline a hosted human sees', () => {
  let fixture: ComponentFixture<DoorwayAccountComponent>;
  let originalLocation: Location;

  /** A freshly registered hosted human, exactly as `/auth/account` answers. */
  const FRESHLY_HOSTED: AccountResponse = {
    humanId: 'human-newcomer',
    identifier: 'newcomer@alpha.elohim.host',
    permissionLevel: 'AUTHENTICATED',
    storageBytes: 0,
    storageLimit: 104_857_600,
    storagePercent: 0,
    projectionQueries: 0,
    dailyQueryLimit: 1000,
    queriesPercent: 0,
    bandwidthBytes: 0,
    dailyBandwidthLimit: 524_288_000,
    bandwidthPercent: 0,
    conductorId: 'conductor-0',
    isSteward: false,
    keyExported: false,
    displayName: 'Newcomer',
    createdAt: '2026-09-11T04:36:34Z',
    lastLoginAt: '2026-09-11T04:36:34Z',
  };

  /** What the a2o glue reads: every `.step`, its label, and its classes. */
  function pipelineSteps(): { label: string; classes: string[] }[] {
    return [...fixture.nativeElement.querySelectorAll('.step')].map((step: HTMLElement) => ({
      label: step.querySelector<HTMLElement>('.step-label')?.textContent?.trim() ?? '',
      classes: [...step.classList],
    }));
  }

  async function renderAccount(account: AccountResponse): Promise<void> {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [DoorwayAccountComponent],
      providers: [
        provideRouter([]),
        provideHttpClient(),
        provideHttpClientTesting(),
        {
          provide: DoorwayAdminService,
          useValue: {
            getAccount: vi.fn().mockReturnValue(of(account)),
            getPortalHostUrl: vi.fn().mockResolvedValue({ hostUrl: null }),
            mintSessionToken: vi.fn(),
            closeAccount: vi.fn(),
          },
        },
      ],
    });

    fixture = TestBed.createComponent(DoorwayAccountComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    originalLocation = globalThis.location;
    Object.defineProperty(globalThis, 'location', {
      value: {
        href: 'https://doorway-alpha.elohim.host/threshold/account',
        origin: 'https://doorway-alpha.elohim.host',
        hostname: 'doorway-alpha.elohim.host',
      },
      writable: true,
      configurable: true,
    });
  });

  afterEach(() => {
    Object.defineProperty(globalThis, 'location', {
      value: originalLocation,
      writable: true,
      configurable: true,
    });
  });

  it('marks "Hosted" as the step a freshly hosted human is at', async () => {
    await renderAccount(FRESHLY_HOSTED);
    const steps = pipelineSteps();

    expect(steps.map(step => step.label)).toEqual([
      'Hosted',
      'Key Export',
      'Install App',
      'Steward',
    ]);
    const hosted = steps.find(step => step.label === 'Hosted');
    expect(hosted?.classes).toContain('current');
    // Exactly one step is where the human IS.
    expect(steps.filter(step => step.classes.includes('current'))).toHaveLength(1);
  });

  it('marks no step after the current one completed', async () => {
    await renderAccount(FRESHLY_HOSTED);
    const steps = pipelineSteps();
    const current = steps.findIndex(step => step.classes.includes('current'));

    expect(current).toBeGreaterThanOrEqual(0);
    expect(steps.slice(current + 1).every(step => !step.classes.includes('completed'))).toBe(true);
  });

  it('still renders the pipeline when the doorway sends an instant DatePipe cannot read', async () => {
    // Measured on the household mesh, run 20260911T041337Z-faca0d95.
    await renderAccount({
      ...FRESHLY_HOSTED,
      createdAt: '2026-09-11 4:36:34.217 +00:00:00',
      lastLoginAt: '2026-09-11 4:36:34.217 +00:00:00',
    });

    expect(pipelineSteps().map(step => step.label)).toContain('Hosted');
    // Everything below the bad row survives it.
    expect(fixture.nativeElement.querySelector('.gauge-grid')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('.close-account')).not.toBeNull();
    // The unreadable instant is said to be unknown, not silently shown as blank.
    const text = (fixture.nativeElement.textContent ?? '').replaceAll(/\s+/g, ' ');
    expect(text).toContain('Member Since —');
  });

  it('reads a hosted human’s promised-until date off the wire', async () => {
    await renderAccount({ ...FRESHLY_HOSTED, hostedCellValidUntil: '2026-10-11T04:36:33Z' });

    const until: HTMLElement | null = fixture.nativeElement.querySelector(
      '[data-testid="account-hosted-until"]'
    );
    expect(until?.textContent?.trim()).toBeTruthy();
  });
});
