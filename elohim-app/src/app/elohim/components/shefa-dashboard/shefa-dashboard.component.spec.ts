import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NO_ERRORS_SCHEMA } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';

import { ShefaDashboardComponent } from './shefa-dashboard.component';
import { ShefaService } from '../../services/shefa.service';
import { CustodianSelectionService } from '../../services/custodian-selection.service';
import { HolochainClientService } from '../../services/holochain-client.service';

describe('ShefaDashboardComponent', () => {
  let component: ShefaDashboardComponent;
  let fixture: ComponentFixture<ShefaDashboardComponent>;
  let shefaServiceMock: jasmine.SpyObj<ShefaService>;
  let custodianSelectionMock: jasmine.SpyObj<CustodianSelectionService>;
  let holochainClientMock: jasmine.SpyObj<HolochainClientService>;

  beforeEach(async () => {
    shefaServiceMock = jasmine.createSpyObj('ShefaService', [
      'getMetricsForCustodian',
      'getAllMetrics',
    ]);

    custodianSelectionMock = jasmine.createSpyObj('CustodianSelectionService', [
      'selectCustodians',
    ]);

    holochainClientMock = jasmine.createSpyObj('HolochainClientService', [
      'callZome',
      'isConnected',
    ]);

    await TestBed.configureTestingModule({
      imports: [ShefaDashboardComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ShefaService, useValue: shefaServiceMock },
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
