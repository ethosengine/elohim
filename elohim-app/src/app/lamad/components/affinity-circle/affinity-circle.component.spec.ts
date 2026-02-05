import { ComponentFixture, TestBed } from '@angular/core/testing';
import { AffinityCircleComponent } from './affinity-circle.component';

describe('AffinityCircleComponent', () => {
  let component: AffinityCircleComponent;
  let fixture: ComponentFixture<AffinityCircleComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [AffinityCircleComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(AffinityCircleComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Input Properties
  // ==========================================================================

  describe('Input Properties', () => {
    it('should have default affinity of 0', () => {
      expect(component.affinity).toBe(0);
    });

    it('should have default size of 80', () => {
      expect(component.size).toBe(80);
    });

    it('should accept custom affinity value', () => {
      component.affinity = 0.75;
      expect(component.affinity).toBe(0.75);
    });

    it('should accept custom size value', () => {
      component.size = 120;
      expect(component.size).toBe(120);
    });
  });

  // ==========================================================================
  // Circumference Calculation
  // ==========================================================================

  describe('circumference', () => {
    it('should calculate circumference for default size', () => {
      component.size = 80;
      const radius = (80 / 2) - 4;
      const expected = 2 * Math.PI * radius;
      expect(component.circumference).toBeCloseTo(expected, 2);
    });

    it('should calculate circumference for custom size', () => {
      component.size = 100;
      const radius = (100 / 2) - 4;
      const expected = 2 * Math.PI * radius;
      expect(component.circumference).toBeCloseTo(expected, 2);
    });

    it('should recalculate when size changes', () => {
      component.size = 80;
      const circumference1 = component.circumference;

      component.size = 100;
      const circumference2 = component.circumference;

      expect(circumference2).toBeGreaterThan(circumference1);
    });
  });

  // ==========================================================================
  // Stroke Dash Offset Calculation
  // ==========================================================================

  describe('strokeDashoffset', () => {
    it('should return full circumference for 0% affinity', () => {
      component.affinity = 0;
      expect(component.strokeDashoffset).toBe(component.circumference);
    });

    it('should return 0 for 100% affinity', () => {
      component.affinity = 1.0;
      expect(component.strokeDashoffset).toBeCloseTo(0, 2);
    });

    it('should return half circumference for 50% affinity', () => {
      component.affinity = 0.5;
      const expected = component.circumference * 0.5;
      expect(component.strokeDashoffset).toBeCloseTo(expected, 2);
    });

    it('should return quarter circumference for 75% affinity', () => {
      component.affinity = 0.75;
      const expected = component.circumference * 0.25;
      expect(component.strokeDashoffset).toBeCloseTo(expected, 2);
    });
  });

  // ==========================================================================
  // Percentage Calculation
  // ==========================================================================

  describe('getPercentage', () => {
    it('should return 0 for affinity of 0', () => {
      component.affinity = 0;
      expect(component.getPercentage()).toBe(0);
    });

    it('should return 100 for affinity of 1.0', () => {
      component.affinity = 1.0;
      expect(component.getPercentage()).toBe(100);
    });

    it('should return 50 for affinity of 0.5', () => {
      component.affinity = 0.5;
      expect(component.getPercentage()).toBe(50);
    });

    it('should return 75 for affinity of 0.75', () => {
      component.affinity = 0.75;
      expect(component.getPercentage()).toBe(75);
    });

    it('should round to nearest integer', () => {
      component.affinity = 0.678;
      expect(component.getPercentage()).toBe(68);
    });

    it('should handle edge case of 0.005', () => {
      component.affinity = 0.005;
      expect(component.getPercentage()).toBe(1);
    });
  });

  // ==========================================================================
  // Affinity Level Classification
  // ==========================================================================

  describe('getAffinityLevel', () => {
    it('should return "high" for affinity >= 0.8', () => {
      component.affinity = 0.8;
      expect(component.getAffinityLevel()).toBe('high');

      component.affinity = 0.9;
      expect(component.getAffinityLevel()).toBe('high');

      component.affinity = 1.0;
      expect(component.getAffinityLevel()).toBe('high');
    });

    it('should return "medium" for affinity >= 0.5 and < 0.8', () => {
      component.affinity = 0.5;
      expect(component.getAffinityLevel()).toBe('medium');

      component.affinity = 0.65;
      expect(component.getAffinityLevel()).toBe('medium');

      component.affinity = 0.79;
      expect(component.getAffinityLevel()).toBe('medium');
    });

    it('should return "low" for affinity >= 0.2 and < 0.5', () => {
      component.affinity = 0.2;
      expect(component.getAffinityLevel()).toBe('low');

      component.affinity = 0.35;
      expect(component.getAffinityLevel()).toBe('low');

      component.affinity = 0.49;
      expect(component.getAffinityLevel()).toBe('low');
    });

    it('should return "none" for affinity < 0.2', () => {
      component.affinity = 0;
      expect(component.getAffinityLevel()).toBe('none');

      component.affinity = 0.1;
      expect(component.getAffinityLevel()).toBe('none');

      component.affinity = 0.19;
      expect(component.getAffinityLevel()).toBe('none');
    });

    it('should handle boundary values correctly', () => {
      component.affinity = 0.2;
      expect(component.getAffinityLevel()).toBe('low');

      component.affinity = 0.5;
      expect(component.getAffinityLevel()).toBe('medium');

      component.affinity = 0.8;
      expect(component.getAffinityLevel()).toBe('high');
    });
  });

  // ==========================================================================
  // Stroke Color Mapping
  // ==========================================================================

  describe('getStrokeColor', () => {
    it('should return correct color for "none" level', () => {
      component.affinity = 0;
      expect(component.getStrokeColor()).toBe('#e0e0e0');
    });

    it('should return correct color for "low" level', () => {
      component.affinity = 0.3;
      expect(component.getStrokeColor()).toBe('#f57c00');
    });

    it('should return correct color for "medium" level', () => {
      component.affinity = 0.6;
      expect(component.getStrokeColor()).toBe('#1976d2');
    });

    it('should return correct color for "high" level', () => {
      component.affinity = 0.9;
      expect(component.getStrokeColor()).toBe('#2e7d32');
    });

    it('should return default color for undefined level', () => {
      spyOn(component, 'getAffinityLevel').and.returnValue('unknown' as any);
      expect(component.getStrokeColor()).toBe('#e0e0e0');
    });
  });

  // ==========================================================================
  // Template Rendering
  // ==========================================================================

  describe('Template Rendering', () => {
    it('should render SVG with correct size attributes', () => {
      component.size = 100;
      fixture.detectChanges();

      const svg = fixture.nativeElement.querySelector('svg');
      expect(svg.getAttribute('width')).toBe('100');
      expect(svg.getAttribute('height')).toBe('100');
      expect(svg.getAttribute('viewBox')).toBe('0 0 100 100');
    });

    it('should display correct percentage text', () => {
      component.affinity = 0.75;
      fixture.detectChanges();

      const percentageText = fixture.nativeElement.querySelector('.percentage-text');
      expect(percentageText.textContent).toBe('75%');
    });

    it('should apply correct affinity level class', () => {
      component.affinity = 0.9;
      fixture.detectChanges();

      const container = fixture.nativeElement.querySelector('.affinity-circle');
      expect(container.classList.contains('affinity-high')).toBe(true);
    });

    it('should apply correct size to container', () => {
      component.size = 120;
      fixture.detectChanges();

      const container = fixture.nativeElement.querySelector('.affinity-circle');
      expect(container.style.width).toBe('120px');
      expect(container.style.height).toBe('120px');
    });

    it('should set progress circle stroke color', () => {
      component.affinity = 0.6;
      fixture.detectChanges();

      const progressCircle = fixture.nativeElement.querySelectorAll('circle')[1];
      expect(progressCircle.getAttribute('stroke')).toBe('#1976d2');
    });
  });

  // ==========================================================================
  // Integration Tests
  // ==========================================================================

  describe('Integration - Affinity Changes', () => {
    it('should update all visual properties when affinity changes', () => {
      component.affinity = 0.3;
      fixture.detectChanges();

      expect(component.getAffinityLevel()).toBe('low');
      expect(component.getStrokeColor()).toBe('#f57c00');
      expect(component.getPercentage()).toBe(30);

      const container = fixture.nativeElement.querySelector('.affinity-circle');
      expect(container.classList.contains('affinity-low')).toBe(true);
    });

    it('should handle rapid affinity changes', () => {
      const affinities = [0, 0.25, 0.5, 0.75, 1.0];

      affinities.forEach(affinity => {
        component.affinity = affinity;
        fixture.detectChanges();

        const percentageText = fixture.nativeElement.querySelector('.percentage-text');
        expect(percentageText.textContent).toBe(`${Math.round(affinity * 100)}%`);
      });
    });
  });
});
