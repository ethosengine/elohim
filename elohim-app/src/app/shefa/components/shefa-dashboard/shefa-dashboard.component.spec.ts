/**
 * Shefa Dashboard Component Tests
 *
 * Tests the main dashboard component providing operator visibility into
 * compute resources, data protection, and token economics.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NO_ERRORS_SCHEMA } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { ShefaService } from '@app/elohim/services/shefa.service';
import { PerformanceMetricsService } from '@app/elohim/services/performance-metrics.service';
import { CustodianCommitmentService } from '@app/elohim/services/custodian-commitment.service';
import { EconomicService } from '../../services/economic.service';
import { StewardedResourceService } from '../../services/stewarded-resources.service';
import { ComputeEventService } from '../../services/compute-event.service';
import { FamilyCommunityProtectionService } from '../../services/family-community-protection.service';
import { ShefaComputeService } from '../../services/shefa-compute.service';
import { ShefaDashboardComponent } from './shefa-dashboard.component';

describe('ShefaDashboardComponent', () => {
  let component: ShefaDashboardComponent;
  let fixture: ComponentFixture<ShefaDashboardComponent>;
  let shefaComputeMock: jasmine.SpyObj<ShefaComputeService>;
  let familyProtectionMock: jasmine.SpyObj<FamilyCommunityProtectionService>;
  let computeEventsMock: jasmine.SpyObj<ComputeEventService>;
  let holochainClientMock: jasmine.SpyObj<HolochainClientService>;
  let shefaServiceMock: jasmine.SpyObj<ShefaService>;
  let performanceMetricsMock: jasmine.SpyObj<PerformanceMetricsService>;
  let custodianCommitmentMock: jasmine.SpyObj<CustodianCommitmentService>;
  let economicServiceMock: jasmine.SpyObj<EconomicService>;
  let stewardedResourceMock: jasmine.SpyObj<StewardedResourceService>;

  beforeEach(async () => {
    // Reset to ensure clean test isolation
    TestBed.resetTestingModule();

    holochainClientMock = jasmine.createSpyObj('HolochainClientService', [
      'callZome',
      'isConnected',
    ]);
    holochainClientMock.isConnected.and.returnValue(true);

    shefaServiceMock = jasmine.createSpyObj('ShefaService', [
      'getComputeMetrics',
      'getTokenBalance',
    ]);

    performanceMetricsMock = jasmine.createSpyObj('PerformanceMetricsService', [
      'getMetrics',
      'recordResponseTime',
    ]);

    custodianCommitmentMock = jasmine.createSpyObj('CustodianCommitmentService', [
      'getCommitments',
      'createCommitment',
    ]);

    economicServiceMock = jasmine.createSpyObj('EconomicService', [
      'isAvailable',
      'getTokenBalance',
    ]);

    stewardedResourceMock = jasmine.createSpyObj('StewardedResourceService', [
      'getAllocatedResources',
    ]);

    shefaComputeMock = jasmine.createSpyObj('ShefaComputeService', [
      'initializeDashboard',
    ]);
    familyProtectionMock = jasmine.createSpyObj('FamilyCommunityProtectionService', [
      'initializeProtectionMonitoring',
    ]);
    computeEventsMock = jasmine.createSpyObj('ComputeEventService', [
      'initializeEventEmission',
    ]);

    await TestBed.configureTestingModule({
      imports: [ShefaDashboardComponent],
      providers: [
        // HttpClient providers must come first
        provideHttpClient(),
        provideHttpClientTesting(),
        // Override all services that have root-level providedIn
        { provide: HolochainClientService, useValue: holochainClientMock },
        { provide: ShefaService, useValue: shefaServiceMock },
        { provide: PerformanceMetricsService, useValue: performanceMetricsMock },
        { provide: CustodianCommitmentService, useValue: custodianCommitmentMock },
        { provide: EconomicService, useValue: economicServiceMock },
        { provide: StewardedResourceService, useValue: stewardedResourceMock },
        { provide: ShefaComputeService, useValue: shefaComputeMock },
        { provide: FamilyCommunityProtectionService, useValue: familyProtectionMock },
        { provide: ComputeEventService, useValue: computeEventsMock },
      ],
      schemas: [NO_ERRORS_SCHEMA],
      teardown: { destroyAfterEach: true },
    }).compileComponents();

    fixture = TestBed.createComponent(ShefaDashboardComponent);
    component = fixture.componentInstance;

    // Set required inputs before detectChanges
    component.operatorId = 'test-operator';
    component.stewardedResourceId = 'test-resource';

    // Don't call detectChanges in beforeEach to avoid initialization
  });

  describe('component creation', () => {
    it('should create', () => {
      // Component should be truthy after TestBed.createComponent
      // This verifies the component can be instantiated with mocked dependencies
      expect(component).toBeTruthy();
      expect(component.operatorId).toBe('test-operator');
      expect(component.stewardedResourceId).toBe('test-resource');
    });
  });

  describe('initialization', () => {
    it('should have operatorId input property', () => {
      expect(component.operatorId).toBeDefined();
    });

    it('should have stewardedResourceId input property', () => {
      expect(component.stewardedResourceId).toBeDefined();
    });

    it('should have config input property', () => {
      expect(component.config).toBeDefined();
    });

    it('should initialize with default config', () => {
      expect(component.mergedConfig).toBeDefined();
      expect(component.mergedConfig.displayMode).toBe('detailed');
    });
  });

  describe('UI state', () => {
    it('should start with loading state', () => {
      expect(component.isLoading).toBeTrue();
    });

    it('should have null lastUpdateTime initially', () => {
      expect(component.lastUpdateTime).toBeNull();
    });

    it('should have default selected panel', () => {
      expect(component.selectedPanel).toBe('compute');
    });

    it('should have config panel hidden initially', () => {
      expect(component.showConfigPanel).toBeFalse();
    });
  });

  describe('getStatusClass', () => {
    it('should have getStatusClass method', () => {
      expect(component.getStatusClass).toBeDefined();
      expect(typeof component.getStatusClass).toBe('function');
    });

    it('should return status-unknown when no current state', () => {
      component.currentState = null;
      expect(component.getStatusClass()).toBe('status-unknown');
    });
  });

  describe('getStatusText', () => {
    it('should have getStatusText method', () => {
      expect(component.getStatusText).toBeDefined();
      expect(typeof component.getStatusText).toBe('function');
    });

    it('should return UNKNOWN when no current state', () => {
      component.currentState = null;
      expect(component.getStatusText()).toBe('UNKNOWN');
    });
  });

  describe('getUptimePercent', () => {
    it('should have getUptimePercent method', () => {
      expect(component.getUptimePercent).toBeDefined();
      expect(typeof component.getUptimePercent).toBe('function');
    });

    it('should return 0 when no current state', () => {
      component.currentState = null;
      expect(component.getUptimePercent()).toBe(0);
    });
  });

  describe('getReliabilityLabel', () => {
    it('should have getReliabilityLabel method', () => {
      expect(component.getReliabilityLabel).toBeDefined();
      expect(typeof component.getReliabilityLabel).toBe('function');
    });

    it('should return Unknown when no current state', () => {
      component.currentState = null;
      expect(component.getReliabilityLabel()).toBe('Unknown');
    });
  });

  describe('getNodeLocation', () => {
    it('should have getNodeLocation method', () => {
      expect(component.getNodeLocation).toBeDefined();
      expect(typeof component.getNodeLocation).toBe('function');
    });

    it('should return Location unknown when no current state', () => {
      component.currentState = null;
      expect(component.getNodeLocation()).toBe('Location unknown');
    });
  });

  describe('panel visibility checks', () => {
    it('should have isComputeMetricsVisible method', () => {
      expect(component.isComputeMetricsVisible).toBeDefined();
      expect(typeof component.isComputeMetricsVisible).toBe('function');
    });

    it('should have isFamilyProtectionVisible method', () => {
      expect(component.isFamilyProtectionVisible).toBeDefined();
      expect(typeof component.isFamilyProtectionVisible).toBe('function');
    });

    it('should have isTokenEarningsVisible method', () => {
      expect(component.isTokenEarningsVisible).toBeDefined();
      expect(typeof component.isTokenEarningsVisible).toBe('function');
    });

    it('should have isEconomicEventsVisible method', () => {
      expect(component.isEconomicEventsVisible).toBeDefined();
      expect(typeof component.isEconomicEventsVisible).toBe('function');
    });

    it('should have isConstitutionalLimitsVisible method', () => {
      expect(component.isConstitutionalLimitsVisible).toBeDefined();
      expect(typeof component.isConstitutionalLimitsVisible).toBe('function');
    });
  });

  describe('selectPanel', () => {
    it('should have selectPanel method', () => {
      expect(component.selectPanel).toBeDefined();
      expect(typeof component.selectPanel).toBe('function');
    });

    it('should change selectedPanel', () => {
      component.selectPanel('protection');
      expect(component.selectedPanel).toBe('protection');
    });
  });

  describe('compute resource getters', () => {
    it('should have getCpuUsage method', () => {
      expect(component.getCpuUsage).toBeDefined();
      expect(typeof component.getCpuUsage).toBe('function');
    });

    it('should have getMemoryUsage method', () => {
      expect(component.getMemoryUsage).toBeDefined();
      expect(typeof component.getMemoryUsage).toBe('function');
    });

    it('should have getStorageUsage method', () => {
      expect(component.getStorageUsage).toBeDefined();
      expect(typeof component.getStorageUsage).toBe('function');
    });

    it('should return 0 for CPU when no state', () => {
      component.currentState = null;
      expect(component.getCpuUsage()).toBe(0);
    });

    it('should return 0 for memory when no state', () => {
      component.currentState = null;
      expect(component.getMemoryUsage()).toBe(0);
    });

    it('should return 0 for storage when no state', () => {
      component.currentState = null;
      expect(component.getStorageUsage()).toBe(0);
    });
  });

  describe('protection getters', () => {
    it('should have getTotalCustodians method', () => {
      expect(component.getTotalCustodians).toBeDefined();
      expect(typeof component.getTotalCustodians).toBe('function');
    });

    it('should have getProtectionLevelClass method', () => {
      expect(component.getProtectionLevelClass).toBeDefined();
      expect(typeof component.getProtectionLevelClass).toBe('function');
    });

    it('should return 0 for custodians when no state', () => {
      component.currentState = null;
      expect(component.getTotalCustodians()).toBe(0);
    });

    it('should return protection-unknown when no state', () => {
      component.currentState = null;
      expect(component.getProtectionLevelClass()).toBe('protection-unknown');
    });
  });

  describe('token getters', () => {
    it('should have getTokenBalance method', () => {
      expect(component.getTokenBalance).toBeDefined();
      expect(typeof component.getTokenBalance).toBe('function');
    });

    it('should have getTokenEarningRate method', () => {
      expect(component.getTokenEarningRate).toBeDefined();
      expect(typeof component.getTokenEarningRate).toBe('function');
    });

    it('should have getEstimatedMonthlyEarnings method', () => {
      expect(component.getEstimatedMonthlyEarnings).toBeDefined();
      expect(typeof component.getEstimatedMonthlyEarnings).toBe('function');
    });

    it('should return 0 for balance when no state', () => {
      component.currentState = null;
      expect(component.getTokenBalance()).toBe(0);
    });
  });

  describe('constitutional limits getters', () => {
    it('should have getConstitutionalStatus method', () => {
      expect(component.getConstitutionalStatus).toBeDefined();
      expect(typeof component.getConstitutionalStatus).toBe('function');
    });

    it('should have getAlertCount method', () => {
      expect(component.getAlertCount).toBeDefined();
      expect(typeof component.getAlertCount).toBe('function');
    });

    it('should have getCriticalAlerts method', () => {
      expect(component.getCriticalAlerts).toBeDefined();
      expect(typeof component.getCriticalAlerts).toBe('function');
    });

    it('should return Unknown when no state', () => {
      component.currentState = null;
      expect(component.getConstitutionalStatus()).toBe('Unknown');
    });

    it('should return 0 for alert count when no state', () => {
      component.currentState = null;
      expect(component.getAlertCount()).toBe(0);
    });
  });

  describe('utility methods', () => {
    it('should have getTimeSinceUpdate method', () => {
      expect(component.getTimeSinceUpdate).toBeDefined();
      expect(typeof component.getTimeSinceUpdate).toBe('function');
    });

    it('should return Never when no update time', () => {
      component.lastUpdateTime = null;
      expect(component.getTimeSinceUpdate()).toBe('Never');
    });

    it('should have toggleConfigPanel method', () => {
      expect(component.toggleConfigPanel).toBeDefined();
      expect(typeof component.toggleConfigPanel).toBe('function');
    });

    it('should toggle config panel visibility', () => {
      const initial = component.showConfigPanel;
      component.toggleConfigPanel();
      expect(component.showConfigPanel).toBe(!initial);
    });

    it('should have setDisplayMode method', () => {
      expect(component.setDisplayMode).toBeDefined();
      expect(typeof component.setDisplayMode).toBe('function');
    });

    it('should have togglePanelVisibility method', () => {
      expect(component.togglePanelVisibility).toBeDefined();
      expect(typeof component.togglePanelVisibility).toBe('function');
    });

    it('should have exportDashboard method', () => {
      expect(component.exportDashboard).toBeDefined();
      expect(typeof component.exportDashboard).toBe('function');
    });
  });

  describe('lifecycle', () => {
    it('should have ngOnInit method', () => {
      expect(component.ngOnInit).toBeDefined();
      expect(typeof component.ngOnInit).toBe('function');
    });

    it('should have ngOnDestroy method', () => {
      expect(component.ngOnDestroy).toBeDefined();
      expect(typeof component.ngOnDestroy).toBe('function');
    });
  });
});
