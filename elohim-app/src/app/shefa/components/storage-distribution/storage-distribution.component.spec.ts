/**
 * Storage Distribution Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { StorageDistributionComponent } from './storage-distribution.component';
import { ShefaComputeService } from '../../services/shefa-compute.service';

describe('StorageDistributionComponent', () => {
  let component: StorageDistributionComponent;
  let fixture: ComponentFixture<StorageDistributionComponent>;
  let mockShefaCompute: jasmine.SpyObj<ShefaComputeService>;

  beforeEach(async () => {
    mockShefaCompute = jasmine.createSpyObj('ShefaComputeService', ['getStorageContentDistribution']);
    mockShefaCompute.getStorageContentDistribution.and.returnValue(
      of({
        byContentType: [],
        byReachLevel: [],
        byNode: [],
        totalContent: {
          items: 0,
          sizeGB: 0,
          replicaCount: 0,
        },
        replicationSummary: {
          overReplicated: 0,
          underReplicated: 0,
          metTarget: 0,
        },
      })
    );

    await TestBed.configureTestingModule({
      imports: [StorageDistributionComponent],
      providers: [{ provide: ShefaComputeService, useValue: mockShefaCompute }],
    }).compileComponents();

    fixture = TestBed.createComponent(StorageDistributionComponent);
    component = fixture.componentInstance;

    // Set required input
    component.operatorId = 'test-operator';

    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
