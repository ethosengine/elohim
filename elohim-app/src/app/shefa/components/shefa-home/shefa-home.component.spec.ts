/**
 * Shefa Home Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { provideRouter } from '@angular/router';
import { of } from 'rxjs';

import { ShefaHomeComponent } from './shefa-home.component';
import { AppreciationService } from '../../services/appreciation.service';
import { EconomicService } from '../../services/economic.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

describe('ShefaHomeComponent', () => {
  let component: ShefaHomeComponent;
  let fixture: ComponentFixture<ShefaHomeComponent>;
  let mockEconomicService: jasmine.SpyObj<EconomicService>;
  let mockAppreciationService: jasmine.SpyObj<AppreciationService>;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;

  beforeEach(async () => {
    mockEconomicService = jasmine.createSpyObj('EconomicService', [
      'testAvailability',
      'isAvailable',
      'getEventsForAgent',
    ]);
    mockEconomicService.testAvailability.and.returnValue(Promise.resolve(true));
    mockEconomicService.isAvailable.and.returnValue(false);
    mockEconomicService.getEventsForAgent.and.returnValue(of([]));

    mockAppreciationService = jasmine.createSpyObj('AppreciationService', [
      'testAvailability',
      'isAvailable',
      'getAppreciationsFor',
    ]);
    mockAppreciationService.testAvailability.and.returnValue(Promise.resolve(true));
    mockAppreciationService.isAvailable.and.returnValue(false);
    mockAppreciationService.getAppreciationsFor.and.returnValue(of([]));

    mockHolochainClient = jasmine.createSpyObj(
      'HolochainClientService',
      ['testAdminConnection'],
      {
        isConnected: jasmine.createSpy('isConnected').and.returnValue(false),
      }
    );
    mockHolochainClient.testAdminConnection.and.returnValue(
      Promise.resolve({ success: false })
    );

    await TestBed.configureTestingModule({
      imports: [ShefaHomeComponent],
      providers: [
        provideRouter([]),
        { provide: EconomicService, useValue: mockEconomicService },
        { provide: AppreciationService, useValue: mockAppreciationService },
        { provide: HolochainClientService, useValue: mockHolochainClient },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ShefaHomeComponent);
    component = fixture.componentInstance;
    // Don't call detectChanges here - let individual tests control it
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Component Initialization
  // ==========================================================================

  describe('Initialization', () => {
    it('should load data on init', async () => {
      fixture.detectChanges(); // Triggers ngOnInit
      await fixture.whenStable();

      expect(mockEconomicService.testAvailability).toHaveBeenCalled();
      expect(mockAppreciationService.testAvailability).toHaveBeenCalled();
    });

    it('should set loading state during init', () => {
      expect(component.loading()).toBe(true);
    });

    it('should initialize with empty events array', () => {
      expect(component.events()).toEqual([]);
    });

    it('should initialize with empty appreciations array', () => {
      expect(component.appreciations()).toEqual([]);
    });
  });

  // ==========================================================================
  // Computed Stats
  // ==========================================================================

  describe('Computed Stats', () => {
    it('should calculate total events count', () => {
      component.events.set([
        { id: '1', action: 'use', provider: 'a', receiver: 'b', hasPointInTime: '', state: 'validated' },
        { id: '2', action: 'produce', provider: 'c', receiver: 'd', hasPointInTime: '', state: 'validated' },
      ] as any);

      expect(component.totalEvents()).toBe(2);
    });

    it('should calculate total appreciations count', () => {
      component.appreciations.set([
        { id: '1', appreciationOf: 'x', appreciatedBy: 'a', appreciationTo: 'b', quantityValue: 10, quantityUnit: 'points', note: null, createdAt: '' },
        { id: '2', appreciationOf: 'y', appreciatedBy: 'c', appreciationTo: 'd', quantityValue: 20, quantityUnit: 'points', note: null, createdAt: '' },
      ]);

      expect(component.totalAppreciations()).toBe(2);
    });

    it('should calculate unique agents from events and appreciations', () => {
      component.events.set([
        { id: '1', action: 'use', provider: 'agent-1', receiver: 'agent-2', hasPointInTime: '', state: 'validated' },
        { id: '2', action: 'produce', provider: 'agent-2', receiver: 'agent-3', hasPointInTime: '', state: 'validated' },
      ] as any);

      component.appreciations.set([
        { id: '1', appreciationOf: 'x', appreciatedBy: 'agent-1', appreciationTo: 'agent-4', quantityValue: 10, quantityUnit: 'points', note: null, createdAt: '' },
      ]);

      expect(component.uniqueAgents()).toBe(4);
    });

    it('should calculate total recognition points', () => {
      component.appreciations.set([
        { id: '1', appreciationOf: 'x', appreciatedBy: 'a', appreciationTo: 'b', quantityValue: 10, quantityUnit: 'points', note: null, createdAt: '' },
        { id: '2', appreciationOf: 'y', appreciatedBy: 'c', appreciationTo: 'd', quantityValue: 25, quantityUnit: 'points', note: null, createdAt: '' },
        { id: '3', appreciationOf: 'z', appreciatedBy: 'e', appreciationTo: 'f', quantityValue: 15, quantityUnit: 'points', note: null, createdAt: '' },
      ]);

      expect(component.totalRecognition()).toBe(50);
    });

    it('should handle zero recognition points', () => {
      component.appreciations.set([]);
      expect(component.totalRecognition()).toBe(0);
    });
  });

  // ==========================================================================
  // Data Loading - Success Cases
  // ==========================================================================

  describe('Data Loading - Success', () => {
    beforeEach(() => {
      mockEconomicService.testAvailability.and.returnValue(Promise.resolve(true));
      mockEconomicService.isAvailable.and.returnValue(true);
      mockEconomicService.getEventsForAgent.and.returnValue(of([
        { id: 'event-1', action: 'use', provider: 'p1', receiver: 'r1', hasPointInTime: new Date().toISOString(), state: 'validated' },
      ] as any));

      mockAppreciationService.testAvailability.and.returnValue(Promise.resolve(true));
      mockAppreciationService.isAvailable.and.returnValue(true);
      mockAppreciationService.getAppreciationsFor.and.returnValue(of([
        { id: 'app-1', appreciationOf: 'content', appreciatedBy: 'user1', appreciationTo: 'user2', quantityValue: 10, quantityUnit: 'points', note: null, createdAt: new Date().toISOString() },
      ]));
    });

    it('should load events when service is available', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.events().length).toBeGreaterThan(0);
    });

    it('should load appreciations when service is available', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.appreciations().length).toBeGreaterThan(0);
    });

    it('should set loading to false after data loads', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.loading()).toBe(false);
    });

    it('should not set error on successful load', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.error()).toBeNull();
    });
  });

  // ==========================================================================
  // Data Loading - Error Cases
  // ==========================================================================

  describe('Data Loading - Errors', () => {
    it('should load demo data when not connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);
      mockEconomicService.isAvailable.and.returnValue(false);
      mockAppreciationService.isAvailable.and.returnValue(false);

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.events().length).toBeGreaterThan(0);
      expect(component.appreciations().length).toBeGreaterThan(0);
    });

    it('should show error message when connection fails', async () => {
      mockEconomicService.testAvailability.and.returnValue(Promise.reject(new Error('Connection failed')));

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.error()).toContain('Failed to connect');
    });

    it('should set loading to false even on error', async () => {
      mockEconomicService.testAvailability.and.returnValue(Promise.reject(new Error('Error')));

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.loading()).toBe(false);
    });
  });

  // ==========================================================================
  // Demo Data
  // ==========================================================================

  describe('Demo Data Loading', () => {
    it('should load demo events', () => {
      component.loadDemoData();

      expect(component.events().length).toBeGreaterThan(0);
      expect(component.events()[0].id).toBeDefined();
      expect(component.events()[0].action).toBeDefined();
    });

    it('should load demo appreciations', () => {
      component.loadDemoData();

      expect(component.appreciations().length).toBeGreaterThan(0);
      expect(component.appreciations()[0].quantityValue).toBeDefined();
    });

    it('should create valid demo economic events', () => {
      component.loadDemoData();

      const event = component.events()[0];
      expect(event.id).toBeTruthy();
      expect(event.provider).toBeTruthy();
      expect(event.receiver).toBeTruthy();
      expect(event.hasPointInTime).toBeTruthy();
    });
  });

  // ==========================================================================
  // User Actions
  // ==========================================================================

  describe('User Actions', () => {
    it('should refresh data when refreshData is called', async () => {
      mockEconomicService.testAvailability.calls.reset();
      mockAppreciationService.testAvailability.calls.reset();

      await component.refreshData();

      expect(mockEconomicService.testAvailability).toHaveBeenCalled();
      expect(mockAppreciationService.testAvailability).toHaveBeenCalled();
    });

    it('should test connection when testConnection is called', async () => {
      mockHolochainClient.testAdminConnection.and.returnValue(Promise.resolve({ success: true }));

      await component.testConnection();

      expect(mockHolochainClient.testAdminConnection).toHaveBeenCalled();
    });

    it('should show error when connection test fails', async () => {
      mockHolochainClient.testAdminConnection.and.returnValue(Promise.resolve({ success: false }));

      await component.testConnection();

      expect(component.error()).toContain('Could not connect');
    });

    it('should handle exception in testConnection', async () => {
      mockHolochainClient.testAdminConnection.and.returnValue(Promise.reject(new Error('Network error')));

      await component.testConnection();

      expect(component.error()).toBe('Connection test failed');
    });

    it('should dismiss error when dismissError is called', () => {
      component.error.set('Test error');
      component.dismissError();

      expect(component.error()).toBeNull();
    });
  });

  // ==========================================================================
  // Formatting Helpers
  // ==========================================================================

  describe('Formatting Helpers', () => {
    describe('formatAction', () => {
      it('should format "use" action', () => {
        expect(component.formatAction('use')).toBe('Used Resource');
      });

      it('should format "produce" action', () => {
        expect(component.formatAction('produce')).toBe('Produced');
      });

      it('should format "raise" action', () => {
        expect(component.formatAction('raise')).toBe('Recognition');
      });

      it('should return original action for unknown actions', () => {
        expect(component.formatAction('unknown')).toBe('unknown');
      });
    });

    describe('getActionIcon', () => {
      it('should return icon for "use"', () => {
        expect(component.getActionIcon('use')).toBe('👁');
      });

      it('should return icon for "produce"', () => {
        expect(component.getActionIcon('produce')).toBe('✨');
      });

      it('should return default icon for unknown action', () => {
        expect(component.getActionIcon('unknown')).toBe('●');
      });
    });

    describe('getActionClass', () => {
      it('should replace hyphens with underscores', () => {
        expect(component.getActionClass('deliver-service')).toBe('deliver_service');
      });

      it('should handle action without hyphens', () => {
        expect(component.getActionClass('use')).toBe('use');
      });
    });

    describe('shortenId', () => {
      it('should return "Unknown" for empty string', () => {
        expect(component.shortenId('')).toBe('Unknown');
      });

      it('should return full ID if length <= 16', () => {
        expect(component.shortenId('short-id')).toBe('short-id');
      });

      it('should shorten long IDs', () => {
        const longId = 'very-long-agent-id-12345678901234567890';
        const shortened = component.shortenId(longId);

        expect(shortened.length).toBeLessThan(longId.length);
        expect(shortened).toContain('...');
        expect(shortened).toContain(longId.slice(0, 8));
      });
    });

    describe('formatTime', () => {
      it('should return "Just now" for recent time', () => {
        const now = new Date().toISOString();
        expect(component.formatTime(now)).toBe('Just now');
      });

      it('should return minutes ago for times < 1 hour', () => {
        const tenMinutesAgo = new Date(Date.now() - 10 * 60 * 1000).toISOString();
        const result = component.formatTime(tenMinutesAgo);

        expect(result).toContain('m ago');
      });

      it('should return hours ago for times < 1 day', () => {
        const twoHoursAgo = new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString();
        const result = component.formatTime(twoHoursAgo);

        expect(result).toContain('h ago');
      });

      it('should return date for times > 1 day', () => {
        const twoDaysAgo = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString();
        const result = component.formatTime(twoDaysAgo);

        expect(result).not.toContain('ago');
      });

      it('should return "Unknown" for invalid date string', () => {
        expect(component.formatTime('invalid-date')).toBe('Unknown');
      });
    });
  });

  // ==========================================================================
  // Template Rendering
  // ==========================================================================

  describe('Template Rendering', () => {
    it('should show loading state initially', () => {
      // Component starts with loading = true
      fixture.detectChanges();
      const compiled = fixture.nativeElement;
      const loadingOverlay = compiled.querySelector('.loading-overlay');

      expect(loadingOverlay).toBeTruthy();
    });

    it('should show connection status', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const statusElement = fixture.nativeElement.querySelector('.connection-status');
      expect(statusElement.textContent).toContain('Connected');
    });

    it('should show disconnected status', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);
      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const statusElement = fixture.nativeElement.querySelector('.connection-status');
      expect(statusElement.textContent).toContain('Disconnected');
    });

    it('should display stats grid when not loading', async () => {
      // Manually set loading to false without triggering ngOnInit
      component.loading.set(false);
      // Initial render
      fixture.detectChanges();
      await fixture.whenStable();
      // Second render to ensure template updates
      fixture.detectChanges();

      const statsGrid = fixture.nativeElement.querySelector('.stats-grid');
      expect(statsGrid).toBeTruthy();
    });

    it('should display action buttons', async () => {
      component.loading.set(false);
      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const refreshButton = fixture.nativeElement.querySelector('.action-btn.primary');
      expect(refreshButton).toBeTruthy();
      expect(refreshButton.textContent).toContain('Refresh Data');
    });

    it('should show error banner when error is set', async () => {
      // First render the component with initialization
      fixture.detectChanges();
      await fixture.whenStable();

      // Now set the error state
      component.loading.set(false);
      component.error.set('Test error message');

      // Trigger change detection to reflect the new state
      fixture.detectChanges();
      await fixture.whenStable();

      const errorBanner = fixture.debugElement.query(By.css('.error-banner'));
      expect(errorBanner).toBeTruthy();
      expect(errorBanner.nativeElement.textContent).toContain('Test error message');
    });

    it('should call refreshData when refresh button is clicked', async () => {
      const refreshSpy = spyOn(component, 'refreshData');
      component.loading.set(false);
      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const refreshButton = fixture.nativeElement.querySelector('.action-btn.primary');
      expect(refreshButton).toBeTruthy();
      refreshButton.click();

      expect(refreshSpy).toHaveBeenCalled();
    });
  });
});
