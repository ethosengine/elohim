import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import type { DistributionSummary } from '@app/generated/distribution-summary';

import { DistributionBadgeComponent } from './distribution-badge.component';

function makeSummary(overrides: Partial<DistributionSummary> = {}): DistributionSummary {
  return {
    replicaCount: 12,
    replicaTarget: 14,
    replicaHealth: 'healthy',
    projectorCount: 2,
    reachClass: 'public',
    diversityHint: { kind: 'region_metro', value: ['us-central'] },
    thisFetchSource: 'projected_via_doorway',
    lastVerifiedSeconds: 60,
    ...overrides,
  };
}

describe('DistributionBadgeComponent', () => {
  let fixture: ComponentFixture<DistributionBadgeComponent>;
  let component: DistributionBadgeComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DistributionBadgeComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(DistributionBadgeComponent);
    component = fixture.componentInstance;
  });

  it('renders the replica count', () => {
    component.summary = makeSummary({ replicaCount: 12 });
    fixture.detectChanges();
    const el = fixture.nativeElement.querySelector(
      '[data-testid="distribution-badge-replica-count"]'
    );
    expect(el?.textContent).toContain('12');
  });

  it('applies the critical class when health is critical', () => {
    component.summary = makeSummary({
      replicaHealth: 'critical',
      replicaCount: 1,
      replicaTarget: 4,
    });
    fixture.detectChanges();
    const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]');
    expect(badge?.classList.contains('critical')).toBe(true);
  });

  it('renders private icon for private reach', () => {
    component.summary = makeSummary({ reachClass: 'private' });
    fixture.detectChanges();
    const reach = fixture.nativeElement.querySelector('[data-testid="distribution-badge-reach"]');
    expect(reach?.textContent).toMatch(/private/);
  });

  it('shows my-role star when viewer hosts this content', () => {
    component.summary = makeSummary({ myRole: 'replica' });
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="distribution-badge-my-role"]')
    ).toBeTruthy();
  });

  it('hides my-role star for visitors', () => {
    component.summary = makeSummary();
    delete component.summary.myRole;
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="distribution-badge-my-role"]')
    ).toBeFalsy();
  });
});
