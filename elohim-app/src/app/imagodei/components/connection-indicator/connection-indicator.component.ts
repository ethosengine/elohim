/**
 * ConnectionIndicatorComponent - Network connection status indicator
 *
 * Displays the current connection mode with visual feedback:
 * - Green: Local node (steward)
 * - Blue: Doorway (hosted)
 * - Yellow: Connecting...
 * - Red: Offline (cached mode)
 */

import { CommonModule } from '@angular/common';
import { Component, computed, inject } from '@angular/core';

// @coverage: 76.0% (2026-02-05)

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import { type IdentityMode } from '../../models/identity.model';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { TauriAuthService } from '../../services/tauri-auth.service';

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
  private readonly tauriAuth = inject(TauriAuthService);

  /** Current connection status computed from services */
  readonly status = computed<ConnectionStatus>(() => {
    const identityMode = this.identityService.mode();
    const holochainState = this.holochainService.state();
    const selectedDoorway = this.doorwayRegistry.selected();

    // Graduating state (during stewardship confirmation)
    if (this.tauriAuth.graduationStatus() === 'confirming') {
      return {
        mode: 'migrating' as ConnectionMode,
        label: 'Graduating...',
        icon: 'swap_horiz',
        color: '#f59e0b',
        cssClass: 'status-graduating',
      };
    }

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

    // Steward mode (local node connected)
    if (identityMode === 'steward' && holochainState === 'connected') {
      return {
        mode: 'steward',
        label: 'Steward',
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

  /** Doorway URL for display in hosted mode */
  readonly doorwayUrl = computed(() => this.doorwayRegistry.selectedUrl());

  /** Whether the indicator should be visible */
  readonly isVisible = computed(() => {
    const mode = this.identityService.mode();
    // Show for all non-anonymous users (session, hosted, steward)
    // Session users are valued participants - they should see their status
    return mode !== 'anonymous';
  });

  /** Expanded state for showing details */
  expanded = false;

  toggleExpanded(): void {
    this.expanded = !this.expanded;
  }

  /** Strip protocol and trailing slash from URL for compact display */
  shortenUrl(url: string): string {
    return url.replace(/^https?:\/\//, '').replace(/\/$/, '');
  }
}
