import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ConceptCardComponent } from './concept-card.component';
import { ContentNode } from '../../models/content-node.model';

describe('ConceptCardComponent', () => {
  let component: ConceptCardComponent;
  let fixture: ComponentFixture<ConceptCardComponent>;

  const mockConcept: ContentNode = {
    id: 'test-concept-1',
    title: 'Test Concept',
    description: 'A test concept for the concept card component',
    contentType: 'concept',
  } as ContentNode;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ConceptCardComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ConceptCardComponent);
    component = fixture.componentInstance;

    // Set required input before detectChanges
    component.concept = mockConcept;

    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Component Initialization', () => {
    it('should have defined component properties', () => {
      expect(component).toBeDefined();
      expect(typeof component).toBe('object');
    });
  });

  describe('Template', () => {
    it('should render without errors', () => {
      expect(fixture.nativeElement).toBeTruthy();
    });

    it('should have a root element', () => {
      const root = fixture.nativeElement.querySelector('[app-concept-card], .concept-card');
      expect(root || fixture.nativeElement.firstChild).toBeTruthy();
    });
  });
});
