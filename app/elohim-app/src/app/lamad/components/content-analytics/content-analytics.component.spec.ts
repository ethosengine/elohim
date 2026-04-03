import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ContentAnalyticsComponent } from './content-analytics.component';
import { EventService } from '@app/shefa/services/event.service';

describe('ContentAnalyticsComponent', () => {
  let component: ContentAnalyticsComponent;
  let fixture: ComponentFixture<ContentAnalyticsComponent>;
  let eventServiceSpy: jasmine.SpyObj<EventService>;

  beforeEach(async () => {
    eventServiceSpy = jasmine.createSpyObj('EventService', [
      'getViewCount',
      'getCompletionCount',
    ]);
    eventServiceSpy.getViewCount.and.returnValue(of(42));
    eventServiceSpy.getCompletionCount.and.returnValue(of(8));

    await TestBed.configureTestingModule({
      imports: [ContentAnalyticsComponent],
      providers: [{ provide: EventService, useValue: eventServiceSpy }],
    }).compileComponents();

    fixture = TestBed.createComponent(ContentAnalyticsComponent);
    component = fixture.componentInstance;
    component.contentId = 'concept-trust';
    fixture.detectChanges();
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
    eventServiceSpy.getViewCount.and.returnValue(of(0));
    eventServiceSpy.getCompletionCount.and.returnValue(of(0));

    component.contentId = 'empty-node';
    component.ngOnChanges();

    expect(component.completionRate).toBe(0);
  });
});
