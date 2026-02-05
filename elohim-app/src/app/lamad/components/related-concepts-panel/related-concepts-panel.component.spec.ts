import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { RelatedConceptsPanelComponent } from './related-concepts-panel.component';
import { RelatedConceptsService } from '../../services/related-concepts.service';
import { of } from 'rxjs';

describe('RelatedConceptsPanelComponent', () => {
  let component: RelatedConceptsPanelComponent;
  let fixture: ComponentFixture<RelatedConceptsPanelComponent>;
  let mockRelatedConceptsService: jasmine.SpyObj<RelatedConceptsService>;

  beforeEach(async () => {
    mockRelatedConceptsService = jasmine.createSpyObj('RelatedConceptsService', [
      'getRelatedConcepts',
    ]);
    mockRelatedConceptsService.getRelatedConcepts.and.returnValue(
      of({
        prerequisites: [],
        extensions: [],
        related: [],
        children: [],
        parents: [],
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
});
