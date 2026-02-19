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
import { HttpClient } from '@angular/common/http';
import { Component, computed, DestroyRef, inject, OnInit, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';

import { catchError, of, switchMap, timer } from 'rxjs';

// @coverage: 76.0% (2026-02-05)

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { StorageClientService } from '@app/elohim/services/storage-client.service';

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
  peerCount?: number;
}

@Component({
  selector: 'app-connection-indicator',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './connection-indicator.component.html',
  styleUrls: ['./connection-indicator.component.css'],
})
export class ConnectionIndicatorComponent implements OnInit {
  private readonly identityService = inject(IdentityService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly holochainService = inject(HolochainClientService);
  private readonly tauriAuth = inject(TauriAuthService);
  private readonly storageClient = inject(StorageClientService);
  private readonly http = inject(HttpClient);
  private readonly destroyRef = inject(DestroyRef);

  /** P2P peer count from elohim-storage or doorway health */
  readonly peerCount = signal<number>(0);

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
        peerCount: this.peerCount(),
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
        peerCount: this.peerCount(),
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

  ngOnInit(): void {
    // Poll P2P status every 30 seconds
    timer(0, 30_000)
      .pipe(
        switchMap(() => {
          const mode = this.storageClient.connectionMode;
          if (mode === 'direct') {
            // Steward mode: poll elohim-storage directly
            const baseUrl = this.storageClient.getStorageBaseUrl();
            return this.http
              .get<{ connected_peers?: number; peer_count?: number }>(`${baseUrl}/p2p/status`)
              .pipe(catchError(() => of(null)));
          }
          // Hosted mode: extract from doorway /health
          const baseUrl = this.storageClient.getStorageBaseUrl();
          return this.http
            .get<{ p2p?: { peer_count?: number } }>(`${baseUrl}/health`)
            .pipe(catchError(() => of(null)));
        }),
        takeUntilDestroyed(this.destroyRef)
      )
      .subscribe(resp => {
        if (!resp) {
          this.peerCount.set(0);
          return;
        }
        // Direct mode returns connected_peers, doorway health returns p2p.peer_count
        const count =
          (resp as { connected_peers?: number }).connected_peers ??
          (resp as { p2p?: { peer_count?: number } }).p2p?.peer_count ??
          0;
        this.peerCount.set(count);
      });
  }
}
