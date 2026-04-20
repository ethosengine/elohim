/**
 * SignalsCardComponent — unit tests
 *
 * Covers:
 * - Empty state renders "All distribution is within contract bounds."
 * - Non-empty: gap counts render per kind
 * - Filter-by-kind sectioning: only sections with gaps are rendered
 * - Loading state hides gap content
 * - Error state shows error message
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NO_ERRORS_SCHEMA } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { of, throwError } from 'rxjs';
import { vi } from 'vitest';

import { SignalsCardComponent } from './signals-card.component';
import { ResilienceService } from '@elohim/service/public-api';
import type { PlacementGapView } from '@app/generated/placement-gap-view';

const makeGap = (id: string, gapKind: PlacementGapView['gapKind']): PlacementGapView => ({
  id,
  contentId: `content-${id}`,
  shardHash: `hash-${id}`,
  requestedStewardCount: 3,
  achievedStewardCount: 1,
  contractCoverage: 0.33,
  gapKind,
  firstSeenAt: '2026-04-20T00:00:00.000Z',
  lastSeenAt: '2026-04-20T01:00:00.000Z',
});

describe('SignalsCardComponent', () => {
  let component: SignalsCardComponent;
  let fixture: ComponentFixture<SignalsCardComponent>;
  let resilienceMock: { listPlacementGaps: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    resilienceMock = {
      listPlacementGaps: vi.fn().mockReturnValue(of({ items: [], total: 0 })),
    };

    await TestBed.configureTestingModule({
      imports: [SignalsCardComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ResilienceService, useValue: resilienceMock },
      ],
      schemas: [NO_ERRORS_SCHEMA],
    }).compileComponents();

    fixture = TestBed.createComponent(SignalsCardComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('empty state', () => {
    it('renders "All distribution is within contract bounds." when no gaps', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(of({ items: [], total: 0 }));
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="shefa-signals-card"]')).toBeTruthy();
      expect(el.textContent).toContain('All distribution is within contract bounds.');
    });

    it('shows "0 placement gaps" in the subline', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(of({ items: [], total: 0 }));
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('0 placement gaps');
    });
  });

  describe('non-empty rendering', () => {
    it('shows correct total gap count in subline', () => {
      const gaps = [
        makeGap('g1', 'peers-unavailable'),
        makeGap('g2', 'contracts-short'),
        makeGap('g3', 'under-committed'),
      ];
      resilienceMock.listPlacementGaps.mockReturnValue(of({ items: gaps, total: 3 }));
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('3 placement gaps');
    });

    it('uses singular "gap" when total is 1', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(
        of({ items: [makeGap('g1', 'peers-unavailable')], total: 1 })
      );
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('1 placement gap');
      expect(el.textContent).not.toContain('1 placement gaps');
    });

    it('does not show empty state message when gaps exist', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(
        of({ items: [makeGap('g1', 'peers-unavailable')], total: 1 })
      );
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).not.toContain('All distribution is within contract bounds.');
    });
  });

  describe('filter-by-kind sectioning', () => {
    it('renders peers-unavailable section only when that kind is present', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(
        of({ items: [makeGap('g1', 'peers-unavailable')], total: 1 })
      );
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      const puSection = el.querySelector('.signal-peers-unavailable');
      expect(puSection).toBeTruthy();
      expect(puSection?.textContent).toContain('recruit');

      expect(el.querySelector('.signal-contracts-short')).toBeNull();
      expect(el.querySelector('.signal-under-committed')).toBeNull();
    });

    it('renders contracts-short section only when that kind is present', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(
        of({ items: [makeGap('g1', 'contracts-short')], total: 1 })
      );
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      const csSection = el.querySelector('.signal-contracts-short');
      expect(csSection).toBeTruthy();
      expect(csSection?.textContent).toContain('propose contracts');

      expect(el.querySelector('.signal-peers-unavailable')).toBeNull();
      expect(el.querySelector('.signal-under-committed')).toBeNull();
    });

    it('renders under-committed section only when that kind is present', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(
        of({ items: [makeGap('g1', 'under-committed')], total: 1 })
      );
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      const ucSection = el.querySelector('.signal-under-committed');
      expect(ucSection).toBeTruthy();
      expect(ucSection?.textContent).toContain('subsidize');

      expect(el.querySelector('.signal-peers-unavailable')).toBeNull();
      expect(el.querySelector('.signal-contracts-short')).toBeNull();
    });

    it('renders all three sections when all kinds are present', () => {
      const gaps = [
        makeGap('g1', 'peers-unavailable'),
        makeGap('g2', 'peers-unavailable'),
        makeGap('g3', 'contracts-short'),
        makeGap('g4', 'under-committed'),
      ];
      resilienceMock.listPlacementGaps.mockReturnValue(of({ items: gaps, total: 4 }));
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('.signal-peers-unavailable')).toBeTruthy();
      expect(el.querySelector('.signal-contracts-short')).toBeTruthy();
      expect(el.querySelector('.signal-under-committed')).toBeTruthy();
    });

    it('shows correct count per kind within each section', () => {
      const gaps = [
        makeGap('g1', 'peers-unavailable'),
        makeGap('g2', 'peers-unavailable'),
        makeGap('g3', 'contracts-short'),
      ];
      resilienceMock.listPlacementGaps.mockReturnValue(of({ items: gaps, total: 3 }));
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      const puSection = el.querySelector('.signal-peers-unavailable');
      expect(puSection?.querySelector('strong')?.textContent?.trim()).toBe('2');

      const csSection = el.querySelector('.signal-contracts-short');
      expect(csSection?.querySelector('strong')?.textContent?.trim()).toBe('1');
    });
  });

  describe('loading state', () => {
    it('hides gap content while loading', () => {
      // Return an observable that never emits
      resilienceMock.listPlacementGaps.mockReturnValue(new (require('rxjs').Subject)());
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('Loading');
      expect(el.querySelector('.signal-peers-unavailable')).toBeNull();
      expect(el.querySelector('.empty')).toBeNull();
    });
  });

  describe('error state', () => {
    it('shows error message when service throws', () => {
      resilienceMock.listPlacementGaps.mockReturnValue(
        throwError(() => new Error('Network timeout'))
      );
      fixture.detectChanges();

      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('Signals unavailable');
      expect(el.textContent).toContain('Network timeout');
    });
  });

  describe('byKind helper', () => {
    it('filters gaps by kind correctly', () => {
      component.gaps = [
        makeGap('g1', 'peers-unavailable'),
        makeGap('g2', 'contracts-short'),
        makeGap('g3', 'peers-unavailable'),
      ];

      expect(component.byKind('peers-unavailable')).toHaveLength(2);
      expect(component.byKind('contracts-short')).toHaveLength(1);
      expect(component.byKind('under-committed')).toHaveLength(0);
    });

    it('totalGaps returns gaps array length', () => {
      component.gaps = [makeGap('g1', 'peers-unavailable'), makeGap('g2', 'contracts-short')];
      expect(component.totalGaps).toBe(2);
    });
  });
});
