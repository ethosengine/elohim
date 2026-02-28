/**
 * Offline Node Alert Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { of } from 'rxjs';

import { OfflineNodeAlertComponent } from './offline-node-alert.component';
import { ShefaComputeService } from '../../services/shefa-compute.service';
import { vi } from 'vitest';

describe('OfflineNodeAlertComponent', () => {
  let component: OfflineNodeAlertComponent;
  let fixture: ComponentFixture<OfflineNodeAlertComponent>;
  let mockShefaCompute: any;
  let mockRouter: any;

  beforeEach(async () => {
    mockShefaCompute = {
    getNodeTopology: vi.fn(),
    getComputeNeedsAssessment: vi.fn(),
  };
    mockShefaCompute.getNodeTopology.mockReturnValue(
      of({
        nodes: [],
        totalNodes: 0,
        onlineNodes: 0,
        offlineNodes: 0,
        degradedNodes: 0,
        clusterHealth: 'healthy',
        alerts: [],
        lastUpdated: new Date().toISOString(),
      })
    );
    mockShefaCompute.getComputeNeedsAssessment.mockReturnValue(
      of({
        currentCapacity: {
          totalCPUCores: 0,
          totalMemoryGB: 0,
          totalStorageGB: 0,
          totalBandwidthMbps: 0,
        },
        gaps: [],
        hasGaps: false,
        overallGapSeverity: 'none',
        recommendations: [],
        helpFlowUrl: '',
        helpFlowCTA: 'Get Help',
      })
    );

    mockRouter = {
    navigate: vi.fn(),
  };

    await TestBed.configureTestingModule({
      imports: [OfflineNodeAlertComponent],
      providers: [
        { provide: ShefaComputeService, useValue: mockShefaCompute },
        { provide: Router, useValue: mockRouter },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(OfflineNodeAlertComponent);
    component = fixture.componentInstance;

    // Set required input
    component.operatorId = 'test-operator';

    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
