/**
 * Custodian View Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { of } from 'rxjs';

import { CustodianViewComponent } from './custodian-view.component';
import { ShefaComputeService } from '../../services/shefa-compute.service';

describe('CustodianViewComponent', () => {
  let component: CustodianViewComponent;
  let fixture: ComponentFixture<CustodianViewComponent>;
  let mockShefaCompute: jasmine.SpyObj<ShefaComputeService>;

  beforeEach(async () => {
    mockShefaCompute = jasmine.createSpyObj('ShefaComputeService', [
      'getBidirectionalCustodianView',
    ]);
    mockShefaCompute.getBidirectionalCustodianView.and.returnValue(
      of({
        helping: [],
        helpingCount: 0,
        helpingTotalGB: 0,
        beingHelpedBy: [],
        beingHelpedByCount: 0,
        beingHelpedByTotalGB: 0,
        mutualAidBalance: {
          ratio: 1.0,
          status: 'balanced',
          message: 'Balanced mutual aid',
        },
        communityStrength: 'weak',
      })
    );

    await TestBed.configureTestingModule({
      imports: [CustodianViewComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ShefaComputeService, useValue: mockShefaCompute },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(CustodianViewComponent);
    component = fixture.componentInstance;
    component.operatorId = 'test-operator-id';
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
