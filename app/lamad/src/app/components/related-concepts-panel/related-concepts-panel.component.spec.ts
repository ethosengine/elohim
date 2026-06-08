import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { RelatedConceptsPanelComponent } from './related-concepts-panel.component';
import { RelatedConceptsService } from '../../services/related-concepts.service';
import { of } from 'rxjs';
import { vi } from 'vitest';

describe('RelatedConceptsPanelComponent', () => {
  let component: RelatedConceptsPanelComponent;
  let fixture: ComponentFixture<RelatedConceptsPanelComponent>;
  let mockRelatedConceptsService: any;

  beforeEach(async () => {
    mockRelatedConceptsService = {
      getRelatedConcepts: vi.fn(),
    };
    mockRelatedConceptsService.getRelatedConcepts.mockReturnValue(
      of({
        prerequisites: [],
        extensions: [],
        related: [],
        children: [],
        parents: [],
        discovered: [],
        allRelationships: [],
      })
    );

    await TestBed.configureTestingModule({
      imports: [RelatedConceptsPanelComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: RelatedConceptsService, useValue: mockRelatedConceptsService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(RelatedConceptsPanelComponent);
    component = fixture.componentInstance;
    component.contentId = 'test-content-id';
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('renders discovered concepts as a distinct, non-explicit section', () => {
    mockRelatedConceptsService.getRelatedConcepts.mockReturnValue(
      of({
        prerequisites: [],
        extensions: [],
        related: [],
        children: [],
        parents: [],
        discovered: [
          {
            id: 'discovered-node-1',
            title: 'Tag-Discovered Concept',
            contentType: 'concept',
            content: '',
          },
        ],
        allRelationships: [],
      })
    );

    // Re-trigger the load with the discovered-bearing result.
    component.contentId = 'content-with-discovered';
    component.ngOnChanges({
      contentId: {
        currentValue: 'content-with-discovered',
        previousValue: 'test-content-id',
        firstChange: false,
        isFirstChange: () => false,
      },
    });
    fixture.detectChanges();

    const card: HTMLElement | null = fixture.nativeElement.querySelector(
      '[data-testid="discovered-concept-card"]'
    );
    expect(card).not.toBeNull();
    const source = card?.getAttribute('inference-source');
    expect(source).toBeTruthy();
    expect(source).not.toBe('explicit');
  });

  it('is not empty when only discovered edges exist', () => {
    mockRelatedConceptsService.getRelatedConcepts.mockReturnValue(
      of({
        prerequisites: [],
        extensions: [],
        related: [],
        children: [],
        parents: [],
        discovered: [
          {
            id: 'discovered-node-1',
            title: 'Tag-Discovered Concept',
            contentType: 'concept',
            content: '',
          },
        ],
        allRelationships: [],
      })
    );

    component.contentId = 'content-with-discovered';
    component.ngOnChanges({
      contentId: {
        currentValue: 'content-with-discovered',
        previousValue: 'test-content-id',
        firstChange: false,
        isFirstChange: () => false,
      },
    });
    fixture.detectChanges();

    expect(component.isEmpty).toBe(false);
  });
});
