import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ResilienceSnapshotComponent } from './resilience-snapshot.component';
import { ResilienceSnapshotView } from '../../generated/resilience-snapshot-view';

const sampleProtected: ResilienceSnapshotView = {
  contentId: 'c1',
  stewardingCollectives: 4,
  commitmentBackedCollectives: 4,
  diversityScore: 0.95,
  regionalDistribution: { local: 1, regional: 2, global: 1, unknown: 0 },
  placementGaps: [],
  protectionStatus: 'protected',
} as ResilienceSnapshotView;

const samplePartial: ResilienceSnapshotView = {
  ...sampleProtected,
  stewardingCollectives: 2,
  diversityScore: 0.5,
  protectionStatus: 'partial',
};

describe('ResilienceSnapshotComponent', () => {
  let fixture: ComponentFixture<ResilienceSnapshotComponent>;
  let component: ResilienceSnapshotComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ResilienceSnapshotComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(ResilienceSnapshotComponent);
    component = fixture.componentInstance;
  });

  it('renders icon density with green indicator when protected', () => {
    component.snapshot = sampleProtected;
    component.density = 'icon';
    fixture.detectChanges();

    const el: HTMLElement = fixture.nativeElement;
    const icon = el.querySelector('[data-testid="resilience-icon"]');
    expect(icon?.classList.contains('status-protected')).toBe(true);
    const tooltip = el.querySelector('[data-testid="resilience-tooltip"]')?.textContent ?? '';
    expect(tooltip).toContain('4 collectives');
    expect(tooltip).toContain('protected');
  });

  it('renders yellow indicator when partial', () => {
    component.snapshot = samplePartial;
    component.density = 'icon';
    fixture.detectChanges();
    const icon = fixture.nativeElement.querySelector('[data-testid="resilience-icon"]');
    expect(icon?.classList.contains('status-partial')).toBe(true);
  });

  it('context-menu density lists placement gap count', () => {
    const withGaps: ResilienceSnapshotView = {
      ...samplePartial,
      placementGaps: [{
        id: 'g1', contentId: 'c1', shardHash: 'h1',
        requestedStewardCount: 3, achievedStewardCount: 1,
        contractCoverage: 0.33, gapKind: 'peers-unavailable',
        firstSeenAt: '2026-04-19T00:00:00Z', lastSeenAt: '2026-04-19T00:00:00Z',
      }],
    };
    component.snapshot = withGaps;
    component.density = 'context';
    fixture.detectChanges();
    const gapCount = fixture.nativeElement.querySelector('[data-testid="resilience-gap-count"]');
    expect(gapCount?.textContent).toContain('1');
  });

  it('full card lists steward collectives', () => {
    const withCollectives: ResilienceSnapshotView = {
      ...sampleProtected,
      details: {
        stewardingCollectives: [
          { id: 'home-alpha', kind: 'household', label: 'Alpha Household' },
          { id: 'home-beta', kind: 'household', label: 'Beta Household' },
          { id: 'home-gamma', kind: 'household', label: 'Gamma Household' },
          { id: 'home-delta', kind: 'household', label: 'Delta Household' },
        ],
        onlinePeerCount: 4,
        healthScore: 0.95,
      },
    };
    component.snapshot = withCollectives;
    component.density = 'full';
    fixture.detectChanges();
    const rows = fixture.nativeElement.querySelectorAll('[data-testid^="resilience-collective-"]');
    expect(rows.length).toBe(4);
  });
});
