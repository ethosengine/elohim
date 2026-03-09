import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NO_ERRORS_SCHEMA } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';

import { ShefaDashboardComponent } from './shefa-dashboard.component';
import { CUSTODIAN_METRICS } from '@app/shefa';
import { CustodianSelectionService } from '../../services/custodian-selection.service';
import { HolochainClientService } from '../../services/holochain-client.service';
import { vi } from 'vitest';

describe('ShefaDashboardComponent', () => {
  let component: ShefaDashboardComponent;
  let fixture: ComponentFixture<ShefaDashboardComponent>;
  let shefaServiceMock: any;
  let custodianSelectionMock: any;
  let holochainClientMock: any;

  beforeEach(async () => {
    shefaServiceMock = {
      getMetricsForCustodian: vi.fn(),
      getAllMetrics: vi.fn(),
    };

    custodianSelectionMock = {
      selectCustodians: vi.fn(),
    };

    holochainClientMock = {
      callZome: vi.fn(),
      isConnected: vi.fn(),
    };

    await TestBed.configureTestingModule({
      imports: [ShefaDashboardComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: CUSTODIAN_METRICS, useValue: shefaServiceMock },
        { provide: CustodianSelectionService, useValue: custodianSelectionMock },
        { provide: HolochainClientService, useValue: holochainClientMock },
      ],
      schemas: [NO_ERRORS_SCHEMA],
    }).compileComponents();

    fixture = TestBed.createComponent(ShefaDashboardComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
