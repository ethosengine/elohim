import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { Component, signal } from '@angular/core';

import type { DiscoveryResult } from '@app/lamad/quiz-engine/models/discovery-assessment.model';
import { DiscoveryAttestationService } from '@app/lamad/quiz-engine/services/discovery-attestation.service';

import { ProfileDiscoverySectionComponent } from './profile-discovery-section.component';
import { vi } from 'vitest';

describe('ProfileDiscoverySectionComponent', () => {
  let component: ProfileDiscoverySectionComponent;
  let fixture: ComponentFixture<TestHostComponent>;
  let mockDiscoveryService: any;
  let mockRouter: any;

  const buildResult = (overrides: Partial<DiscoveryResult> = {}): DiscoveryResult => ({
    id: 'discovery-1',
    assessmentId: 'assessment-enneagram',
    assessmentTitle: 'Enneagram Assessment',
    framework: 'enneagram',
    category: 'personality',
    humanId: 'human-123',
    completedAt: '2026-01-15T10:00:00.000Z',
    primaryType: { typeId: 'type-4', name: 'Individualist', score: 0.85 },
    subscaleScores: { type4: 0.85 },
    displayString: 'Type 4w5',
    shortDisplay: '4w5',
    ...overrides,
  });

  // Test host to set required inputs
  @Component({
    standalone: true,
    imports: [ProfileDiscoverySectionComponent],
    template: `
      <app-profile-discovery-section
        [results]="results()"
        (navigateToDiscovery)="navigated = true"
      />
    `,
  })
  class TestHostComponent {
    results = signal<DiscoveryResult[]>([]);
    navigated = false;
  }

  beforeEach(async () => {
    mockDiscoveryService = {
      getBadgeDisplay: vi.fn(),
      results: signal([]),
      attestations: signal([]),
      featuredResults: signal([]),
      resultsByCategory: signal({}),
      resultsByFramework: signal(new Map()),
      aggregatableResults: signal([]),
    };
    mockDiscoveryService.getBadgeDisplay.mockReturnValue({
      label: '4w5',
      icon: '🎭',
      color: '#8B5CF6',
    });

    mockRouter = {
    navigate: vi.fn(),
  };
    mockRouter.navigate.mockReturnValue(Promise.resolve(true));

    await TestBed.configureTestingModule({
      imports: [TestHostComponent, ProfileDiscoverySectionComponent],
      providers: [
        { provide: DiscoveryAttestationService, useValue: mockDiscoveryService },
        { provide: Router, useValue: mockRouter },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(TestHostComponent);
    component = fixture.debugElement.children[0].componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('empty state', () => {
    it('should show empty state when no results', () => {
      fixture.detectChanges();
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('.discovery-empty')).toBeTruthy();
      expect(el.querySelector('.discovery-card')).toBeNull();
    });

    it('should show start button in empty state', () => {
      fixture.detectChanges();
      const btn = fixture.nativeElement.querySelector('[data-testid="discovery-start-btn"]');
      expect(btn).toBeTruthy();
      expect(btn.textContent).toContain('Start a Discovery Assessment');
    });
  });

  describe('with results', () => {
    beforeEach(() => {
      const host = fixture.componentInstance;
      host.results.set([buildResult()]);
      fixture.detectChanges();
    });

    it('should render discovery cards', () => {
      const cards = fixture.nativeElement.querySelectorAll('[data-testid="discovery-card"]');
      expect(cards.length).toBe(1);
    });

    it('should display framework name', () => {
      const framework = fixture.nativeElement.querySelector('.discovery-framework');
      expect(framework.textContent).toBeTruthy();
    });

    it('should display result string', () => {
      const result = fixture.nativeElement.querySelector('.discovery-result');
      expect(result.textContent).toContain('Type 4w5');
    });

    it('should render hex badge with short display', () => {
      const badge = fixture.nativeElement.querySelector('[data-testid="discovery-hex-badge"]');
      expect(badge).toBeTruthy();
      expect(badge.textContent.trim()).toBe('4w5');
    });

    it('should show add button', () => {
      const btn = fixture.nativeElement.querySelector('[data-testid="discovery-add-btn"]');
      expect(btn).toBeTruthy();
    });
  });

  describe('getBadgeColor()', () => {
    it('should delegate to discovery service', () => {
      const result = buildResult();
      const color = component.getBadgeColor(result);

      expect(mockDiscoveryService.getBadgeDisplay).toHaveBeenCalledWith(result);
      expect(color).toBe('#8B5CF6');
    });

    it('should return different colors for different categories', () => {
      mockDiscoveryService.getBadgeDisplay.mockReturnValue({
        label: 'test',
        icon: '💪',
        color: '#10B981',
      });

      const color = component.getBadgeColor(buildResult({ category: 'strengths' }));
      expect(color).toBe('#10B981');
    });
  });

  describe('hex badge coloring', () => {
    it('should set --badge-color CSS custom property', () => {
      fixture.componentInstance.results.set([buildResult()]);
      fixture.detectChanges();

      const badge = fixture.nativeElement.querySelector('[data-testid="discovery-hex-badge"]');
      expect(badge.style.getPropertyValue('--badge-color')).toBe('#8B5CF6');
    });
  });

  describe('hasLink()', () => {
    it('should return true when contentNodeId is present', () => {
      expect(component.hasLink(buildResult({ contentNodeId: 'content-123' }))).toBe(true);
    });

    it('should return false when contentNodeId is absent', () => {
      expect(component.hasLink(buildResult())).toBe(false);
    });

    it('should return false when contentNodeId is empty string', () => {
      expect(component.hasLink(buildResult({ contentNodeId: '' }))).toBe(false);
    });
  });

  describe('navigateToResult()', () => {
    it('should navigate to resource when contentNodeId is present', () => {
      component.navigateToResult(buildResult({ contentNodeId: 'content-enneagram-001' }));

      expect(mockRouter.navigate).toHaveBeenCalledWith([
        '/lamad/resource',
        'content-enneagram-001',
      ]);
    });

    it('should emit navigateToDiscovery when contentNodeId is absent', () => {
      fixture.detectChanges();
      const host = fixture.componentInstance;
      host.navigated = false;

      component.navigateToResult(buildResult());

      expect(mockRouter.navigate).not.toHaveBeenCalled();
      expect(host.navigated).toBe(true);
    });
  });

  describe('link icon rendering', () => {
    it('should show link icon for results with contentNodeId', () => {
      fixture.componentInstance.results.set([
        buildResult({ contentNodeId: 'content-enneagram-001' }),
      ]);
      fixture.detectChanges();

      const icon = fixture.nativeElement.querySelector('.discovery-link-icon');
      expect(icon).toBeTruthy();
      expect(icon.textContent).toContain('open_in_new');
    });

    it('should not show link icon for results without contentNodeId', () => {
      fixture.componentInstance.results.set([buildResult()]);
      fixture.detectChanges();

      const icon = fixture.nativeElement.querySelector('.discovery-link-icon');
      expect(icon).toBeNull();
    });
  });

  describe('linked card modifier', () => {
    it('should add linked class when contentNodeId is present', () => {
      fixture.componentInstance.results.set([
        buildResult({ contentNodeId: 'content-enneagram-001' }),
      ]);
      fixture.detectChanges();

      const card = fixture.nativeElement.querySelector('[data-testid="discovery-card"]');
      expect(card.classList.contains('discovery-card--linked')).toBe(true);
    });

    it('should not add linked class when contentNodeId is absent', () => {
      fixture.componentInstance.results.set([buildResult()]);
      fixture.detectChanges();

      const card = fixture.nativeElement.querySelector('[data-testid="discovery-card"]');
      expect(card.classList.contains('discovery-card--linked')).toBe(false);
    });
  });

  describe('multiple results', () => {
    it('should render a card for each result', () => {
      fixture.componentInstance.results.set([
        buildResult({ id: 'r1', framework: 'enneagram' }),
        buildResult({ id: 'r2', framework: 'mbti', displayString: 'ISTP', shortDisplay: 'ISTP' }),
      ]);
      fixture.detectChanges();

      const cards = fixture.nativeElement.querySelectorAll('[data-testid="discovery-card"]');
      expect(cards.length).toBe(2);
    });
  });
});
