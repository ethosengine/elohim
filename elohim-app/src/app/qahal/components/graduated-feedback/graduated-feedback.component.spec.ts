import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { FormsModule } from '@angular/forms';

import { of, throwError, Subject } from 'rxjs';

import {
  GovernanceSignalService,
  FeedbackStats,
} from '@app/elohim/services/governance-signal.service';

import { GraduatedFeedbackComponent, FeedbackContext } from './graduated-feedback.component';

describe('GraduatedFeedbackComponent', () => {
  let component: GraduatedFeedbackComponent;
  let fixture: ComponentFixture<GraduatedFeedbackComponent>;
  let mockSignalService: jasmine.SpyObj<GovernanceSignalService>;
  let signalChanges$: Subject<any>;

  const mockStats: FeedbackStats = {
    totalResponses: 10,
    distribution: {
      'Not Useful': 1,
      'Slightly Useful': 2,
      Useful: 4,
      'Very Useful': 2,
      Transformative: 1,
    },
    averagePosition: 0.6,
    averageIntensity: 6,
    contexts: ['usefulness'],
  };

  beforeEach(async () => {
    signalChanges$ = new Subject();
    mockSignalService = jasmine.createSpyObj(
      'GovernanceSignalService',
      ['recordGraduatedFeedback', 'getFeedbackStats'],
      {
        signalChanges$: signalChanges$.asObservable(),
      }
    );
    mockSignalService.getFeedbackStats.and.returnValue(of(mockStats));
    mockSignalService.recordGraduatedFeedback.and.returnValue(of(true));

    await TestBed.configureTestingModule({
      imports: [GraduatedFeedbackComponent, FormsModule],
      providers: [{ provide: GovernanceSignalService, useValue: mockSignalService }],
    }).compileComponents();

    fixture = TestBed.createComponent(GraduatedFeedbackComponent);
    component = fixture.componentInstance;
    component.contentId = 'content-1';
  });

  afterEach(() => {
    fixture.destroy();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should have default context as usefulness', () => {
      expect(component.context).toBe('usefulness');
    });

    it('should have default intensity of 5', () => {
      expect(component.intensity).toBe(5);
    });

    it('should have no selected position initially', () => {
      expect(component.selectedPosition).toBeNull();
    });

    it('should load stats when showAggregates is true', fakeAsync(() => {
      component.showAggregates = true;
      fixture.detectChanges();
      tick();

      expect(mockSignalService.getFeedbackStats).toHaveBeenCalledWith('content-1');
      expect(component.stats).toEqual(mockStats);
    }));

    it('should not load stats when showAggregates is false', fakeAsync(() => {
      component.showAggregates = false;
      fixture.detectChanges();
      tick();

      expect(mockSignalService.getFeedbackStats).not.toHaveBeenCalled();
    }));
  });

  describe('scales', () => {
    it('should have 5 contexts defined', () => {
      const contexts: FeedbackContext[] = [
        'accuracy',
        'usefulness',
        'proposal',
        'clarity',
        'relevance',
      ];
      contexts.forEach(ctx => {
        expect(component.scales[ctx]).toBeDefined();
      });
    });

    it('should have 5 positions per scale', () => {
      Object.values(component.scales).forEach(scale => {
        expect(scale.positions.length).toBe(5);
      });
    });

    it('should have positions indexed 0 to 1', () => {
      Object.values(component.scales).forEach(scale => {
        expect(scale.positions[0].index).toBe(0);
        expect(scale.positions[4].index).toBe(1);
      });
    });
  });

  describe('currentScale getter', () => {
    it('should return scale for current context', () => {
      component.context = 'accuracy';
      expect(component.currentScale.label).toContain('accurate');
    });

    it('should change when context changes', () => {
      component.context = 'usefulness';
      const usefulnessLabel = component.currentScale.label;

      component.context = 'clarity';
      const clarityLabel = component.currentScale.label;

      expect(usefulnessLabel).not.toBe(clarityLabel);
    });
  });

  describe('selectedPositionData getter', () => {
    it('should return null when no position selected', () => {
      component.selectedPosition = null;
      expect(component.selectedPositionData).toBeNull();
    });

    it('should return position data when selected', () => {
      component.context = 'usefulness';
      component.selectedPosition = 0.5;
      expect(component.selectedPositionData?.label).toBe('Useful');
    });
  });

  describe('reasoningRequired getter', () => {
    it('should return true when requiresReasoning input is true', () => {
      component.requiresReasoning = true;
      component.selectedPosition = 0.5;
      expect(component.reasoningRequired).toBeTrue();
    });

    it('should return true for positions that require reasoning', () => {
      component.context = 'proposal';
      component.selectedPosition = 0; // Block position requires reasoning
      expect(component.reasoningRequired).toBeTrue();
    });

    it('should return false for normal positions', () => {
      component.requiresReasoning = false;
      component.context = 'usefulness';
      component.selectedPosition = 0.5;
      expect(component.reasoningRequired).toBeFalse();
    });
  });

  describe('canSubmit getter', () => {
    it('should return false when no position selected', () => {
      component.selectedPosition = null;
      expect(component.canSubmit).toBeFalse();
    });

    it('should return true when position selected and no reasoning required', () => {
      component.context = 'usefulness';
      component.selectedPosition = 0.5;
      expect(component.canSubmit).toBeTrue();
    });

    it('should return false when reasoning required but empty', () => {
      component.context = 'proposal';
      component.selectedPosition = 0; // Block
      component.reasoning = '';
      expect(component.canSubmit).toBeFalse();
    });

    it('should return true when reasoning required and provided', () => {
      component.context = 'proposal';
      component.selectedPosition = 0; // Block
      component.reasoning = 'My reasoning for blocking';
      expect(component.canSubmit).toBeTrue();
    });
  });

  describe('selectPosition()', () => {
    it('should set selected position', () => {
      const position = component.currentScale.positions[2]; // Useful
      component.selectPosition(position);

      expect(component.selectedPosition).toBe(0.5);
    });

    it('should show reasoning field when position requires it', () => {
      component.context = 'proposal';
      const blockPosition = component.currentScale.positions[0]; // Block
      component.selectPosition(blockPosition);

      expect(component.showReasoningField).toBeTrue();
    });

    it('should show reasoning field when component requires it', () => {
      component.requiresReasoning = true;
      const position = component.currentScale.positions[2];
      component.selectPosition(position);

      expect(component.showReasoningField).toBeTrue();
    });
  });

  describe('toggleReasoning()', () => {
    it('should toggle reasoning field visibility', () => {
      component.showReasoningField = false;
      component.toggleReasoning();
      expect(component.showReasoningField).toBeTrue();

      component.toggleReasoning();
      expect(component.showReasoningField).toBeFalse();
    });
  });

  describe('submit()', () => {
    it('should not submit when canSubmit is false', () => {
      component.selectedPosition = null;
      component.submit();

      expect(mockSignalService.recordGraduatedFeedback).not.toHaveBeenCalled();
    });

    it('should not submit when already submitting', () => {
      component.selectedPosition = 0.5;
      component.isSubmitting = true;
      component.submit();

      expect(mockSignalService.recordGraduatedFeedback).not.toHaveBeenCalled();
    });

    it('should submit feedback with correct data', fakeAsync(() => {
      component.context = 'usefulness';
      component.selectedPosition = 0.5;
      component.intensity = 7;
      component.reasoning = 'Very helpful content';

      component.submit();
      tick();

      expect(mockSignalService.recordGraduatedFeedback).toHaveBeenCalledWith(
        'content-1',
        jasmine.objectContaining({
          context: 'usefulness',
          position: 'Useful',
          positionIndex: 0.5,
          intensity: 7,
          reasoning: 'Very helpful content',
        })
      );
    }));

    it('should not include reasoning when empty', fakeAsync(() => {
      component.context = 'usefulness';
      component.selectedPosition = 0.5;
      component.reasoning = '   ';

      component.submit();
      tick();

      expect(mockSignalService.recordGraduatedFeedback).toHaveBeenCalledWith(
        'content-1',
        jasmine.objectContaining({
          reasoning: undefined,
        })
      );
    }));

    it('should set hasSubmitted on success', fakeAsync(() => {
      component.selectedPosition = 0.5;
      component.submit();
      tick();

      expect(component.hasSubmitted).toBeTrue();
    }));

    it('should emit feedbackSubmitted on success', fakeAsync(() => {
      spyOn(component.feedbackSubmitted, 'emit');
      component.selectedPosition = 0.5;
      component.submit();
      tick();

      expect(component.feedbackSubmitted.emit).toHaveBeenCalled();
    }));

    it('should refresh stats on success', fakeAsync(() => {
      component.showAggregates = true;
      fixture.detectChanges();
      tick();

      mockSignalService.getFeedbackStats.calls.reset();
      component.selectedPosition = 0.5;
      component.submit();
      tick();

      expect(mockSignalService.getFeedbackStats).toHaveBeenCalled();
    }));

    it('should handle submission error', fakeAsync(() => {
      mockSignalService.recordGraduatedFeedback.and.returnValue(
        throwError(() => new Error('Failed'))
      );
      component.selectedPosition = 0.5;
      component.submit();
      tick();

      expect(component.isSubmitting).toBeFalse();
      expect(component.hasSubmitted).toBeFalse();
    }));
  });

  describe('reset()', () => {
    it('should reset all form state', () => {
      component.selectedPosition = 0.5;
      component.intensity = 8;
      component.reasoning = 'Some reasoning';
      component.showReasoningField = true;
      component.hasSubmitted = true;

      component.reset();

      expect(component.selectedPosition).toBeNull();
      expect(component.intensity).toBe(5);
      expect(component.reasoning).toBe('');
      expect(component.showReasoningField).toBeFalse();
      expect(component.hasSubmitted).toBeFalse();
    });
  });

  describe('getDistributionWidth()', () => {
    beforeEach(fakeAsync(() => {
      component.showAggregates = true;
      fixture.detectChanges();
      tick();
    }));

    it('should calculate width percentage correctly', () => {
      const position = component.currentScale.positions[2]; // Useful with 4 out of 10
      const width = component.getDistributionWidth(position);
      expect(width).toBe(40);
    });

    it('should return 0 when no stats', () => {
      component.stats = null;
      const position = component.currentScale.positions[0];
      expect(component.getDistributionWidth(position)).toBe(0);
    });

    it('should return 0 when total responses is 0', () => {
      component.stats = { ...mockStats, totalResponses: 0 };
      const position = component.currentScale.positions[0];
      expect(component.getDistributionWidth(position)).toBe(0);
    });

    it('should return 0 for position not in distribution', () => {
      component.stats = { ...mockStats, distribution: {} };
      const position = component.currentScale.positions[0];
      expect(component.getDistributionWidth(position)).toBe(0);
    });
  });

  describe('getIntensityLabel()', () => {
    it('should return "Slightly" for intensity 1-2', () => {
      component.intensity = 1;
      expect(component.getIntensityLabel()).toBe('Slightly');

      component.intensity = 2;
      expect(component.getIntensityLabel()).toBe('Slightly');
    });

    it('should return "Moderately" for intensity 3-4', () => {
      component.intensity = 3;
      expect(component.getIntensityLabel()).toBe('Moderately');

      component.intensity = 4;
      expect(component.getIntensityLabel()).toBe('Moderately');
    });

    it('should return "Fairly" for intensity 5-6', () => {
      component.intensity = 5;
      expect(component.getIntensityLabel()).toBe('Fairly');

      component.intensity = 6;
      expect(component.getIntensityLabel()).toBe('Fairly');
    });

    it('should return "Strongly" for intensity 7-8', () => {
      component.intensity = 7;
      expect(component.getIntensityLabel()).toBe('Strongly');

      component.intensity = 8;
      expect(component.getIntensityLabel()).toBe('Strongly');
    });

    it('should return "Very Strongly" for intensity 9-10', () => {
      component.intensity = 9;
      expect(component.getIntensityLabel()).toBe('Very Strongly');

      component.intensity = 10;
      expect(component.getIntensityLabel()).toBe('Very Strongly');
    });
  });

  describe('formatPercentage()', () => {
    it('should format percentage correctly', () => {
      expect(component.formatPercentage(0.5)).toBe('50%');
      expect(component.formatPercentage(0.75)).toBe('75%');
      expect(component.formatPercentage(1)).toBe('100%');
    });

    it('should round to nearest integer', () => {
      expect(component.formatPercentage(0.333)).toBe('33%');
      expect(component.formatPercentage(0.666)).toBe('67%');
    });
  });

  describe('signal changes subscription', () => {
    it('should refresh stats when relevant signal change occurs', fakeAsync(() => {
      component.showAggregates = true;
      fixture.detectChanges();
      tick();

      mockSignalService.getFeedbackStats.calls.reset();

      signalChanges$.next({
        type: 'graduated-feedback',
        contentId: 'content-1',
      });
      tick();

      expect(mockSignalService.getFeedbackStats).toHaveBeenCalled();
    }));

    it('should not refresh for different content', fakeAsync(() => {
      component.showAggregates = true;
      fixture.detectChanges();
      tick();

      mockSignalService.getFeedbackStats.calls.reset();

      signalChanges$.next({
        type: 'graduated-feedback',
        contentId: 'different-content',
      });
      tick();

      expect(mockSignalService.getFeedbackStats).not.toHaveBeenCalled();
    }));

    it('should not refresh for different signal type', fakeAsync(() => {
      component.showAggregates = true;
      fixture.detectChanges();
      tick();

      mockSignalService.getFeedbackStats.calls.reset();

      signalChanges$.next({
        type: 'reaction',
        contentId: 'content-1',
      });
      tick();

      expect(mockSignalService.getFeedbackStats).not.toHaveBeenCalled();
    }));
  });
});
