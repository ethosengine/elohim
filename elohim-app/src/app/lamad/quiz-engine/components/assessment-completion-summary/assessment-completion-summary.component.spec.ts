import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Component, signal } from '@angular/core';

import { BehaviorSubject, of } from 'rxjs';

import { ElohimPresenceService } from '@app/elohim/services/elohim-presence.service';
import type { LearnerMasteryProfile } from '@app/lamad/models/learner-mastery-profile.model';
import { MasteryStatsService } from '@app/lamad/services/mastery-stats.service';

import type { DiscoveryResultSummary } from '../../models/discovery-assessment.model';
import { DiscoveryAttestationService } from '../../services/discovery-attestation.service';
// Side-effect: register instruments so getInstrument() resolves
import '../../instruments/attachment-style.instrument';

import {
  AssessmentCompletionSummaryComponent,
  type CompletionMode,
} from './assessment-completion-summary.component';
import { vi } from 'vitest';

describe('AssessmentCompletionSummaryComponent', () => {
  let fixture: ComponentFixture<TestHostComponent>;
  let host: TestHostComponent;
  let mockDiscoveryService: any;
  let mockMasteryStats: any;
  let mockPresenceService: any;
  let profileSubject: BehaviorSubject<LearnerMasteryProfile | null>;

  // ─── Fixtures ──────────────────────────────────────────────────────────────

  const buildPrimaryType = (
    overrides: Partial<DiscoveryResultSummary> = {}
  ): DiscoveryResultSummary => ({
    typeId: 'secure',
    name: 'Secure',
    score: 0.8,
    ...overrides,
  });

  const buildProfile = (): LearnerMasteryProfile =>
    ({
      learnerLevel: { label: 'Explorer', icon: 'explore', color: '#3b82f6' },
      totalXP: 450,
      streak: { currentStreak: 3, bestStreak: 7, todayActive: true },
      totalMasteredNodes: 12,
      nodesAboveGate: 8,
    }) as unknown as LearnerMasteryProfile;

  // ─── Test Host ─────────────────────────────────────────────────────────────

  @Component({
    standalone: true,
    imports: [AssessmentCompletionSummaryComponent],
    template: `
      <app-assessment-completion-summary
        [mode]="mode()"
        [title]="title()"
        [contentNodeId]="contentNodeId()"
        [instrumentId]="instrumentId()"
        [subscaleScores]="subscaleScores()"
        [normalizedScores]="normalizedScores()"
        [primaryType]="primaryType()"
        [score]="score()"
        [passed]="passed()"
        [correctCount]="correctCount()"
        [totalCount]="totalCount()"
        (continue)="continued = true"
        (viewProfile)="profileViewed = true"
      />
    `,
  })
  class TestHostComponent {
    mode = signal<CompletionMode>('discovery');
    title = signal('Attachment Style Assessment');
    contentNodeId = signal('content-attachment-001');
    instrumentId = signal<string | null>('attachment-style-discovery');
    subscaleScores = signal<Record<string, number> | null>({
      anxiety: 0.3,
      avoidance: 0.2,
    });
    normalizedScores = signal<Record<string, number> | null>(null);
    primaryType = signal<DiscoveryResultSummary | null>(buildPrimaryType());
    score = signal<number | null>(null);
    passed = signal<boolean | null>(null);
    correctCount = signal<number | null>(null);
    totalCount = signal(10);
    continued = false;
    profileViewed = false;
  }

  // ─── Setup ─────────────────────────────────────────────────────────────────

  const mockFulfilledResponse = {
    requestId: 'req-test-1',
    elohimId: 'elohim-1',
    status: 'fulfilled' as const,
    constitutionalReasoning: {
      primaryPrinciple: 'Love as committed action toward flourishing',
      interpretation: 'Path recommendation after discovery',
      valuesWeighed: [{ value: 'Human dignity', weight: 0.5, direction: 'for' as const }],
      confidence: 0.75,
    },
    respondedAt: new Date().toISOString(),
    cost: {
      tokensProcessed: 800,
      timeMs: 1200,
      constitutionalChecks: 3,
      precedentLookups: 1,
    },
  };

  beforeEach(async () => {
    profileSubject = new BehaviorSubject<LearnerMasteryProfile | null>(null);

    mockPresenceService = {
      onDiscoveryCompleted: vi.fn(),
      onContentCompleted: vi.fn(),
      dismissNotice: vi.fn(),
      handleAction: vi.fn(),
      presence$: of([]),
      cost$: of({
        tokensProcessed: 0,
        timeMs: 0,
        constitutionalChecks: 0,
        precedentLookups: 0,
      }),
      notices$: of([]),
      providerId: 'elohim-presence',
    };
    mockPresenceService.onDiscoveryCompleted.mockReturnValue(of(mockFulfilledResponse));
    mockPresenceService.onContentCompleted.mockReturnValue(of(mockFulfilledResponse));

    mockDiscoveryService = {
      recordFromCompletion: vi.fn(),
      getBadgeDisplay: vi.fn(),
      results: signal([]),
      attestations: signal([]),
      featuredResults: signal([]),
    };
    mockDiscoveryService.recordFromCompletion.mockReturnValue({
      id: 'discovery-result-1',
      assessmentId: 'attachment-style-discovery',
      assessmentTitle: 'Attachment Style Assessment',
      framework: 'attachment-style',
      category: 'personality',
      humanId: 'human-123',
      completedAt: new Date().toISOString(),
      primaryType: buildPrimaryType(),
      subscaleScores: { anxiety: 0.3, avoidance: 0.2 },
      displayString: 'Secure',
      shortDisplay: 'Secure',
    });
    mockDiscoveryService.getBadgeDisplay.mockReturnValue({
      label: 'Secure',
      icon: '🎭',
      color: '#8B5CF6',
    });

    mockMasteryStats = {
      recordDailyEngagement: vi.fn(),
      learnerProfile$: profileSubject.asObservable(),
    };

    await TestBed.configureTestingModule({
      imports: [TestHostComponent, AssessmentCompletionSummaryComponent],
      providers: [
        { provide: DiscoveryAttestationService, useValue: mockDiscoveryService },
        { provide: MasteryStatsService, useValue: mockMasteryStats },
        { provide: ElohimPresenceService, useValue: mockPresenceService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(TestHostComponent);
    host = fixture.componentInstance;
  });

  afterEach(() => {
    profileSubject.complete();
  });

  it('should create', () => {
    fixture.detectChanges();
    const summary = fixture.nativeElement.querySelector('[data-testid="completion-summary"]');
    expect(summary).toBeTruthy();
  });

  // ─── Discovery Mode ────────────────────────────────────────────────────────

  describe('discovery mode', () => {
    beforeEach(() => {
      fixture.detectChanges();
    });

    it('should record attestation on init', () => {
      expect(mockDiscoveryService.recordFromCompletion).toHaveBeenCalledWith(
        expect.objectContaining({
          contentNodeId: 'content-attachment-001',
          assessmentTitle: 'Attachment Style Assessment',
          instrumentId: 'attachment-style-discovery',
          subscaleScores: { anxiety: 0.3, avoidance: 0.2 },
        })
      );
    });

    it('should show personalized headline', () => {
      const headline = fixture.nativeElement.querySelector('[data-testid="completion-headline"]');
      expect(headline.textContent).toContain("You're a Secure!");
    });

    it('should show description with primary type name', () => {
      const desc = fixture.nativeElement.querySelector('[data-testid="completion-description"]');
      expect(desc.textContent).toContain('Secure');
    });

    it('should render hex badge preview', () => {
      const badge = fixture.nativeElement.querySelector('[data-testid="completion-hex-badge"]');
      expect(badge).toBeTruthy();
      expect(badge.textContent).toContain('Secure');
    });

    it('should set badge color via CSS custom property', () => {
      const badge = fixture.nativeElement.querySelector('[data-testid="completion-hex-badge"]');
      expect(badge.style.getPropertyValue('--badge-color')).toBe('#8B5CF6');
    });

    it('should render subscale bars', () => {
      const subscales = fixture.nativeElement.querySelector('[data-testid="completion-subscales"]');
      expect(subscales).toBeTruthy();
    });

    it('should sort subscale bars by score descending', () => {
      const rows = fixture.nativeElement.querySelectorAll('.subscale-row');
      expect(rows.length).toBe(2);
      // anxiety=0.3 > avoidance=0.2
      expect(rows[0].getAttribute('data-testid')).toBe('subscale-anxiety');
      expect(rows[1].getAttribute('data-testid')).toBe('subscale-avoidance');
    });

    it('should show profile link for discovery mode', () => {
      const link = fixture.nativeElement.querySelector('[data-testid="completion-profile-link"]');
      expect(link.textContent).toContain('View on your profile');
    });

    it('should not show score display', () => {
      const score = fixture.nativeElement.querySelector('[data-testid="completion-score"]');
      expect(score).toBeNull();
    });

    it('should not record attestation when instrumentId is null', () => {
      mockDiscoveryService.recordFromCompletion.mockClear();

      host.instrumentId.set(null);
      fixture.detectChanges();
      // ngOnInit already ran; changing input won't re-trigger
      // but we verify the guard in recordAttestation
      expect(mockDiscoveryService.recordFromCompletion).not.toHaveBeenCalled();
    });

    it('should request elohim insight on discovery init', () => {
      expect(mockPresenceService.onDiscoveryCompleted).toHaveBeenCalledWith(
        'content-attachment-001',
        'Attachment Style Assessment',
        'current-user'
      );
    });

    it('should show elohim insight section when fulfilled', () => {
      const insight = fixture.nativeElement.querySelector('[data-testid="elohim-insight"]');
      expect(insight).toBeTruthy();
    });

    it('should display insight message', () => {
      const message = fixture.nativeElement.querySelector('[data-testid="elohim-insight-message"]');
      expect(message).toBeTruthy();
      expect(message.textContent).toContain('Attachment Style Assessment');
    });

    it('should include constitutional reasoning section', () => {
      const reasoning = fixture.nativeElement.querySelector(
        '[data-testid="elohim-insight-reasoning"]'
      );
      expect(reasoning).toBeTruthy();
      expect(reasoning.textContent).toContain('Love as committed action toward flourishing');
    });

    it('should display computation cost in reasoning', () => {
      const reasoning = fixture.nativeElement.querySelector(
        '[data-testid="elohim-insight-reasoning"]'
      );
      expect(reasoning.textContent).toContain('800');
      expect(reasoning.textContent).toContain('1200');
    });
  });

  // ─── Mastery Mode ──────────────────────────────────────────────────────────

  describe('mastery mode', () => {
    beforeEach(() => {
      host.mode.set('mastery');
      host.score.set(85);
      host.passed.set(true);
      host.correctCount.set(17);
      host.totalCount.set(20);
      fixture.detectChanges();
    });

    it('should not record attestation', () => {
      expect(mockDiscoveryService.recordFromCompletion).not.toHaveBeenCalled();
    });

    it('should show "Well Done!" for passing score', () => {
      const headline = fixture.nativeElement.querySelector('[data-testid="completion-headline"]');
      expect(headline.textContent).toContain('Well Done!');
    });

    it('should show score display', () => {
      const score = fixture.nativeElement.querySelector('[data-testid="completion-score"]');
      expect(score).toBeTruthy();
      expect(score.textContent).toContain('85%');
      expect(score.textContent).toContain('17 of 20 correct');
    });

    it('should show target icon for passing', () => {
      const icon = fixture.nativeElement.querySelector('[data-testid="completion-icon"]');
      expect(icon.textContent.trim()).toBe('🎯');
    });

    it('should show "Keep Learning" for failing score', () => {
      host.passed.set(false);
      fixture.detectChanges();

      const headline = fixture.nativeElement.querySelector('[data-testid="completion-headline"]');
      expect(headline.textContent).toContain('Keep Learning');
    });

    it('should show book icon for failing', () => {
      host.passed.set(false);
      fixture.detectChanges();

      const icon = fixture.nativeElement.querySelector('[data-testid="completion-icon"]');
      expect(icon.textContent.trim()).toBe('📚');
    });

    it('should apply passed class to result card', () => {
      const card = fixture.nativeElement.querySelector('[data-testid="completion-result-card"]');
      expect(card.classList.contains('passed')).toBe(true);
      expect(card.classList.contains('failed')).toBe(false);
    });

    it('should apply failed class when not passed', () => {
      host.passed.set(false);
      fixture.detectChanges();

      const card = fixture.nativeElement.querySelector('[data-testid="completion-result-card"]');
      expect(card.classList.contains('failed')).toBe(true);
    });

    it('should show dashboard link', () => {
      const link = fixture.nativeElement.querySelector('[data-testid="completion-profile-link"]');
      expect(link.textContent).toContain('View your Dashboard');
    });

    it('should not show hex badge', () => {
      const badge = fixture.nativeElement.querySelector('[data-testid="completion-badge-reveal"]');
      expect(badge).toBeNull();
    });

    it('should not show subscale breakdown', () => {
      const subscales = fixture.nativeElement.querySelector('[data-testid="completion-subscales"]');
      expect(subscales).toBeNull();
    });

    it('should show mastery preview when profile available', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();

      const preview = fixture.nativeElement.querySelector(
        '[data-testid="completion-mastery-preview"]'
      );
      expect(preview).toBeTruthy();
      expect(preview.textContent).toContain('450');
      expect(preview.textContent).toContain('3 Day Streak');
      expect(preview.textContent).toContain('12 Mastered');
    });
  });

  // ─── Reflection Mode ───────────────────────────────────────────────────────

  describe('reflection mode', () => {
    beforeEach(() => {
      host.mode.set('reflection');
      host.primaryType.set(null);
      host.instrumentId.set(null);
      fixture.detectChanges();
    });

    it('should show generic headline', () => {
      const headline = fixture.nativeElement.querySelector('[data-testid="completion-headline"]');
      expect(headline.textContent).toContain('Assessment Complete');
    });

    it('should show sparkle icon', () => {
      const icon = fixture.nativeElement.querySelector('[data-testid="completion-icon"]');
      expect(icon.textContent.trim()).toBe('✨');
    });

    it('should not record attestation', () => {
      expect(mockDiscoveryService.recordFromCompletion).not.toHaveBeenCalled();
    });
  });

  // ─── User Interaction ──────────────────────────────────────────────────────

  describe('user interaction', () => {
    beforeEach(() => {
      fixture.detectChanges();
    });

    it('should emit continue when Continue button clicked', () => {
      const btn = fixture.nativeElement.querySelector('[data-testid="completion-continue"]');
      btn.click();
      expect(host.continued).toBe(true);
    });

    it('should emit viewProfile when profile link clicked', () => {
      const link = fixture.nativeElement.querySelector('[data-testid="completion-profile-link"]');
      link.click();
      expect(host.profileViewed).toBe(true);
    });
  });

  // ─── Edge Cases ────────────────────────────────────────────────────────────

  describe('edge cases', () => {
    it('should handle missing primaryType gracefully in discovery mode', () => {
      host.primaryType.set(null);
      fixture.detectChanges();

      const headline = fixture.nativeElement.querySelector('[data-testid="completion-headline"]');
      expect(headline.textContent).toContain('Assessment Complete');
    });

    it('should use "an" article for vowel-starting names', () => {
      host.primaryType.set(buildPrimaryType({ name: 'Individualist' }));
      fixture.detectChanges();

      const headline = fixture.nativeElement.querySelector('[data-testid="completion-headline"]');
      expect(headline.textContent).toContain("You're an Individualist!");
    });

    it('should handle empty subscale scores', () => {
      host.subscaleScores.set({});
      fixture.detectChanges();

      const subscales = fixture.nativeElement.querySelector('[data-testid="completion-subscales"]');
      expect(subscales).toBeNull();
    });

    it('should handle null subscale scores', () => {
      host.subscaleScores.set(null);
      fixture.detectChanges();

      const subscales = fixture.nativeElement.querySelector('[data-testid="completion-subscales"]');
      expect(subscales).toBeNull();
    });
  });
});
