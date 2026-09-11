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
      expect(component.isCurrentStep('key_export')).toBe(true);
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

  /** Fields S2 Task 13 adds to GET /auth/account; not yet in the generated view. */
  type AccountWire = AccountResponse & { hostedCellValidUntil?: string; displayName?: string };

  const HOSTED: AccountWire = {
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

  async function renderAccount(account: AccountWire): Promise<void> {
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
        hostedCellValidUntil: '2026-10-11T00:00:00Z',
      });

      const household = testId('account-hosted-by-household');
      const until = testId('account-hosted-until');
      expect(household?.textContent?.trim()).toBeTruthy();
      expect(until?.textContent?.trim()).toBeTruthy();
      // Never the opaque notary handle for the promise.
      expect(household?.textContent?.trim()).not.toMatch(/^u[A-Za-z0-9_-]{40,}$/);
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
