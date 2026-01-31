/**
 * ConnectionIndicatorComponent Tests
 *
 * Tests for network connection status display component.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ConnectionIndicatorComponent, type ConnectionStatus } from './connection-indicator.component';
import { IdentityService } from '../../services/identity.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { signal } from '@angular/core';

describe('ConnectionIndicatorComponent', () => {
  let component: ConnectionIndicatorComponent;
  let fixture: ComponentFixture<ConnectionIndicatorComponent>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let mockHolochainService: jasmine.SpyObj<HolochainClientService>;

  beforeEach(async () => {
    // Create mocks
    mockIdentityService = jasmine.createSpyObj(
      'IdentityService',
      [],
      {
        mode: signal('session'),
      }
    );

    mockDoorwayRegistry = jasmine.createSpyObj(
      'DoorwayRegistryService',
      [],
      {
        selected: signal(null),
      }
    );

    mockHolochainService = jasmine.createSpyObj(
      'HolochainClientService',
      [],
      {
        state: signal('disconnected'),
      }
    );

    await TestBed.configureTestingModule({
      imports: [ConnectionIndicatorComponent],
      providers: [
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: HolochainClientService, useValue: mockHolochainService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ConnectionIndicatorComponent);
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
  // Signal Injections
  // ==========================================================================

  it('should have status computed signal', () => {
    expect(component.status).toBeDefined();
  });

  it('should have isVisible computed signal', () => {
    expect(component.isVisible).toBeDefined();
  });

  // ==========================================================================
  // State
  // ==========================================================================

  it('should initialize expanded as false', () => {
    expect(component.expanded).toBe(false);
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have toggleExpanded method', () => {
    expect(component.toggleExpanded).toBeDefined();
    expect(typeof component.toggleExpanded).toBe('function');
  });

  // ==========================================================================
  // Toggle Expanded
  // ==========================================================================

  it('should toggle expanded state when toggleExpanded is called', () => {
    expect(component.expanded).toBe(false);
    component.toggleExpanded();
    expect(component.expanded).toBe(true);
    component.toggleExpanded();
    expect(component.expanded).toBe(false);
  });

  // ==========================================================================
  // Status Computation - Session Mode
  // ==========================================================================

  it('should return session status when mode is session', () => {
    const status = component.status();
    expect(status).toBeDefined();
    expect(status.label).toBe('Human Session');
    expect(status.icon).toBe('face');
  });

  // ==========================================================================
  // Status Computation - Visibility
  // ==========================================================================

  it('should be visible when mode is session', () => {
    expect(component.isVisible()).toBe(true);
  });

  it('should not be visible when mode is anonymous', () => {
    Object.defineProperty(mockIdentityService, 'mode', {
      value: signal('anonymous'),
      writable: true,
      configurable: true,
    });

    const newFixture = TestBed.createComponent(ConnectionIndicatorComponent);
    const newComponent = newFixture.componentInstance;

    expect(newComponent.isVisible()).toBe(false);
  });

  // ==========================================================================
  // ConnectionStatus Interface
  // ==========================================================================

  it('status should have required ConnectionStatus properties', () => {
    const status = component.status();
    expect(status.mode).toBeDefined();
    expect(status.label).toBeDefined();
    expect(status.icon).toBeDefined();
    expect(status.color).toBeDefined();
    expect(status.cssClass).toBeDefined();
  });
});
