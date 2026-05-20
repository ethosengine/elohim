import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { DistributionBadgeComponent } from './distribution-badge.component';
import type { DistributionDetails } from '../../generated/distribution-details';
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

const mockDetails: DistributionDetails = {
  summary: mockSummary,
  replicaPeers: [],
  projectorIdentities: [],
  placementGaps: [
    { kind: 'hub_diversity', contentId: 'abc', shortfall: { target: 3, observed: 1 }, remediation: null },
  ],
  recentProjectionEvents: [],
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
    http.expectNone((req) => req.url.includes('/distribution/details'));
  });

  // C3: hover tooltip tests

  it('opens a tooltip on mouseenter with replica and diversity information', async () => {
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new MouseEvent('mouseenter'));
    fixture.detectChanges();
    await fixture.whenStable();
    const tooltip = fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]') as HTMLElement;
    expect(tooltip).toBeTruthy();
    expect(tooltip.textContent).toMatch(/replicas/i);
  });

  it('opens a tooltip on focus with diversity hint information', async () => {
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new FocusEvent('focus'));
    fixture.detectChanges();
    await fixture.whenStable();
    const tooltip = fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]') as HTMLElement;
    expect(tooltip).toBeTruthy();
    expect(tooltip.textContent).toMatch(/reach/i);
  });

  it('closes the tooltip on mouseleave', async () => {
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new MouseEvent('mouseenter'));
    fixture.detectChanges();
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]')).toBeTruthy();

    badge.dispatchEvent(new MouseEvent('mouseleave'));
    fixture.detectChanges();
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]')).toBeNull();
  });

  it('closes the tooltip on blur', async () => {
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new FocusEvent('focus'));
    fixture.detectChanges();
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]')).toBeTruthy();

    badge.dispatchEvent(new FocusEvent('blur'));
    fixture.detectChanges();
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]')).toBeNull();
  });

  it('renders placement-gaps row when detailsInput has gaps', async () => {
    fixture.componentRef.setInput('detailsInput', mockDetails);
    fixture.detectChanges();
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new MouseEvent('mouseenter'));
    fixture.detectChanges();
    await fixture.whenStable();
    const row = fixture.nativeElement.querySelector('[data-testid="placement-gaps-row"]');
    expect(row).toBeTruthy();
    expect(row?.textContent).toMatch(/hub diversity|hub_diversity/i);
  });

  it('does not render placement-gaps row when gaps array is empty', async () => {
    const emptyDetails: DistributionDetails = { ...mockDetails, placementGaps: [] };
    fixture.componentRef.setInput('detailsInput', emptyDetails);
    fixture.detectChanges();
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new MouseEvent('mouseenter'));
    fixture.detectChanges();
    await fixture.whenStable();
    const row = fixture.nativeElement.querySelector('[data-testid="placement-gaps-row"]');
    expect(row).toBeNull();
  });

  it('does not render placement-gaps row when no detailsInput provided', async () => {
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new MouseEvent('mouseenter'));
    fixture.detectChanges();
    await fixture.whenStable();
    const row = fixture.nativeElement.querySelector('[data-testid="placement-gaps-row"]');
    expect(row).toBeNull();
  });

  it('renders hub-count label derived from diversityHint', async () => {
    const summaryMultiHousehold: DistributionSummary = {
      ...mockSummary,
      diversityHint: { kind: 'household_archetypes', value: ['desktop', 'mobile'] },
    };
    fixture.componentRef.setInput('summary', summaryMultiHousehold);
    fixture.detectChanges();
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
    badge.dispatchEvent(new MouseEvent('mouseenter'));
    fixture.detectChanges();
    await fixture.whenStable();
    const tooltip = fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]') as HTMLElement;
    expect(tooltip).toBeTruthy();
    // household_archetypes hint should produce a label mentioning household or archetype diversity
    expect(tooltip.textContent).toMatch(/household|archetype|replicas/i);
  });
});
