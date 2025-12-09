/**
 * Sovereignty Service
 *
 * Computes the current sovereignty state for a user by aggregating
 * data from session management and Holochain connection services.
 *
 * This service provides a unified view of:
 * - Current sovereignty stage (Visitor → Hosted → App User → Node Operator)
 * - Data residency (where is my data stored?)
 * - Connection status (am I connected to the network?)
 * - Key/credential information
 * - Migration options
 */

import { Injectable, computed, inject } from '@angular/core';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import {
  type SovereigntyState,
  type SovereigntyStage,
  type ConnectionStatus,
  type KeyInfo,
  type DataResidencyItem,
  SOVEREIGNTY_STAGES,
  getNextStage,
  getVisitorDataResidency,
  getHostedDataResidency,
  getAppUserDataResidency,
} from '../models/sovereignty.model';

@Injectable({
  providedIn: 'root',
})
export class SovereigntyService {
  private readonly holochainService = inject(HolochainClientService);

  /**
   * Computed sovereignty state based on current connections and session.
   */
  readonly sovereigntyState = computed<SovereigntyState>(() => {
    const holochainConnection = this.holochainService.connection();
    const holochainState = holochainConnection.state;
    const displayInfo = this.holochainService.getDisplayInfo();

    // Determine current stage based on connection state
    const currentStage = this.determineStage(holochainState, displayInfo.hasStoredCredentials);

    // Get connection status
    const connectionStatus = this.getConnectionStatus(holochainState, displayInfo.error);

    // Get data residency for current stage
    const dataResidency = this.getDataResidency(currentStage);

    // Get key information if connected
    const keys = this.getKeyInfo(currentStage, displayInfo);

    // Determine next stage for migration
    const nextStage = getNextStage(currentStage);

    return {
      currentStage,
      stageInfo: SOVEREIGNTY_STAGES[currentStage],
      connectionStatus,
      dataResidency,
      keys,
      hasStoredCredentials: displayInfo.hasStoredCredentials,
      networkStats: holochainState === 'connected' ? {
        connectedSince: displayInfo.connectedAt ?? undefined,
        totalPeers: 0, // TODO: Get from conductor when available
        dataShared: 0,
        dataReceived: 0,
      } : undefined,
      migrationAvailable: nextStage !== null,
      migrationTarget: nextStage ?? undefined,
    };
  });

  /**
   * Quick access to current stage.
   */
  readonly currentStage = computed(() => this.sovereigntyState().currentStage);

  /**
   * Quick access to stage info.
   */
  readonly stageInfo = computed(() => this.sovereigntyState().stageInfo);

  /**
   * Quick access to connection status.
   */
  readonly connectionStatus = computed(() => this.sovereigntyState().connectionStatus);

  /**
   * Whether user can upgrade to next stage.
   */
  readonly canUpgrade = computed(() => this.sovereigntyState().migrationAvailable);

  /**
   * Determine sovereignty stage based on connection state.
   */
  private determineStage(
    holochainState: string,
    hasStoredCredentials: boolean
  ): SovereigntyStage {
    // For now, we detect based on Holochain connection status
    // In the future, this will also check for local conductor vs hosted

    if (holochainState === 'connected') {
      // Connected to Edge Node = Hosted User
      // TODO: Detect app-user vs hosted based on conductor location
      return 'hosted';
    }

    if (holochainState === 'connecting' || holochainState === 'authenticating') {
      // Still connecting, but we have intent to connect
      return hasStoredCredentials ? 'hosted' : 'visitor';
    }

    // Not connected = Visitor
    return 'visitor';
  }

  /**
   * Map Holochain connection state to sovereignty connection status.
   */
  private getConnectionStatus(
    holochainState: string,
    error: string | null
  ): ConnectionStatus {
    switch (holochainState) {
      case 'connected':
        return {
          state: 'connected',
          label: 'Connected to Network',
          description: 'Your data syncs with the Elohim DHT network.',
        };

      case 'connecting':
        return {
          state: 'connecting',
          label: 'Connecting...',
          description: 'Establishing connection to Edge Node.',
        };

      case 'authenticating':
        return {
          state: 'connecting',
          label: 'Authenticating...',
          description: 'Setting up your network identity.',
        };

      case 'error':
        return {
          state: 'error',
          label: 'Connection Error',
          description: error ?? 'Failed to connect to network.',
        };

      case 'disconnected':
      default:
        return {
          state: 'offline',
          label: 'Offline',
          description: 'You are not connected to the Elohim network.',
        };
    }
  }

  /**
   * Get data residency items based on sovereignty stage.
   */
  private getDataResidency(stage: SovereigntyStage): DataResidencyItem[] {
    switch (stage) {
      case 'visitor':
        return getVisitorDataResidency();
      case 'hosted':
        return getHostedDataResidency();
      case 'app-user':
      case 'node-operator':
        return getAppUserDataResidency();
      default:
        return getVisitorDataResidency();
    }
  }

  /**
   * Get key information for display.
   */
  private getKeyInfo(
    stage: SovereigntyStage,
    displayInfo: ReturnType<HolochainClientService['getDisplayInfo']>
  ): KeyInfo[] {
    if (stage === 'visitor') {
      return [];
    }

    const keys: KeyInfo[] = [];

    if (displayInfo.agentPubKey) {
      keys.push({
        type: 'agent-pubkey',
        label: 'Agent Public Key',
        value: displayInfo.agentPubKey,
        truncated: this.truncateKey(displayInfo.agentPubKey),
        canExport: true, // Always exportable for non-visitor stages
        canRevoke: false,
      });
    }

    if (displayInfo.hasStoredCredentials) {
      keys.push({
        type: 'signing-key',
        label: 'Signing Key',
        value: '(stored in browser)',
        truncated: 'Active',
        canExport: true,
        canRevoke: true,
      });
    }

    return keys;
  }

  /**
   * Truncate a key for display (first 8 + last 4 chars).
   */
  private truncateKey(key: string): string {
    if (key.length <= 16) return key;
    return `${key.substring(0, 8)}...${key.substring(key.length - 4)}`;
  }

  /**
   * Get summary text for data location.
   */
  getDataSummary(): string {
    const state = this.sovereigntyState();
    const locations = new Set(state.dataResidency.map((d) => d.locationLabel));
    return `${state.dataResidency.length} categories in ${Array.from(locations).join(', ')}`;
  }

  /**
   * Get summary text for sovereignty stage (for compact display).
   */
  getStageSummary(): { data: string; progress: string } {
    const stage = this.currentStage();

    switch (stage) {
      case 'visitor':
        return { data: 'Browser only', progress: 'Temporary' };
      case 'hosted':
        return { data: 'DHT Network', progress: 'Saved' };
      case 'app-user':
        return { data: 'Your Device', progress: 'Saved' };
      case 'node-operator':
        return { data: 'Your Node', progress: 'Always-on' };
      default:
        return { data: 'Unknown', progress: 'Unknown' };
    }
  }
}
