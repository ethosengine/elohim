import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { RecommendationListComponent } from './recommendation-list.component';
import { EprResolverService } from '@app/elohim/services/epr-resolver.service';
import type { ContentRecommendation } from '../../services/path-adaptation.service';

describe('RecommendationListComponent', () => {
  let component: RecommendationListComponent;
  let fixture: ComponentFixture<RecommendationListComponent>;

  const mockRecs: ContentRecommendation[] = [
    {
      contentId: 'prereq-foundations',
      reason: 'prerequisite_gap',
      confidence: 0.8,
      triggerContext: { quizType: 'mastery', conceptIds: ['concept-trust'], score: 0.3 },
    },
    {
      contentId: 'alt-perspective',
      reason: 'reinforcement',
      confidence: 0.7,
      triggerContext: { quizType: 'mastery', conceptIds: ['concept-trust'], score: 0.4 },
    },
  ];

  beforeEach(async () => {
    const mockResolver = {
      resolve: vi.fn().mockReturnValue(of(null)),
      resolveEprHead: vi.fn().mockReturnValue(of(null)),
    };

    await TestBed.configureTestingModule({
      imports: [RecommendationListComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([]),
        { provide: EprResolverService, useValue: mockResolver },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(RecommendationListComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render nothing when recommendations is empty', () => {
    component.recommendations = [];
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="recommendation-list"]')).toBeNull();
  });

  it('should render recommendation items when provided', () => {
    component.recommendations = mockRecs;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const list = el.querySelector('[data-testid="recommendation-list"]');
    expect(list).toBeTruthy();

    const items = el.querySelectorAll('[data-testid^="recommendation-item-"]');
    expect(items.length).toBe(2);
  });

  it('should show prerequisite context label', () => {
    component.recommendations = [mockRecs[0]];
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const label = el.querySelector('[data-testid="recommendation-context-0"]');
    expect(label?.textContent).toContain('Foundation');
  });

  it('should show reinforcement context label', () => {
    component.recommendations = [mockRecs[1]];
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const label = el.querySelector('[data-testid="recommendation-context-0"]');
    expect(label?.textContent).toContain('Another angle');
  });

  it('should emit dismiss event with contentId', () => {
    component.recommendations = mockRecs;
    fixture.detectChanges();

    const dismissSpy = vi.fn();
    component.dismiss.subscribe(dismissSpy);

    const btn = fixture.nativeElement.querySelector(
      '[data-testid="recommendation-dismiss-0"]'
    ) as HTMLButtonElement;
    btn?.click();

    expect(dismissSpy).toHaveBeenCalledWith('prereq-foundations');
  });

  it('should render epr-link for each recommendation', () => {
    component.recommendations = mockRecs;
    fixture.detectChanges();

    const links = fixture.nativeElement.querySelectorAll('app-epr-link');
    expect(links.length).toBe(2);
  });

  it('should use custom heading when provided', () => {
    component.recommendations = mockRecs;
    component.heading = 'Recommended Content';
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const heading = el.querySelector('[data-testid="recommendation-heading"]');
    expect(heading?.textContent).toContain('Recommended Content');
  });
});
