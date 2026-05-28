import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { ContentAnalyticsComponent } from './content-analytics.component';
import { StorageClientService } from '@app/elohim/services/storage-client.service';

/**
 * ContentAnalyticsComponent unit tests.
 *
 * M-AGGR-3: component now reads from StorageClientService.getContentEngagement()
 * (GET /api/v1/lamad/content/{contentId}/engagement) instead of EventService
 * count methods. The projection returns a ContentEngagementStatsView with
 * pre-computed completionRate (0.0–1.0 as a fraction).
 */
describe('ContentAnalyticsComponent', () => {
  let component: ContentAnalyticsComponent;
  const getContentEngagement = vi.fn();

  beforeEach(() => {
    getContentEngagement.mockReset();
    getContentEngagement.mockReturnValue(
      of({
        contentId: 'concept-trust',
        views: BigInt(42),
        completions: BigInt(8),
        uniqueViewers: BigInt(35),
        completionRate: 0.1904,
        computedAt: '2026-05-28T12:00:00Z',
      })
    );

    TestBed.configureTestingModule({
      providers: [
        { provide: StorageClientService, useValue: { getContentEngagement } },
      ],
    });

    component = TestBed.runInInjectionContext(() => new ContentAnalyticsComponent());
    component.contentId = 'concept-trust';
    component.ngOnChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('loads view count', () => {
    expect(component.viewCount).toBe(42);
  });

  it('loads completion count', () => {
    expect(component.completionCount).toBe(8);
  });

  it('derives completion rate from projection (rounds to integer percent)', () => {
    // completionRate from projection is 0.0–1.0; component multiplies by 100 and rounds
    expect(component.completionRate).toBe(19);
  });

  it('handles zero views without division error', () => {
    getContentEngagement.mockReturnValue(
      of({
        contentId: 'empty-node',
        views: BigInt(0),
        completions: BigInt(0),
        uniqueViewers: BigInt(0),
        completionRate: 0.0,
        computedAt: '2026-05-28T12:00:00Z',
      })
    );

    component.contentId = 'empty-node';
    component.ngOnChanges();

    expect(component.completionRate).toBe(0);
    expect(component.viewCount).toBe(0);
    expect(component.completionCount).toBe(0);
  });

  it('recalculates on contentId change', () => {
    expect(getContentEngagement).toHaveBeenCalledWith('concept-trust');

    component.contentId = 'concept-governance';
    component.ngOnChanges();
    expect(getContentEngagement).toHaveBeenCalledWith('concept-governance');
  });
});
