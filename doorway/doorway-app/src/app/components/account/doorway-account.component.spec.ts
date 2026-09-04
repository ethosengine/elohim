import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { AccountResponse } from '../../models/doorway.model';
import { DoorwayAdminService } from '../../services/doorway-admin.service';

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
