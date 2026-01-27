import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal, computed } from '@angular/core';

import { Observable } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { IdentityService } from '@app/imagodei/services/identity.service';

/**
 * Registered node information
 */
export interface RegisteredNode {
  nodeId: string;
  displayName: string;
  nodeType: 'holoport' | 'holoport-plus' | 'self-hosted' | 'cloud';
  status: 'online' | 'offline' | 'degraded' | 'unknown';
  lastSeen: string | null;
  /** URL to doorway web hosting interface (null if no web hosting) */
  doorwayUrl: string | null;
  /** Whether this node has web hosting (doorway) capability */
  hasDoorway: boolean;
}

/**
 * User's compute context
 */
export interface ComputeContext {
  /** Whether user has any registered nodes */
  hasRegisteredNodes: boolean;
  /** All registered nodes */
  registeredNodes: RegisteredNode[];
  /** Primary node (first holoport or first node) */
  primaryNode: RegisteredNode | null;
  /** Total node count */
  totalNodes: number;
  /** Online node count */
  onlineNodes: number;
  /** Whether user has any nodes with web hosting (doorway) capability */
  hasDoorwayCapableNode: boolean;
  /** Nodes that have doorway/web hosting capability */
  doorwayNodes: RegisteredNode[];
  /** When this context was last detected */
  detectedAt: Date;
}

/**
 * RunningContextService
 *
 * Determines if the authenticated user has always-on nodes registered
 * to their Imago Dei identity. This controls visibility of:
 *
 * - Operator app in context tray (for users with registered nodes)
 * - Compute resources section in Shefa (simplified view for all native users)
 *
 * Detection flow:
 * 1. Check if user is authenticated via IdentityService
 * 2. Query node_registry for nodes registered to this agent
 * 3. Update signals for reactive UI binding
 *
 * Users who have always-on nodes (Holoports, self-hosted, cloud) get
 * access to the operator dashboard to monitor their compute resources.
 */
@Injectable({ providedIn: 'root' })
export class RunningContextService {
  private readonly http = inject(HttpClient);
  private readonly identityService = inject(IdentityService);
  private readonly holochainClient = inject(HolochainClientService);

  // Compute context state
  private readonly _context = signal<ComputeContext>({
    hasRegisteredNodes: false,
    registeredNodes: [],
    primaryNode: null,
    totalNodes: 0,
    onlineNodes: 0,
    hasDoorwayCapableNode: false,
    doorwayNodes: [],
    detectedAt: new Date(),
  });

  readonly context = this._context.asReadonly();
  readonly hasRegisteredNodes = computed(() => this._context().hasRegisteredNodes);
  readonly registeredNodes = computed(() => this._context().registeredNodes);
  readonly primaryNode = computed(() => this._context().primaryNode);

  /**
   * Whether Doorway app should be visible in context tray
   * True when user has at least one always-on node with web hosting capability
   */
  readonly hasDoorwayCapableNode = computed(() => this._context().hasDoorwayCapableNode);

  /**
   * Nodes that have doorway/web hosting capability
   */
  readonly doorwayNodes = computed(() => this._context().doorwayNodes);

  /**
   * Whether user is using Holochain natively (has nodes or is connected)
   */
  readonly isHolochainNative = computed(
    () => this._context().hasRegisteredNodes || this.holochainClient.state() === 'connected'
  );

  // Detection interval
  private detectionInterval: ReturnType<typeof setInterval> | null = null;

  /**
   * Initialize context detection
   * Call this early in app initialization
   */
  async detect(): Promise<ComputeContext> {
    const context = await this.performDetection();
    this._context.set(context);
    return context;
  }

  /**
   * Start periodic context detection (every 60 seconds)
   */
  startPeriodicDetection(): void {
    if (this.detectionInterval) return;

    // Initial detection
    this.detect();

    // Periodic refresh
    this.detectionInterval = setInterval(() => {
      this.detect();
    }, 60000);
  }

  /**
   * Stop periodic detection
   */
  stopPeriodicDetection(): void {
    if (this.detectionInterval) {
      clearInterval(this.detectionInterval);
      this.detectionInterval = null;
    }
  }

  /**
   * Perform context detection by querying user's registered nodes
   */
  private async performDetection(): Promise<ComputeContext> {
    const detectedAt = new Date();

    // Check if user is authenticated
    const mode = this.identityService.mode();
    if (mode !== 'hosted' && mode !== 'self-sovereign') {
      // Not authenticated - no registered nodes
      return {
        hasRegisteredNodes: false,
        registeredNodes: [],
        primaryNode: null,
        totalNodes: 0,
        onlineNodes: 0,
        hasDoorwayCapableNode: false,
        doorwayNodes: [],
        detectedAt,
      };
    }

    // Query for registered nodes
    try {
      const nodes = await this.getRegisteredNodes();
      const onlineNodes = nodes.filter(n => n.status === 'online').length;
      const primaryNode =
        nodes.find(n => n.nodeType === 'holoport' || n.nodeType === 'holoport-plus') ??
        nodes[0] ??
        null;

      // Find nodes with doorway/web hosting capability
      // Holoports and holoport-plus always have doorway
      // Self-hosted nodes have doorway if doorwayUrl is set
      const doorwayNodes = nodes.filter(n => n.hasDoorway);

      return {
        hasRegisteredNodes: nodes.length > 0,
        registeredNodes: nodes,
        primaryNode,
        totalNodes: nodes.length,
        onlineNodes,
        hasDoorwayCapableNode: doorwayNodes.length > 0,
        doorwayNodes,
        detectedAt,
      };
    } catch (error) {
      console.warn('[RunningContext] Failed to get registered nodes:', error);
      return {
        hasRegisteredNodes: false,
        registeredNodes: [],
        primaryNode: null,
        totalNodes: 0,
        onlineNodes: 0,
        hasDoorwayCapableNode: false,
        doorwayNodes: [],
        detectedAt,
      };
    }
  }

  /**
   * Get nodes registered to the current user's Imago Dei identity
   */
  private async getRegisteredNodes(): Promise<RegisteredNode[]> {
    // Try to get nodes from node_registry_coordinator
    try {
      const result = await this.holochainClient.callZome<any[]>({
        zomeName: 'node_registry_coordinator',
        fnName: 'get_my_nodes',
        payload: null,
      });

      if (!result.success || !result.data) {
        return [];
      }

      return (result.data || []).map(n => {
        const nodeType = n.node_type || 'self-hosted';
        const doorwayUrl = n.doorway_url || null;

        // Holoports always have doorway capability
        // Self-hosted/cloud nodes have doorway if doorwayUrl is configured
        const hasDoorway =
          nodeType === 'holoport' || nodeType === 'holoport-plus' || doorwayUrl !== null;

        return {
          nodeId: n.node_id,
          displayName: n.display_name || n.node_id.substring(0, 8),
          nodeType,
          status: this.mapStatus(n.status),
          lastSeen: n.last_heartbeat || null,
          doorwayUrl,
          hasDoorway,
        };
      });
    } catch {
      return [];
    }
  }

  /**
   * Map node status string to known status
   */
  private mapStatus(status: string | undefined): 'online' | 'offline' | 'degraded' | 'unknown' {
    switch (status?.toLowerCase()) {
      case 'online':
        return 'online';
      case 'offline':
        return 'offline';
      case 'degraded':
        return 'degraded';
      default:
        return 'unknown';
    }
  }

  /**
   * Get context as observable (for async pipe)
   */
  get context$(): Observable<ComputeContext> {
    return new Observable(subscriber => {
      // Emit current value
      subscriber.next(this._context());

      // Subscribe to changes via effect-like pattern
      const intervalId = setInterval(() => {
        subscriber.next(this._context());
      }, 1000);

      return () => clearInterval(intervalId);
    });
  }
}
