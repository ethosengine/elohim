import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { By } from '@angular/platform-browser';

import { BehaviorSubject } from 'rxjs';

import { MasteryStatsService } from '../../services/mastery-stats.service';

import { LearnerDashboardComponent } from './learner-dashboard.component';

import type { MasteryLevel } from '../../models/content-mastery.model';
import type { LearnerMasteryProfile } from '../../models/learner-mastery-profile.model';

describe('LearnerDashboardComponent', () => {
  let component: LearnerDashboardComponent;
  let fixture: ComponentFixture<LearnerDashboardComponent>;
  let mockMasteryStats: jasmine.SpyObj<MasteryStatsService>;
  let profileSubject: BehaviorSubject<LearnerMasteryProfile | null>;

  const buildProfile = (overrides: Partial<LearnerMasteryProfile> = {}): LearnerMasteryProfile => ({
    learnerLevel: { level: 3, label: 'Student', icon: 'school', xpThreshold: 500, color: '#ffc107' },
    levelProgress: 40,
    totalXP: 700,
    earnedPoints: 600,
    masteryXP: 100,
    levelDistribution: {
      not_started: 0,
      seen: 5,
      remember: 3,
      understand: 2,
      apply: 4,
      analyze: 1,
      evaluate: 0,
      create: 0,
    } as Record<MasteryLevel, number>,
    totalMasteredNodes: 15,
    nodesAboveGate: 5,
    streak: {
      currentStreak: 3,
      bestStreak: 7,
      todayActive: true,
      lastActiveDate: '2026-02-15',
      streakStartDate: '2026-02-13',
      recentActivity: { '2026-02-15': true, '2026-02-14': true, '2026-02-13': true },
    },
    recentLevelUps: [
      {
        id: 'lu-1',
        contentId: 'content-1',
        fromLevel: 'seen' as MasteryLevel,
        toLevel: 'remember' as MasteryLevel,
        timestamp: '2026-02-15T10:00:00.000Z',
        pointsEarned: 20,
        isGateLevel: false,
      },
    ],
    practice: {
      totalChallenges: 25,
      totalLevelUps: 10,
      totalLevelDowns: 3,
      totalDiscoveries: 5,
      activePoolSize: 8,
      refreshQueueSize: 2,
    },
    paths: {
      inProgress: [
        {
          pathId: 'path-1',
          title: 'Test Path',
          progressPercent: 60,
          completedSteps: 6,
          totalSteps: 10,
          lastActiveAt: '2026-02-15T10:00:00.000Z',
        },
      ],
      completed: [],
    },
    computedAt: '2026-02-15T12:00:00.000Z',
    ...overrides,
  });

  beforeEach(async () => {
    profileSubject = new BehaviorSubject<LearnerMasteryProfile | null>(null);

    mockMasteryStats = jasmine.createSpyObj('MasteryStatsService', ['recordDailyEngagement'], {
      learnerProfile$: profileSubject.asObservable(),
    });

    await TestBed.configureTestingModule({
      imports: [LearnerDashboardComponent, RouterTestingModule],
      providers: [{ provide: MasteryStatsService, useValue: mockMasteryStats }],
    }).compileComponents();

    fixture = TestBed.createComponent(LearnerDashboardComponent);
    component = fixture.componentInstance;
  });

  describe('Loading state', () => {
    it('should create', () => {
      expect(component).toBeTruthy();
    });

    it('should start in loading state before profile emits', () => {
      // isLoading is true before ngOnInit subscribes to the BehaviorSubject
      expect(component.isLoading).toBe(true);
      expect(component.profile).toBeNull();
    });

    it('should hide loading spinner after profile loads', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      const spinner = fixture.debugElement.query(By.css('.loading-spinner'));
      expect(spinner).toBeFalsy();
    });
  });

  describe('Empty state', () => {
    it('should show empty state when profile is null after loading', () => {
      // Trigger subscription (sets isLoading = false)
      fixture.detectChanges();
      // Profile is null, so after subscribe fires with null, isLoading becomes false
      profileSubject.next(null);
      fixture.detectChanges();
      const emptyState = fixture.debugElement.query(By.css('.dashboard-empty'));
      expect(emptyState).toBeTruthy();
    });

    it('should show explore paths link in empty state', () => {
      fixture.detectChanges();
      profileSubject.next(null);
      fixture.detectChanges();
      const link = fixture.debugElement.query(By.css('.dashboard-empty a[routerLink="/lamad"]'));
      expect(link).toBeTruthy();
    });
  });

  describe('Dashboard header', () => {
    beforeEach(() => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
    });

    it('should display learner level label', () => {
      const header = fixture.debugElement.query(By.css('.level-info h1'));
      expect(header.nativeElement.textContent).toBe('Student');
    });

    it('should display level number', () => {
      const levelNumber = fixture.debugElement.query(By.css('.level-number'));
      expect(levelNumber.nativeElement.textContent).toBe('3');
    });

    it('should display XP total', () => {
      const xpText = fixture.debugElement.query(By.css('.xp-text'));
      expect(xpText.nativeElement.textContent).toContain('700');
      expect(xpText.nativeElement.textContent).toContain('XP');
    });

    it('should set XP bar width from levelProgress', () => {
      const xpBar = fixture.debugElement.query(By.css('.xp-bar'));
      expect(xpBar.styles['width']).toBe('40%');
    });

    it('should set level badge background color', () => {
      const badge = fixture.debugElement.query(By.css('.level-badge'));
      expect(badge.styles['background-color']).toBe('rgb(255, 193, 7)');
    });
  });

  describe('Streak card', () => {
    beforeEach(() => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
    });

    it('should display current streak count', () => {
      const bigNumber = fixture.debugElement.query(By.css('.big-number'));
      expect(bigNumber.nativeElement.textContent).toBe('3');
    });

    it('should display best streak', () => {
      const best = fixture.debugElement.query(By.css('.streak-best'));
      expect(best.nativeElement.textContent).toContain('7');
    });

    it('should render 30 activity dots', () => {
      const dots = fixture.debugElement.queryAll(By.css('.activity-dot'));
      expect(dots.length).toBe(30);
    });

    it('should mark active days on dots', () => {
      const activeDots = fixture.debugElement.queryAll(By.css('.activity-dot.active'));
      expect(activeDots.length).toBeGreaterThan(0);
    });

    it('should show active streak icon when todayActive is true', () => {
      const icon = fixture.debugElement.query(By.css('.streak-icon.streak-active'));
      expect(icon).toBeTruthy();
    });
  });

  describe('Mastery distribution', () => {
    beforeEach(() => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
    });

    it('should display mastery bars for each Bloom level', () => {
      const rows = fixture.debugElement.queryAll(By.css('.mastery-bar-row'));
      expect(rows.length).toBe(7); // seen through create (skip not_started)
    });

    it('should mark apply level as gate level', () => {
      const gateRow = fixture.debugElement.query(By.css('.mastery-bar-row.gate-level'));
      expect(gateRow).toBeTruthy();
    });

    it('should display node count summary', () => {
      const summary = fixture.debugElement.query(By.css('.mastery-summary'));
      expect(summary.nativeElement.textContent).toContain('15');
      expect(summary.nativeElement.textContent).toContain('5 above gate');
    });
  });

  describe('Active paths', () => {
    it('should display path entries when paths exist', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      const pathEntry = fixture.debugElement.query(By.css('.path-entry'));
      expect(pathEntry).toBeTruthy();
    });

    it('should show path title', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      const title = fixture.debugElement.query(By.css('.path-title'));
      expect(title.nativeElement.textContent).toBe('Test Path');
    });

    it('should show step count', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      const steps = fixture.debugElement.query(By.css('.path-steps'));
      expect(steps.nativeElement.textContent).toContain('6/10');
    });

    it('should show explore paths link when no active paths', () => {
      profileSubject.next(buildProfile({ paths: { inProgress: [], completed: [] } }));
      fixture.detectChanges();
      const emptyState = fixture.debugElement.query(By.css('.paths-card .empty-state'));
      expect(emptyState).toBeTruthy();
    });
  });

  describe('Recent level-ups', () => {
    it('should display level-up entries', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      const entry = fixture.debugElement.query(By.css('.levelup-entry'));
      expect(entry).toBeTruthy();
    });

    it('should show from and to level badges', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      const from = fixture.debugElement.query(By.css('.level-from'));
      const to = fixture.debugElement.query(By.css('.level-to'));
      expect(from).toBeTruthy();
      expect(to).toBeTruthy();
    });

    it('should show points earned', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      const points = fixture.debugElement.query(By.css('.levelup-points'));
      expect(points.nativeElement.textContent).toContain('+20');
    });

    it('should hide section when no level-ups', () => {
      profileSubject.next(buildProfile({ recentLevelUps: [] }));
      fixture.detectChanges();
      const section = fixture.debugElement.query(By.css('.levelups-card'));
      expect(section).toBeFalsy();
    });

    it('should highlight gate-crossing events', () => {
      profileSubject.next(
        buildProfile({
          recentLevelUps: [
            {
              id: 'lu-gate',
              contentId: 'c-1',
              fromLevel: 'understand' as MasteryLevel,
              toLevel: 'apply' as MasteryLevel,
              timestamp: '2026-02-15T10:00:00.000Z',
              pointsEarned: 20,
              isGateLevel: true,
            },
          ],
        })
      );
      fixture.detectChanges();
      const gateBadge = fixture.debugElement.query(By.css('.gate-badge'));
      expect(gateBadge).toBeTruthy();
      const gateCrossing = fixture.debugElement.query(By.css('.levelup-entry.gate-crossing'));
      expect(gateCrossing).toBeTruthy();
    });
  });

  describe('Practice stats', () => {
    beforeEach(() => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
    });

    it('should display practice stat numbers', () => {
      const stats = fixture.debugElement.queryAll(By.css('.stat-number'));
      expect(stats.length).toBe(3);
      expect(stats[0].nativeElement.textContent).toBe('25');
      expect(stats[1].nativeElement.textContent).toBe('10');
      expect(stats[2].nativeElement.textContent).toBe('5');
    });
  });

  describe('getBarWidth()', () => {
    it('should return 0 when profile is null', () => {
      expect(component.getBarWidth('seen')).toBe(0);
    });

    it('should compute relative bar widths', () => {
      profileSubject.next(buildProfile());
      fixture.detectChanges();
      // seen=5 is max, so it should be 100%
      expect(component.getBarWidth('seen')).toBe(100);
      // apply=4, so 4/5 * 100 = 80%
      expect(component.getBarWidth('apply')).toBe(80);
      // evaluate=0, so 0%
      expect(component.getBarWidth('evaluate')).toBe(0);
    });
  });

  describe('Lifecycle', () => {
    it('should record daily engagement on init', () => {
      fixture.detectChanges();
      expect(mockMasteryStats.recordDailyEngagement).toHaveBeenCalledWith('dashboard_visit');
    });

    it('should unsubscribe on destroy', () => {
      fixture.detectChanges();
      component.ngOnDestroy();
      // Verify no errors on subsequent emissions
      expect(() => profileSubject.next(null)).not.toThrow();
    });
  });
});
