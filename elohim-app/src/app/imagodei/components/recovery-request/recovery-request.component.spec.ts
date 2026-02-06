/**
 * RecoveryRequestComponent Tests
 *
 * Tests for identity recovery request flow.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RecoveryRequestComponent } from './recovery-request.component';
import { RecoveryCoordinatorService } from '../../services/recovery-coordinator.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { Router } from '@angular/router';
import { signal } from '@angular/core';

describe('RecoveryRequestComponent', () => {
  let component: RecoveryRequestComponent;
  let fixture: ComponentFixture<RecoveryRequestComponent>;
  let mockRecoveryService: jasmine.SpyObj<RecoveryCoordinatorService>;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let mockRouter: jasmine.SpyObj<Router>;

  beforeEach(async () => {
    // Create mocks
    mockRecoveryService = jasmine.createSpyObj(
      'RecoveryCoordinatorService',
      ['initiateRecovery', 'checkProgress', 'completeRecovery'],
      {
        activeRequest: signal(null),
        progress: signal(null),
        credential: signal(null),
        isLoading: signal(false),
        error: signal(null),
      }
    );

    mockDoorwayRegistry = jasmine.createSpyObj(
      'DoorwayRegistryService',
      [],
      {
        hasSelection: signal(false),
      }
    );

    mockRouter = jasmine.createSpyObj('Router', ['navigate']);

    await TestBed.configureTestingModule({
      imports: [RecoveryRequestComponent],
      providers: [
        { provide: RecoveryCoordinatorService, useValue: mockRecoveryService },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: Router, useValue: mockRouter },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(RecoveryRequestComponent);
    component = fixture.componentInstance;
  });

  // ==========================================================================
  // Component Creation
  // ==========================================================================

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Delegated Signals
  // ==========================================================================

  it('should delegate activeRequest from RecoveryCoordinatorService', () => {
    expect(component.activeRequest).toBeDefined();
  });

  it('should delegate progress from RecoveryCoordinatorService', () => {
    expect(component.progress).toBeDefined();
  });

  it('should delegate credential from RecoveryCoordinatorService', () => {
    expect(component.credential).toBeDefined();
  });

  it('should delegate isLoading from RecoveryCoordinatorService', () => {
    expect(component.isLoading).toBeDefined();
  });

  it('should delegate error from RecoveryCoordinatorService', () => {
    expect(component.error).toBeDefined();
  });

  it('should delegate hasDoorway from DoorwayRegistryService', () => {
    expect(component.hasDoorway).toBeDefined();
  });

  // ==========================================================================
  // Signals
  // ==========================================================================

  it('should have currentStep signal', () => {
    expect(component.currentStep).toBeDefined();
  });

  // ==========================================================================
  // Form Data
  // ==========================================================================

  it('should initialize form data', () => {
    expect(component.claimedIdentity).toBe('');
    expect(component.additionalContext).toBe('');
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have initiateRecovery method', () => {
    expect(component.initiateRecovery).toBeDefined();
    expect(typeof component.initiateRecovery).toBe('function');
  });

  it('should have completeRecovery method', () => {
    expect(component.completeRecovery).toBeDefined();
    expect(typeof component.completeRecovery).toBe('function');
  });

  it('should have selectDoorway method', () => {
    expect(component.selectDoorway).toBeDefined();
    expect(typeof component.selectDoorway).toBe('function');
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  it('should start with claim step', () => {
    expect(component.currentStep()).toBe('claim');
  });


  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  it('should implement OnInit', () => {
    expect(component.ngOnInit).toBeDefined();
    expect(typeof component.ngOnInit).toBe('function');
  });

  it('should implement OnDestroy', () => {
    expect(component.ngOnDestroy).toBeDefined();
    expect(typeof component.ngOnDestroy).toBe('function');
  });

  it('should not throw when ngOnDestroy called', () => {
    expect(() => {
      component.ngOnDestroy();
    }).not.toThrow();
  });
});
