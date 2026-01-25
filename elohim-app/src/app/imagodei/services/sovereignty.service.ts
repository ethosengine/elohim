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
import { type KeyLocation, type IdentityMode } from '../models/identity.model';
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
      networkStats:
        holochainState === 'connected'
          ? {
              connectedSince: displayInfo.connectedAt ?? undefined,
              totalPeers: 0, // TODO: Get from conductor when available
              dataShared: 0,
              dataReceived: 0,
            }
          : undefined,
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
   * Determine sovereignty stage based on connection state and conductor location.
   *
   * Stage Detection Logic:
   * - visitor: No Holochain connection, session-only or anonymous
   * - hosted: Connected to remote edge node (custodial keys)
   * - app-user: Connected to local conductor on user's device (self-sovereign keys)
   * - node-operator: Local conductor that also hosts other humans
   */
  private determineStage(holochainState: string, hasStoredCredentials: boolean): SovereigntyStage {
    const displayInfo = this.holochainService.getDisplayInfo();

    if (holochainState === 'connected') {
      // Detect if conductor is local or remote
      const isLocalConductor = this.isLocalConductor(displayInfo.appUrl);

      if (isLocalConductor) {
        // Local conductor - either app-user or node-operator
        // For now, detect node-operator based on configuration
        // In the future, check if hosting other humans via DHT query
        const isNodeOperator = this.detectNodeOperatorStatus();
        return isNodeOperator ? 'node-operator' : 'app-user';
      }

      // Remote conductor = Hosted User
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
   * Check if conductor is running locally on user's device.
   */
  private isLocalConductor(url: string | null): boolean {
    if (!url) return false;

    // Local conductor indicators
    const localPatterns = ['localhost', '127.0.0.1', '[::1]', '0.0.0.0'];

    const urlLower = url.toLowerCase();
    return localPatterns.some(pattern => urlLower.includes(pattern));
  }

  /**
   * Detect if user is operating a node (hosting other humans).
   *
   * Node operator detection (future enhancements):
   * - Query DHT for hosted agent count
   * - Check conductor configuration for hosting mode
   * - Verify always-on uptime requirements
   */
  private detectNodeOperatorStatus(): boolean {
    // For MVP, check for node operator flag in localStorage
    // In production, this will query the conductor for hosted agents
    try {
      const nodeConfig = localStorage.getItem('elohim_node_operator_config');
      if (nodeConfig) {
        const config = JSON.parse(nodeConfig);
        return config.isNodeOperator === true && config.hostedHumanCount > 0;
      }
    } catch {
      // Ignore parse errors
    }
    return false;
  }

  /**
   * Map Holochain connection state to sovereignty connection status.
   */
  private getConnectionStatus(holochainState: string, error: string | null): ConnectionStatus {
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
    const keyLocation = this.getKeyLocation(stage);

    if (displayInfo.agentPubKey) {
      keys.push({
        type: 'agent-pubkey',
        label: 'Agent Public Key',
        value: displayInfo.agentPubKey,
        truncated: this.truncateKey(displayInfo.agentPubKey),
        canExport: stage !== 'hosted', // Can't export from custodial hosting
        canRevoke: false,
      });
    }

    // Add key location info based on stage
    const keyLocationInfo = this.getKeyLocationInfo(stage, keyLocation);
    if (keyLocationInfo) {
      keys.push(keyLocationInfo);
    }

    return keys;
  }

  /**
   * Determine key location based on sovereignty stage.
   */
  private getKeyLocation(stage: SovereigntyStage): KeyLocation {
    switch (stage) {
      case 'visitor':
        return 'none';
      case 'hosted':
        return 'custodial'; // Keys held by edge node
      case 'app-user':
        return 'device'; // Keys on local conductor
      case 'node-operator':
        return 'device'; // Could be 'hardware' if using HSM
      default:
        return 'none';
    }
  }

  /**
   * Get key location display info.
   */
  private getKeyLocationInfo(stage: SovereigntyStage, location: KeyLocation): KeyInfo | null {
    switch (location) {
      case 'custodial':
        return {
          type: 'signing-key',
          label: 'Signing Key',
          value: 'Held by Edge Node',
          truncated: 'Custodial',
          canExport: false,
          canRevoke: false,
        };
      case 'device':
        return {
          type: 'signing-key',
          label: 'Signing Key',
          value: 'On your device',
          truncated: stage === 'node-operator' ? 'Steward (Node)' : 'Steward',
          canExport: true,
          canRevoke: true,
        };
      case 'hardware':
        return {
          type: 'signing-key',
          label: 'Signing Key',
          value: 'Hardware Security Module',
          truncated: 'Hardware Protected',
          canExport: false, // Hardware keys can't be exported
          canRevoke: true,
        };
      case 'browser':
        return {
          type: 'signing-key',
          label: 'Signing Key',
          value: 'Browser Storage',
          truncated: 'Browser (Less Secure)',
          canExport: true,
          canRevoke: true,
        };
      default:
        return null;
    }
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
    const locations = new Set(state.dataResidency.map(d => d.locationLabel));
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
