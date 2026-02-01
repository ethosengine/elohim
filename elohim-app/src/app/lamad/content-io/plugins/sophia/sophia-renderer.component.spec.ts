import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { RouterModule } from '@angular/router';
import { NoopAnimationsModule } from '@angular/platform-browser/animations';

import { SophiaRendererComponent } from './sophia-renderer.component';
import { SophiaWrapperComponent } from './sophia-wrapper.component';
import { ContentNode } from '../../../models/content-node.model';
import type { Recognition, Moment } from './sophia-moment.model';

// Mock SophiaWrapperComponent
@Component({
  selector: 'app-sophia-question',
  standalone: true,
  template: '<div class="mock-sophia-question"></div>',
})
class MockSophiaWrapperComponent {
  @Input() moment: Moment | null = null;
  @Input() mode = 'mastery';
  @Input() initialUserInput: Record<string, unknown> | null = null;
  @Input() reviewMode = false;

  @Output() recognized = new EventEmitter<Recognition>();
  @Output() answerChanged = new EventEmitter<boolean>();
  @Output() ready = new EventEmitter<void>();

  private mockRecognition: Recognition | null = null;

  setMockRecognition(recognition: Recognition): void {
    this.mockRecognition = recognition;
  }

  getRecognition(): Recognition | null {
    return this.mockRecognition;
  }
}

describe('SophiaRendererComponent', () => {
  let component: SophiaRendererComponent;
  let fixture: ComponentFixture<SophiaRendererComponent>;

  const createMockNode = (overrides: Partial<ContentNode> = {}): ContentNode => ({
    id: 'test-quiz-1',
    title: 'Test Quiz',
    description: 'A test quiz',
    content: [
      {
        id: 'moment-1',
        purpose: 'mastery',
        content: { content: 'Question 1', widgets: {} },
      },
      {
        id: 'moment-2',
        purpose: 'mastery',
        content: { content: 'Question 2', widgets: {} },
      },
    ],
    contentType: 'assessment',
    contentFormat: 'sophia-quiz-json',
    tags: ['quiz', 'test'],
    relatedNodeIds: [],
    metadata: {},
    ...overrides,
  });

  const createMockRecognition = (overrides: Partial<Recognition> = {}): Recognition => ({
    momentId: 'moment-1',
    purpose: 'mastery',
    userInput: { 'widget-1': 'answer' },
    mastery: {
      demonstrated: true,
      score: 100,
      total: 100,
      message: 'Correct!',
    },
    timestamp: Date.now(),
    ...overrides,
  });

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [SophiaRendererComponent, RouterModule.forRoot([]), NoopAnimationsModule],
    })
      .overrideComponent(SophiaRendererComponent, {
        remove: { imports: [SophiaWrapperComponent] },
        add: { imports: [MockSophiaWrapperComponent] },
      })
      .compileComponents();

    fixture = TestBed.createComponent(SophiaRendererComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Initialization', () => {
    it('should initialize with default values', () => {
      expect(component.moments).toEqual([]);
      expect(component.currentMomentIndex).toBe(0);
      expect(component.hasAnswer).toBeFalse();
      expect(component.isSubmitting).toBeFalse();
      expect(component.showFeedback).toBeFalse();
      expect(component.showResults).toBeFalse();
      expect(component.assessmentMode).toBe('mastery');
    });

    it('should load moments when node input changes', () => {
      component.node = createMockNode();
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.moments.length).toBe(2);
    });

    it('should detect mastery mode from moment purpose', () => {
      component.node = createMockNode();
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.assessmentMode).toBe('mastery');
      expect(component.modeConfig.showFeedback).toBeTrue();
      expect(component.modeConfig.showCorrectness).toBeTrue();
    });

    it('should detect discovery mode from moment purpose', () => {
      component.node = createMockNode({
        content: [{ id: 'm1', purpose: 'discovery', content: { content: 'Q1', widgets: {} } }],
      });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.assessmentMode).toBe('discovery');
      expect(component.modeConfig.showFeedback).toBeFalse();
      expect(component.modeConfig.trackSubscales).toBeTrue();
    });

    it('should detect reflection mode from moment purpose', () => {
      component.node = createMockNode({
        content: [{ id: 'm1', purpose: 'reflection', content: { content: 'Q1', widgets: {} } }],
      });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.assessmentMode).toBe('reflection');
    });

    it('should parse JSON string content', () => {
      component.node = createMockNode({
        content: JSON.stringify([
          { id: 'm1', purpose: 'mastery', content: { content: 'Q1', widgets: {} } },
        ]),
      });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.moments.length).toBe(1);
    });

    it('should handle single object content', () => {
      component.node = createMockNode({
        content: { id: 'm1', purpose: 'mastery', content: { content: 'Q1', widgets: {} } },
      });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.moments.length).toBe(1);
    });

    it('should convert Perseus format to Moment', () => {
      component.node = createMockNode({
        content: [
          {
            id: 'p1',
            question: { content: 'Perseus question', widgets: {} },
            hints: ['hint1'],
          },
        ],
      });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.moments.length).toBe(1);
      expect(component.moments[0].purpose).toBe('mastery');
    });
  });

  describe('Computed Properties', () => {
    beforeEach(() => {
      component.node = createMockNode();
      component.ngOnChanges({ node: { currentValue: component.node } as any });
    });

    it('should return title from node', () => {
      expect(component.title).toBe('Test Quiz');
    });

    it('should return current moment', () => {
      expect(component.currentMoment?.id).toBe('moment-1');
    });

    it('should return total moments count', () => {
      expect(component.totalMoments).toBe(2);
    });

    it('should calculate progress percentage', () => {
      expect(component.progressPercentage).toBe(50); // 1 of 2 = 50%
      component.currentMomentIndex = 1;
      expect(component.progressPercentage).toBe(100); // 2 of 2 = 100%
    });

    it('should detect last moment', () => {
      expect(component.isLastMoment).toBeFalse();
      component.currentMomentIndex = 1;
      expect(component.isLastMoment).toBeTrue();
    });

    it('should calculate mastery score', () => {
      component.demonstratedCount = 1;
      expect(component.masteryScorePercent).toBe(50);
      component.demonstratedCount = 2;
      expect(component.masteryScorePercent).toBe(100);
    });

    it('should determine if mastery passed (70% threshold)', () => {
      component.demonstratedCount = 1;
      expect(component.masteryPassed).toBeFalse(); // 50%
      component.demonstratedCount = 2;
      expect(component.masteryPassed).toBeTrue(); // 100%
    });
  });

  describe('Event Handlers', () => {
    beforeEach(() => {
      component.node = createMockNode();
      component.ngOnChanges({ node: { currentValue: component.node } as any });
      fixture.detectChanges();
    });

    it('should handle answer change events', () => {
      component.handleAnswerChange(true);
      expect(component.hasAnswer).toBeTrue();

      component.handleAnswerChange(false);
      expect(component.hasAnswer).toBeFalse();
    });

    it('should handle recognition events', () => {
      const recognition = createMockRecognition();
      component.handleRecognition(recognition);

      expect(component.lastRecognition).toBe(recognition);
    });

    it('should handle ready events', () => {
      // Should not throw
      expect(() => component.handleReady()).not.toThrow();
    });
  });

  describe('Mastery Mode Submission', () => {
    beforeEach(() => {
      component.node = createMockNode();
      component.ngOnChanges({ node: { currentValue: component.node } as any });
      fixture.detectChanges();
    });

    it('should not submit without moment component', () => {
      component.submitAnswer();
      expect(component.isSubmitting).toBeFalse();
    });

    it('should show feedback after submitting correct answer', () => {
      // Mock the moment component
      const mockWrapper = new MockSophiaWrapperComponent();
      mockWrapper.setMockRecognition(
        createMockRecognition({
          mastery: { demonstrated: true, score: 100, total: 100, message: 'Correct!' },
        })
      );
      (component as any).momentComponent = mockWrapper;
      component.hasAnswer = true;

      component.submitAnswer();

      expect(component.showFeedback).toBeTrue();
      expect(component.showNextButton).toBeTrue();
      expect(component.demonstratedCount).toBe(1);
    });

    it('should show feedback after submitting incorrect answer', () => {
      const mockWrapper = new MockSophiaWrapperComponent();
      mockWrapper.setMockRecognition(
        createMockRecognition({
          mastery: { demonstrated: false, score: 0, total: 100, message: 'Incorrect' },
        })
      );
      (component as any).momentComponent = mockWrapper;
      component.hasAnswer = true;

      component.submitAnswer();

      expect(component.showFeedback).toBeTrue();
      expect(component.demonstratedCount).toBe(0);
    });

    it('should not double-submit when already submitting', () => {
      component.isSubmitting = true;
      component.submitAnswer();
      expect(component.isSubmitting).toBeTrue();
    });

    it('should not submit when feedback is already showing', () => {
      component.showFeedback = true;
      const mockWrapper = new MockSophiaWrapperComponent();
      mockWrapper.setMockRecognition(createMockRecognition());
      (component as any).momentComponent = mockWrapper;

      component.submitAnswer();
      expect(component.recognitions.length).toBe(0);
    });
  });

  describe('Navigation', () => {
    beforeEach(() => {
      component.node = createMockNode();
      component.ngOnChanges({ node: { currentValue: component.node } as any });
      fixture.detectChanges();
    });

    it('should advance to next moment', () => {
      component.nextMoment();

      expect(component.currentMomentIndex).toBe(1);
      expect(component.hasAnswer).toBeFalse();
      expect(component.showFeedback).toBeFalse();
    });

    it('should show results on last moment', () => {
      component.currentMomentIndex = 1; // last moment
      component.nextMoment();

      expect(component.showResults).toBeTrue();
    });

    it('should go to previous moment', () => {
      component.currentMomentIndex = 1;
      component.previousMoment();

      expect(component.currentMomentIndex).toBe(0);
      expect(component.hasAnswer).toBeTrue();
    });

    it('should not go before first moment', () => {
      component.currentMomentIndex = 0;
      component.previousMoment();

      expect(component.currentMomentIndex).toBe(0);
    });

    it('should restore previous answer when navigating back', () => {
      // Set up stored answer
      const mockWrapper = new MockSophiaWrapperComponent();
      mockWrapper.setMockRecognition(createMockRecognition({ momentId: 'moment-1' }));
      (component as any).momentComponent = mockWrapper;
      component.hasAnswer = true;

      // Submit first answer and advance
      component.submitAnswer();
      component.nextMoment();

      // Navigate back
      component.previousMoment();

      expect(component.currentMomentIndex).toBe(0);
      expect(component.hasAnswer).toBeTrue();
    });
  });

  describe('Discovery/Reflection Mode', () => {
    beforeEach(() => {
      component.node = createMockNode({
        content: [
          {
            id: 'm1',
            purpose: 'discovery',
            content: { content: 'Q1', widgets: {} },
            subscaleContributions: { openness: 0.7 },
          },
          {
            id: 'm2',
            purpose: 'discovery',
            content: { content: 'Q2', widgets: {} },
            subscaleContributions: { empathy: 0.8 },
          },
        ],
      });
      component.ngOnChanges({ node: { currentValue: component.node } as any });
      fixture.detectChanges();
    });

    it('should auto-advance after submission in discovery mode', () => {
      const mockWrapper = new MockSophiaWrapperComponent();
      mockWrapper.setMockRecognition(
        createMockRecognition({
          momentId: 'm1',
          reflection: { subscaleContributions: { openness: 0.7 } },
        })
      );
      (component as any).momentComponent = mockWrapper;
      component.hasAnswer = true;

      component.submitAnswer();

      expect(component.currentMomentIndex).toBe(1);
      expect(component.showFeedback).toBeFalse();
    });

    it('should show results after last discovery moment', () => {
      const mockWrapper = new MockSophiaWrapperComponent();
      (component as any).momentComponent = mockWrapper;
      component.hasAnswer = true;
      component.currentMomentIndex = 1;

      mockWrapper.setMockRecognition(createMockRecognition({ momentId: 'm2' }));
      component.submitAnswer();

      expect(component.showResults).toBeTrue();
    });
  });

  describe('Completion Events', () => {
    let emittedEvent: any;

    beforeEach(() => {
      component.node = createMockNode();
      component.ngOnChanges({ node: { currentValue: component.node } as any });
      fixture.detectChanges();

      component.complete.subscribe((event: any) => {
        emittedEvent = event;
      });
    });

    it('should emit mastery completion event', () => {
      component.demonstratedCount = 2;
      component.showResults = true;

      component.completeAndContinue();

      expect(emittedEvent).toBeDefined();
      expect(emittedEvent.type).toBe('quiz');
      expect(emittedEvent.passed).toBeTrue();
      expect(emittedEvent.score).toBe(100);
    });

    it('should emit failed mastery event when score is low', () => {
      component.demonstratedCount = 1;
      component.showResults = true;

      component.completeAndContinue();

      expect(emittedEvent.passed).toBeFalse();
      expect(emittedEvent.score).toBe(50);
    });

    it('should emit discovery completion event with subscale data', () => {
      // Set up discovery mode
      component.assessmentMode = 'discovery';
      component.aggregatedReflection = {
        subscaleTotals: { openness: 0.7, empathy: 0.3 },
        subscaleCounts: { openness: 1, empathy: 1 },
        normalizedScores: { openness: 0.7, empathy: 0.3 },
        momentCount: 2,
        momentIds: ['m1', 'm2'],
        aggregatedAt: Date.now(),
      };
      component.showResults = true;

      component.completeAndContinue();

      expect(emittedEvent.passed).toBeTrue();
      expect(emittedEvent.score).toBe(100);
      expect(emittedEvent.details.subscaleTotals).toBeDefined();
      expect(emittedEvent.details.primarySubscale).toBe('openness');
    });

    it('should handle missing aggregated reflection gracefully', () => {
      component.assessmentMode = 'reflection';
      component.aggregatedReflection = null;
      component.showResults = true;

      component.completeAndContinue();

      expect(emittedEvent.passed).toBeTrue();
      expect(emittedEvent.score).toBe(100);
    });
  });

  describe('Content Renderer Interface', () => {
    it('should implement ContentRenderer interface with node input', () => {
      const node = createMockNode();
      component.node = node;

      expect(component.node).toBe(node);
    });

    it('should implement InteractiveRenderer with complete output', () => {
      expect(component.complete).toBeDefined();
      expect(component.complete instanceof EventEmitter).toBeTrue();
    });
  });

  describe('Input Properties', () => {
    it('should accept showHeader input', () => {
      component.showHeader = false;
      expect(component.showHeader).toBeFalse();
    });

    it('should accept reviewMode input', () => {
      component.reviewMode = true;
      expect(component.reviewMode).toBeTrue();
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty content gracefully', () => {
      component.node = createMockNode({ content: undefined });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.moments).toEqual([]);
    });

    it('should handle invalid JSON content', () => {
      component.node = createMockNode({ content: 'invalid json {' });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.moments).toEqual([]);
    });

    it('should handle zero moments', () => {
      component.node = createMockNode({ content: [] });
      component.ngOnChanges({ node: { currentValue: component.node } as any });

      expect(component.totalMoments).toBe(0);
      expect(component.progressPercentage).toBe(0);
      expect(component.masteryScorePercent).toBe(0);
    });

    it('should handle node without title', () => {
      component.node = createMockNode({ title: undefined as any });
      expect(component.title).toBe('Assessment');
    });
  });

  describe('Cleanup', () => {
    it('should complete destroy$ subject on destroy', () => {
      const destroySpy = spyOn((component as any).destroy$, 'complete');

      component.ngOnDestroy();

      expect(destroySpy).toHaveBeenCalled();
    });
  });
});
