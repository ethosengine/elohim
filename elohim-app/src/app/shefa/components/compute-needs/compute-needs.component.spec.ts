/**
 * Compute Needs Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ComputeNeedsComponent } from './compute-needs.component';
import { ShefaComputeService } from '../../services/shefa-compute.service';

describe('ComputeNeedsComponent', () => {
  let component: ComputeNeedsComponent;
  let fixture: ComponentFixture<ComputeNeedsComponent>;
  let mockShefaCompute: jasmine.SpyObj<ShefaComputeService>;

  beforeEach(async () => {
    mockShefaCompute = jasmine.createSpyObj('ShefaComputeService', [
      'getComputeNeedsAssessment',
    ]);
    mockShefaCompute.getComputeNeedsAssessment.and.returnValue(
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
      providers: [{ provide: ShefaComputeService, useValue: mockShefaCompute }],
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
