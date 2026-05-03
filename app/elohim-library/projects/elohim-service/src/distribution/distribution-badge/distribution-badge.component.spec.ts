import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { DistributionBadgeComponent } from './distribution-badge.component';
import type { DistributionSummary } from '../../generated/distribution-summary';

const mockSummary: DistributionSummary = {
  replicaCount: 3,
  replicaTarget: 4,
  replicaHealth: 'at_risk',
  projectorCount: 1,
  reachClass: 'public',
  diversityHint: { kind: 'region_metro', value: ['us-central'] },
  thisFetchSource: 'projected_via_doorway',
  lastVerifiedSeconds: 30,
};

describe('DistributionBadgeComponent (elohim-library)', () => {
  let fixture: ComponentFixture<DistributionBadgeComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DistributionBadgeComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(DistributionBadgeComponent);
    http = TestBed.inject(HttpTestingController);
    fixture.componentRef.setInput('summary', mockSummary);
    fixture.detectChanges();
  });

  afterEach(() => http.verify());

  it('renders with data-testid="distribution-badge"', () => {
    expect(
      fixture.nativeElement.querySelector('[data-testid="distribution-badge"]'),
    ).toBeTruthy();
  });

  it('renders replica count', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="distribution-badge-replica-count"]');
    expect(el?.textContent?.trim()).toBe('3');
  });

  it('lazy-fetches details on first expand when blobHash is set', async () => {
    fixture.componentRef.setInput('blobHash', 'sha256-xyz');
    fixture.detectChanges();
    const root = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]');
    root.dispatchEvent(new MouseEvent('mouseenter'));
    await fixture.whenStable();
    const req = http.expectOne('/api/v1/blob/sha256-xyz/distribution/details');
    req.flush({
      summary: mockSummary,
      replicaPeers: [],
      projectorIdentities: [],
      placementGaps: [],
      recentProjectionEvents: [],
    });
  });

  it('does not fetch when blobHash is absent', async () => {
    const root = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]');
    root.dispatchEvent(new MouseEvent('mouseenter'));
    await fixture.whenStable();
    http.expectNone('/api/v1/blob/undefined/distribution/details');
  });
});
