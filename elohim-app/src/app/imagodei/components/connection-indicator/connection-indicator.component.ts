/**
 * ConnectionIndicatorComponent - Network connection status indicator
 *
 * Displays the current connection mode with visual feedback:
 * - Green: Local node (self-sovereign)
 * - Blue: Doorway (hosted)
 * - Yellow: Connecting...
 * - Red: Offline (cached mode)
 */

import { Component, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { IdentityService } from '../../services/identity.service';
import { type IdentityMode } from '../../models/identity.model';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

/** Extended connection status including transient states */
export type ConnectionMode = IdentityMode | 'connecting' | 'offline';

export interface ConnectionStatus {
  mode: ConnectionMode;
  label: string;
  icon: string;
  color: string;
  cssClass: string;
  doorwayName?: string;
}

@Component({
  selector: 'app-connection-indicator',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './connection-indicator.component.html',
  styleUrls: ['./connection-indicator.component.css'],
})
export class ConnectionIndicatorComponent {
  private readonly identityService = inject(IdentityService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly holochainService = inject(HolochainClientService);

  /** Current connection status computed from services */
  readonly status = computed<ConnectionStatus>(() => {
    const identityMode = this.identityService.mode();
    const holochainState = this.holochainService.state();
    const selectedDoorway = this.doorwayRegistry.selected();

    // Connecting state
    if (holochainState === 'connecting' || holochainState === 'authenticating') {
      return {
        mode: 'connecting',
        label: 'Connecting',
        icon: 'sync',
        color: '#eab308',
        cssClass: 'status-connecting',
      };
    }

    // Self-sovereign mode (local node connected)
    if (identityMode === 'self-sovereign' && holochainState === 'connected') {
      return {
        mode: 'self-sovereign',
        label: 'Local Node',
        icon: 'shield',
        color: '#22c55e',
        cssClass: 'status-local',
      };
    }

    // Hosted mode (via doorway)
    if (identityMode === 'hosted') {
      const doorwayName = selectedDoorway?.doorway?.name ?? 'Doorway';
      return {
        mode: 'hosted',
        label: doorwayName,
        icon: 'cloud',
        color: '#3b82f6',
        cssClass: 'status-doorway',
        doorwayName,
      };
    }

    // Offline/error mode
    if (holochainState === 'error') {
      return {
        mode: 'offline',
        label: 'Offline',
        icon: 'cloud_off',
        color: '#ef4444',
        cssClass: 'status-offline',
      };
    }

    // Human Session mode (participant with browser-based identity)
    if (identityMode === 'session') {
      return {
        mode: 'session',
        label: 'Human Session',
        icon: 'face',
        color: '#8b5cf6', // Purple for session - valued but not yet on-chain
        cssClass: 'status-session',
      };
    }

    // Default connecting state
    return {
      mode: 'connecting',
      label: 'Connecting',
      icon: 'sync',
      color: '#eab308',
      cssClass: 'status-connecting',
    };
  });

  /** Whether the indicator should be visible */
  readonly isVisible = computed(() => {
    const mode = this.identityService.mode();
    // Show for all non-anonymous users (session, hosted, self-sovereign)
    // Session users are valued participants - they should see their status
    return mode !== 'anonymous';
  });

  /** Expanded state for showing details */
  expanded = false;

  toggleExpanded(): void {
    this.expanded = !this.expanded;
  }
}
