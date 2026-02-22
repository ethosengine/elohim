/**
 * ConnectionIndicatorComponent Tests
 *
 * Tests for network connection status display component.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import {
  ConnectionIndicatorComponent,
  type ConnectionStatus,
} from './connection-indicator.component';
import { IdentityService } from '../../services/identity.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { TauriAuthService } from '../../services/tauri-auth.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { StorageClientService } from '@app/elohim/services/storage-client.service';
import { signal, WritableSignal } from '@angular/core';

/** Helper to create a fresh component with custom signal values */
function createComponent(overrides: {
  mode?: string;
  holochainState?: string;
  selectedDoorway?: unknown;
  graduationStatus?: string;
  peerCount?: number;
}): { fixture: ComponentFixture<ConnectionIndicatorComponent>; component: ConnectionIndicatorComponent } {
  const modeSignal = signal(overrides.mode ?? 'session');
  const stateSignal = signal(overrides.holochainState ?? 'disconnected');
  const selectedSignal = signal(overrides.selectedDoorway ?? null);
  const graduationSignal = signal(overrides.graduationStatus ?? 'idle');

  TestBed.resetTestingModule();
  TestBed.configureTestingModule({
    imports: [ConnectionIndicatorComponent],
    providers: [
      provideHttpClient(),
      provideHttpClientTesting(),
      {
        provide: IdentityService,
        useValue: { mode: modeSignal },
      },
      {
        provide: DoorwayRegistryService,
        useValue: { selected: selectedSignal, selectedUrl: signal(null) },
      },
      {
        provide: HolochainClientService,
        useValue: { state: stateSignal },
      },
      {
        provide: StorageClientService,
        useValue: {
          connectionMode: 'doorway',
          getStorageBaseUrl: () => 'http://localhost:8888',
        },
      },
      {
        provide: TauriAuthService,
        useValue: { graduationStatus: graduationSignal },
      },
    ],
  });

  const fixture = TestBed.createComponent(ConnectionIndicatorComponent);
  const component = fixture.componentInstance;

  if (overrides.peerCount !== undefined) {
    component.peerCount.set(overrides.peerCount);
  }

  fixture.detectChanges();
  return { fixture, component };
}

describe('ConnectionIndicatorComponent', () => {
  let component: ConnectionIndicatorComponent;
  let fixture: ComponentFixture<ConnectionIndicatorComponent>;

  beforeEach(async () => {
    ({ fixture, component } = createComponent({}));
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
    const { component: anonComponent } = createComponent({ mode: 'anonymous' });
    expect(anonComponent.isVisible()).toBe(false);
  });

  // ==========================================================================
  // Status Computation - Graduating State
  // ==========================================================================

  it('should return graduating status when graduation is confirming', () => {
    const { component: gradComponent } = createComponent({ graduationStatus: 'confirming' });
    const status = gradComponent.status();

    expect(status.mode).toBe('migrating');
    expect(status.label).toBe('Graduating...');
    expect(status.cssClass).toBe('status-graduating');
    expect(status.icon).toBe('swap_horiz');
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

  // ==========================================================================
  // Hosted + Error States (root bug fix)
  // ==========================================================================

  it('should show error with doorway name when hosted + error + doorway selected', () => {
    const { component: c } = createComponent({
      mode: 'hosted',
      holochainState: 'error',
      selectedDoorway: { doorway: { name: 'Alpha' } },
    });
    const status = c.status();

    expect(status.mode).toBe('offline');
    expect(status.label).toBe('Alpha (Error)');
    expect(status.icon).toBe('cloud_off');
    expect(status.color).toBe('#ef4444');
    expect(status.cssClass).toBe('status-offline');
    expect(status.doorwayName).toBe('Alpha');
  });

  it('should show generic offline when hosted + error + no doorway name', () => {
    const { component: c } = createComponent({
      mode: 'hosted',
      holochainState: 'error',
      selectedDoorway: null,
    });
    const status = c.status();

    expect(status.mode).toBe('offline');
    expect(status.label).toBe('Offline');
    expect(status.icon).toBe('cloud_off');
    expect(status.color).toBe('#ef4444');
  });

  // ==========================================================================
  // Hosted + Connected
  // ==========================================================================

  it('should show hosted status when hosted + connected', () => {
    const { component: c } = createComponent({
      mode: 'hosted',
      holochainState: 'connected',
      selectedDoorway: { doorway: { name: 'Alpha' } },
    });
    const status = c.status();

    expect(status.mode).toBe('hosted');
    expect(status.label).toBe('Alpha');
    expect(status.icon).toBe('cloud');
    expect(status.color).toBe('#3b82f6');
    expect(status.cssClass).toBe('status-doorway');
    expect(status.doorwayName).toBe('Alpha');
  });

  // ==========================================================================
  // Hosted + Disconnected (should show connecting, not blue hosted)
  // ==========================================================================

  it('should show connecting when hosted + disconnected', () => {
    const { component: c } = createComponent({
      mode: 'hosted',
      holochainState: 'disconnected',
    });
    const status = c.status();

    expect(status.mode).toBe('connecting');
    expect(status.label).toBe('Connecting');
    expect(status.icon).toBe('sync');
    expect(status.color).toBe('#eab308');
  });

  // ==========================================================================
  // Steward + Connected + Peers
  // ==========================================================================

  it('should show steward with hub icon when connected with peers', () => {
    const { component: c } = createComponent({
      mode: 'steward',
      holochainState: 'connected',
      peerCount: 3,
    });
    const status = c.status();

    expect(status.mode).toBe('steward');
    expect(status.label).toBe('Steward');
    expect(status.icon).toBe('hub');
    expect(status.color).toBe('#22c55e');
    expect(status.cssClass).toBe('status-local');
    expect(status.peerCount).toBe(3);
  });

  // ==========================================================================
  // Steward + Connected + No Peers (Local Only)
  // ==========================================================================

  it('should show steward local mode when connected with no peers', () => {
    const { component: c } = createComponent({
      mode: 'steward',
      holochainState: 'connected',
      peerCount: 0,
    });
    const status = c.status();

    expect(status.mode).toBe('local');
    expect(status.label).toBe('Steward (Local)');
    expect(status.icon).toBe('laptop_mac');
    expect(status.color).toBe('#64748b');
    expect(status.cssClass).toBe('status-local-only');
    expect(status.peerCount).toBe(0);
  });

  // ==========================================================================
  // Reconnecting
  // ==========================================================================

  it('should show reconnecting status', () => {
    const { component: c } = createComponent({
      mode: 'hosted',
      holochainState: 'reconnecting',
    });
    const status = c.status();

    expect(status.mode).toBe('reconnecting');
    expect(status.label).toBe('Reconnecting');
    expect(status.icon).toBe('sync');
    expect(status.color).toBe('#eab308');
    expect(status.cssClass).toBe('status-connecting');
  });

  // ==========================================================================
  // Session Ignores Holochain State
  // ==========================================================================

  it('should show session status even when holochain is in error', () => {
    const { component: c } = createComponent({
      mode: 'session',
      holochainState: 'error',
    });
    const status = c.status();

    expect(status.mode).toBe('session');
    expect(status.label).toBe('Human Session');
    expect(status.icon).toBe('face');
    expect(status.color).toBe('#8b5cf6');
    expect(status.cssClass).toBe('status-session');
  });
});
