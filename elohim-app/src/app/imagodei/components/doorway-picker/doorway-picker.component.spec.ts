/**
 * DoorwayPickerComponent Tests
 *
 * Tests for gateway selection UI component.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { DoorwayPickerComponent, type DoorwayPickerMode } from './doorway-picker.component';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';
import { signal } from '@angular/core';

describe('DoorwayPickerComponent', () => {
  let component: DoorwayPickerComponent;
  let fixture: ComponentFixture<DoorwayPickerComponent>;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let mockOAuthProvider: jasmine.SpyObj<OAuthAuthProvider>;

  beforeEach(async () => {
    // Create mocks
    mockDoorwayRegistry = jasmine.createSpyObj(
      'DoorwayRegistryService',
      ['loadDoorways', 'refreshHealth', 'selectDoorway', 'validateDoorway'],
      {
        doorwaysWithHealth: signal([]),
        isLoading: signal(false),
        error: signal(null),
        selected: signal(null),
      }
    );

    mockOAuthProvider = jasmine.createSpyObj('OAuthAuthProvider', ['initiateLogin']);

    await TestBed.configureTestingModule({
      imports: [DoorwayPickerComponent],
      providers: [
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: OAuthAuthProvider, useValue: mockOAuthProvider },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(DoorwayPickerComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  // ==========================================================================
  // Component Creation
  // ==========================================================================

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Inputs
  // ==========================================================================

  it('should have mode input', () => {
    expect(component.mode).toBeDefined();
  });

  it('should default mode to register', () => {
    expect(component.mode()).toBe('register');
  });

  // ==========================================================================
  // Outputs
  // ==========================================================================

  it('should have doorwaySelected output', () => {
    expect(component.doorwaySelected).toBeDefined();
  });

  it('should have cancelled output', () => {
    expect(component.cancelled).toBeDefined();
  });

  // ==========================================================================
  // Signals
  // ==========================================================================

  it('should have searchQuery signal', () => {
    expect(component.searchQuery).toBeDefined();
  });

  it('should have selectedRegion signal', () => {
    expect(component.selectedRegion).toBeDefined();
  });

  it('should have showCustomInput signal', () => {
    expect(component.showCustomInput).toBeDefined();
  });

  it('should have customUrl signal', () => {
    expect(component.customUrl).toBeDefined();
  });

  it('should have customValidating signal', () => {
    expect(component.customValidating).toBeDefined();
  });

  it('should have customError signal', () => {
    expect(component.customError).toBeDefined();
  });

  it('should have sortBy signal', () => {
    expect(component.sortBy).toBeDefined();
  });

  // ==========================================================================
  // Delegated Signals
  // ==========================================================================

  it('should delegate doorways from DoorwayRegistryService', () => {
    expect(component.doorways).toBeDefined();
  });

  it('should delegate isLoading from DoorwayRegistryService', () => {
    expect(component.isLoading).toBeDefined();
  });

  it('should delegate error from DoorwayRegistryService', () => {
    expect(component.error).toBeDefined();
  });

  it('should delegate selected from DoorwayRegistryService', () => {
    expect(component.selected).toBeDefined();
  });

  // ==========================================================================
  // Computed Signals
  // ==========================================================================

  it('should have recommendedDoorway computed signal', () => {
    expect(component.recommendedDoorway).toBeDefined();
  });

  it('should have filteredDoorways computed signal', () => {
    expect(component.filteredDoorways).toBeDefined();
  });

  it('should have availableRegions computed signal', () => {
    expect(component.availableRegions).toBeDefined();
  });

  it('should have selectedId computed signal', () => {
    expect(component.selectedId).toBeDefined();
  });

  it('should have titleText computed signal', () => {
    expect(component.titleText).toBeDefined();
  });

  it('should have subtitleText computed signal', () => {
    expect(component.subtitleText).toBeDefined();
  });

  it('should have actionText computed signal', () => {
    expect(component.actionText).toBeDefined();
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have loadDoorways method', () => {
    expect(component.loadDoorways).toBeDefined();
    expect(typeof component.loadDoorways).toBe('function');
  });

  it('should have selectDoorway method', () => {
    expect(component.selectDoorway).toBeDefined();
    expect(typeof component.selectDoorway).toBe('function');
  });

  it('should have toggleCustomInput method', () => {
    expect(component.toggleCustomInput).toBeDefined();
    expect(typeof component.toggleCustomInput).toBe('function');
  });

  it('should have validateAndSelectCustom method', () => {
    expect(component.validateAndSelectCustom).toBeDefined();
    expect(typeof component.validateAndSelectCustom).toBe('function');
  });

  it('should have cancel method', () => {
    expect(component.cancel).toBeDefined();
    expect(typeof component.cancel).toBe('function');
  });

  it('should have getLatencyClass method', () => {
    expect(component.getLatencyClass).toBeDefined();
    expect(typeof component.getLatencyClass).toBe('function');
  });

  it('should have formatLatency method', () => {
    expect(component.formatLatency).toBeDefined();
    expect(typeof component.formatLatency).toBe('function');
  });

  it('should have getLatencyBarWidth method', () => {
    expect(component.getLatencyBarWidth).toBeDefined();
    expect(typeof component.getLatencyBarWidth).toBe('function');
  });

  it('should have isRecommended method', () => {
    expect(component.isRecommended).toBeDefined();
    expect(typeof component.isRecommended).toBe('function');
  });

  it('should have selectRecommended method', () => {
    expect(component.selectRecommended).toBeDefined();
    expect(typeof component.selectRecommended).toBe('function');
  });

  it('should have trackByDoorway method', () => {
    expect(component.trackByDoorway).toBeDefined();
    expect(typeof component.trackByDoorway).toBe('function');
  });

  // ==========================================================================
  // Toggle Custom Input
  // ==========================================================================

  it('should toggle custom input visibility', () => {
    expect(component.showCustomInput()).toBe(false);
    component.toggleCustomInput();
    expect(component.showCustomInput()).toBe(true);
    component.toggleCustomInput();
    expect(component.showCustomInput()).toBe(false);
  });

  it('should clear custom error when toggling input', () => {
    component.customError.set('Some error');
    component.toggleCustomInput();
    expect(component.customError()).toBeNull();
  });

  it('should clear custom url when toggling input', () => {
    component.customUrl.set('http://example.com');
    component.toggleCustomInput();
    expect(component.customUrl()).toBe('');
  });

  // ==========================================================================
  // Cancel
  // ==========================================================================

  it('should emit cancelled when cancel is called', (done) => {
    component.cancelled.subscribe(() => {
      done();
    });
    component.cancel();
  });

  // ==========================================================================
  // Text Displays Based on Mode
  // ==========================================================================

  it('should return register action text in register mode', () => {
    const text = component.actionText();
    expect(text).toContain('Join');
  });

  // ==========================================================================
  // Latency Formatting
  // ==========================================================================

  it('should format latency in milliseconds', () => {
    const formatted = component.formatLatency(150);
    expect(formatted).toBe('150ms');
  });

  it('should show -- for null latency', () => {
    const formatted = component.formatLatency(null);
    expect(formatted).toBe('--');
  });

  // ==========================================================================
  // Latency Class
  // ==========================================================================

  it('should return fast class for low latency', () => {
    const cssClass = component.getLatencyClass(50);
    expect(cssClass).toBe('latency-fast');
  });

  it('should return medium class for medium latency', () => {
    const cssClass = component.getLatencyClass(200);
    expect(cssClass).toBe('latency-medium');
  });

  it('should return slow class for high latency', () => {
    const cssClass = component.getLatencyClass(400);
    expect(cssClass).toBe('latency-slow');
  });

  it('should return unknown class for null latency', () => {
    const cssClass = component.getLatencyClass(null);
    expect(cssClass).toBe('latency-unknown');
  });

  // ==========================================================================
  // Latency Bar Width
  // ==========================================================================

  it('should return 0 for null latency', () => {
    const width = component.getLatencyBarWidth(null);
    expect(width).toBe(0);
  });

  it('should calculate bar width for latency', () => {
    const width = component.getLatencyBarWidth(250);
    expect(width).toBeGreaterThan(0);
    expect(width).toBeLessThanOrEqual(100);
  });

  // ==========================================================================
  // Track By
  // ==========================================================================

  it('should track doorway by id', () => {
    const doorway = { id: 'test-doorway', name: 'Test' } as any;
    const id = component.trackByDoorway(0, doorway);
    expect(id).toBe('test-doorway');
  });
});
