/**
 * Compute Needs Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ComputeNeedsComponent } from './compute-needs.component';
import { COMPUTE_DASHBOARD } from '../../interfaces';
import { vi } from 'vitest';

describe('ComputeNeedsComponent', () => {
  let component: ComputeNeedsComponent;
  let fixture: ComponentFixture<ComputeNeedsComponent>;
  let mockShefaCompute: any;

  beforeEach(async () => {
    mockShefaCompute = {
      getComputeNeedsAssessment: vi.fn(),
    };
    mockShefaCompute.getComputeNeedsAssessment.mockReturnValue(
      of({
        operatorId: 'test-operator',
        assessmentDate: new Date().toISOString(),
        gaps: [],
        recommendations: [],
        overallGapSeverity: 'none',
        currentCapacity: {},
        hasGaps: false,
        helpFlowUrl: '',
        helpFlowCTA: '',
      } as any)
    );

    await TestBed.configureTestingModule({
      imports: [ComputeNeedsComponent],
      providers: [{ provide: COMPUTE_DASHBOARD, useValue: mockShefaCompute }],
    }).compileComponents();

    fixture = TestBed.createComponent(ComputeNeedsComponent);
    component = fixture.componentInstance;

    // Set required input
    component.operatorId = 'test-operator';
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
