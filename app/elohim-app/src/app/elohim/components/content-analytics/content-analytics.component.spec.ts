import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { ContentAnalyticsComponent } from './content-analytics.component';
import { EventService } from '@app/shefa/services/event.service';

/**
 * ContentAnalyticsComponent unit tests.
 *
 * Tests the protocol-level analytics logic (view count, completion count,
 * completion rate calculation) by creating the component within an injection
 * context that provides a mock EventService.
 */
describe('ContentAnalyticsComponent', () => {
  let component: ContentAnalyticsComponent;
  const getViewCount = vi.fn();
  const getCompletionCount = vi.fn();

  beforeEach(() => {
    getViewCount.mockReset();
    getCompletionCount.mockReset();
    getViewCount.mockReturnValue(of(42));
    getCompletionCount.mockReturnValue(of(8));

    TestBed.configureTestingModule({
      providers: [
        { provide: EventService, useValue: { getViewCount, getCompletionCount } },
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

  it('calculates completion rate', () => {
    expect(component.completionRate).toBe(19);
  });

  it('handles zero views without division error', () => {
    getViewCount.mockReturnValue(of(0));
    getCompletionCount.mockReturnValue(of(0));

    component.contentId = 'empty-node';
    component.ngOnChanges();

    expect(component.completionRate).toBe(0);
    expect(component.viewCount).toBe(0);
    expect(component.completionCount).toBe(0);
  });

  it('recalculates on contentId change', () => {
    expect(getViewCount).toHaveBeenCalledWith('concept-trust');

    component.contentId = 'concept-governance';
    component.ngOnChanges();
    expect(getViewCount).toHaveBeenCalledWith('concept-governance');
  });
});
