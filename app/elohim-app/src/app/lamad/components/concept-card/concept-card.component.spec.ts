import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ConceptCardComponent } from './concept-card.component';
import { ContentNode, DistributionSummary } from '../../models/content-node.model';

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
      providers: [provideHttpClient(), provideHttpClientTesting()],
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

  describe('Distribution badge integration (T50)', () => {
    function withDistribution(d: Partial<DistributionSummary>): ContentNode {
      const summary: DistributionSummary = {
        replicaCount: 6,
        replicaTarget: 8,
        replicaHealth: 'healthy',
        projectorCount: 1,
        reachClass: 'public',
        diversityHint: { kind: 'region_metro', value: ['us-central'] },
        thisFetchSource: 'projected_via_doorway',
        lastVerifiedSeconds: 30,
        ...d,
      };
      return { ...mockConcept, distribution: summary };
    }

    it('renders <app-distribution-badge> when concept.distribution is present', () => {
      fixture.componentRef.setInput('concept', withDistribution({}));
      fixture.detectChanges();
      expect(
        fixture.nativeElement.querySelector('[data-testid="distribution-badge"]')
      ).toBeTruthy();
    });

    it('hides the badge when concept.distribution is absent', () => {
      fixture.componentRef.setInput('concept', { ...mockConcept, distribution: undefined });
      fixture.detectChanges();
      expect(fixture.nativeElement.querySelector('[data-testid="distribution-badge"]')).toBeFalsy();
    });
  });
});
