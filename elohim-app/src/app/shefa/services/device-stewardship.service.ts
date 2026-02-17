/**
 * Device Stewardship Service
 *
 * Signal-based Angular service that aggregates device data from:
 * - IdentityService: current identity mode, agencyStage, agentPubKey
 * - TauriAuthService: Tauri environment detection
 * - ShefaComputeService: node topology for node-steward devices
 *
 * Produces a unified DeviceStewardshipState for the device stewardship view.
 */

import { Injectable, inject, signal, computed } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { TauriAuthService } from '@app/imagodei/services/tauri-auth.service';

import {
  type DeviceCategory,
  type StewardedDevice,
  type DeviceStewardshipState,
  createEmptyDeviceStewardshipState,
  mapNodeClusterStatusToDeviceStatus,
  detectPlatform,
} from '../models/device-stewardship.model';

import { ShefaComputeService } from './shefa-compute.service';

import type { OwnedNode } from '../models/shefa-dashboard.model';

const CATEGORY_APP_STEWARD: DeviceCategory = 'app-steward';
const CATEGORY_NODE_STEWARD: DeviceCategory = 'node-steward';

@Injectable({
  providedIn: 'root',
})
export class DeviceStewardshipService {
  private readonly identityService = inject(IdentityService);
  private readonly tauriAuthService = inject(TauriAuthService);
  private readonly shefaComputeService = inject(ShefaComputeService);
  private readonly holochainClient = inject(HolochainClientService);

  // Internal state
  private readonly stateSignal = signal<DeviceStewardshipState>(
    createEmptyDeviceStewardshipState()
  );

  // Public read-only signals
  readonly state = this.stateSignal.asReadonly();
  readonly devices = computed(() => this.stateSignal().devices);
  readonly currentDevice = computed(() => this.stateSignal().currentDevice);
  readonly appStewardDevices = computed(() => this.stateSignal().appStewardDevices);
  readonly nodeStewardDevices = computed(() => this.stateSignal().nodeStewardDevices);
  readonly isLoading = computed(() => this.stateSignal().isLoading);
  readonly error = computed(() => this.stateSignal().error);
  readonly totalDevices = computed(() => this.stateSignal().totalDevices);
  readonly connectedCount = computed(() => this.stateSignal().connectedCount);
  readonly seenCount = computed(() => this.stateSignal().seenCount);
  readonly offlineCount = computed(() => this.stateSignal().offlineCount);

  /**
   * Load all device data. Called by the component on init and refresh.
   */
  async loadDevices(): Promise<void> {
    this.stateSignal.update(s => ({ ...s, isLoading: true, error: null }));

    try {
      const allDevices: StewardedDevice[] = [];

      // 1. Build current device if in Tauri (app-steward)
      const currentDevice = this.buildCurrentDevice();
      if (currentDevice) {
        allDevices.push(currentDevice);
      }

      // 2. Load node-steward devices from topology
      const nodeDevices = await this.loadNodeStewardDevices();

      // 3. Deduplicate: if current device conductor matches a node in topology, merge
      if (currentDevice) {
        const identity = this.identityService.identity();
        const matchingNodeIndex = nodeDevices.findIndex(
          n => identity.conductorUrl && n.doorwayUrl === identity.conductorUrl
        );

        if (matchingNodeIndex >= 0) {
          // Merge: mark the topology node as current device instead
          nodeDevices[matchingNodeIndex] = {
            ...nodeDevices[matchingNodeIndex],
            isCurrentDevice: true,
            platform: currentDevice.platform,
            appVersion: currentDevice.appVersion,
          };
          // Remove the standalone app-steward entry
          allDevices.length = 0;
          allDevices.push(...nodeDevices);
        } else {
          allDevices.push(...nodeDevices);
        }
      } else {
        allDevices.push(...nodeDevices);
      }

      const appStewardDevices = allDevices.filter(d => d.category === CATEGORY_APP_STEWARD);
      const nodeStewardDevices = allDevices.filter(d => d.category === CATEGORY_NODE_STEWARD);
      const now = new Date().toISOString();

      this.stateSignal.set({
        devices: allDevices,
        currentDevice: allDevices.find(d => d.isCurrentDevice) ?? null,
        appStewardDevices,
        nodeStewardDevices,
        totalDevices: allDevices.length,
        connectedCount: allDevices.filter(d => d.status === 'connected').length,
        seenCount: allDevices.filter(d => d.status === 'seen').length,
        offlineCount: allDevices.filter(d => d.status === 'offline').length,
        isLoading: false,
        error: null,
        lastUpdated: now,
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load device data';
      this.stateSignal.update(s => ({
        ...s,
        isLoading: false,
        error: message,
        lastUpdated: new Date().toISOString(),
      }));
    }
  }

  /**
   * Build a StewardedDevice for the current device if in Tauri + app-steward mode.
   */
  private buildCurrentDevice(): StewardedDevice | null {
    const identity = this.identityService.identity();
    const isTauri = this.tauriAuthService.isTauriEnvironment();

    if (!isTauri || identity.agencyStage !== CATEGORY_APP_STEWARD) {
      return null;
    }

    return {
      deviceId: identity.agentPubKey ?? 'current-device',
      displayName: this.getCurrentDeviceName(),
      category: CATEGORY_APP_STEWARD,
      status: 'connected',
      lastSeen: new Date().toISOString(),
      isCurrentDevice: true,
      platform: detectPlatform(navigator.userAgent),
      doorwayUrl: identity.conductorUrl ?? undefined,
    };
  }

  /**
   * Load node-steward devices from the node topology zome.
   */
  private async loadNodeStewardDevices(): Promise<StewardedDevice[]> {
    const identity = this.identityService.identity();
    const stage = identity.agencyStage;

    // Only fetch topology for steward-level users with a Holochain connection
    if (
      !this.holochainClient.isConnected() ||
      (stage !== CATEGORY_APP_STEWARD && stage !== CATEGORY_NODE_STEWARD)
    ) {
      return [];
    }

    const operatorId = identity.humanId;
    if (!operatorId) return [];

    try {
      const topology = await firstValueFrom(this.shefaComputeService.getNodeTopology(operatorId));
      return (topology.nodes ?? []).map(node => this.mapOwnedNodeToDevice(node));
    } catch {
      // Zome may not be available — not an error for device view
      return [];
    }
  }

  /**
   * Map an OwnedNode from topology to a StewardedDevice.
   */
  private mapOwnedNodeToDevice(node: OwnedNode): StewardedDevice {
    return {
      deviceId: node.nodeId,
      displayName: node.displayName,
      category: CATEGORY_NODE_STEWARD,
      status: mapNodeClusterStatusToDeviceStatus(node.status),
      lastSeen: node.lastHeartbeat,
      isCurrentDevice: false,
      nodeType: node.nodeType,
      location: node.location,
      roles: node.roles,
      resources: node.resources,
      isPrimaryNode: node.isPrimary,
    };
  }

  /**
   * Generate a display name for the current device.
   */
  private getCurrentDeviceName(): string {
    const platform = detectPlatform(navigator.userAgent);
    const labels: Record<string, string> = {
      'desktop-macos': 'Mac',
      'desktop-windows': 'Windows PC',
      'desktop-linux': 'Linux PC',
      'mobile-ios': 'iPhone',
      'mobile-android': 'Android',
      unknown: 'Device',
    };
    return `My ${labels[platform] ?? 'Device'}`;
  }
}
