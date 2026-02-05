import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject, signal, computed } from '@angular/core';

// @coverage: 86.2% (2026-02-05)

import { webSocket, WebSocketSubject } from 'rxjs/webSocket';

import { Observable, Subject, catchError, of, retry, timeout } from 'rxjs';

import { environment } from '../../../environments/environment';
import {
  NodesResponse,
  NodeDetails,
  ClusterMetrics,
  ResourceSummary,
  CustodianNetwork,
  DashboardMessage,
  ClientMessage,
  NodeSnapshot,
  ClusterSnapshot,
} from '../models/doorway.model';

/**
 * Connection state for WebSocket
 */
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error';

/**
 * Doorway Admin Service
 *
 * Provides access to the doorway /admin/* endpoints for
 * operator dashboards. Includes both REST API calls and
 * WebSocket real-time updates.
 */
@Injectable({ providedIn: 'root' })
export class DoorwayAdminService {
  private readonly http = inject(HttpClient);

  // Base URL - defaults to same origin if not configured
  private readonly baseUrl = environment.doorwayUrl ?? '';

  // WebSocket connection
  private ws$: WebSocketSubject<DashboardMessage | ClientMessage> | null = null;
  private readonly wsMessages$ = new Subject<DashboardMessage>();

  // Connection state
  private readonly _connectionState = signal<ConnectionState>('disconnected');
  readonly connectionState = this._connectionState.asReadonly();
  readonly isConnected = computed(() => this._connectionState() === 'connected');

  // Latest state from WebSocket
  private readonly _nodes = signal<NodeSnapshot[]>([]);
  private readonly _cluster = signal<ClusterSnapshot | null>(null);
  readonly nodes = this._nodes.asReadonly();
  readonly cluster = this._cluster.asReadonly();

  // Request timeout
  private readonly timeout = 30000;

  // ============================================================================
  // REST API Methods
  // ============================================================================

  /**
   * Get all nodes with detailed resource and social metrics
   */
  getNodes(): Observable<NodesResponse> {
    return this.http.get<NodesResponse>(`${this.baseUrl}/admin/nodes`).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(
        this.handleError<NodesResponse>('getNodes', {
          total: 0,
          byStatus: {
            online: 0,
            degraded: 0,
            offline: 0,
            failed: 0,
            discovering: 0,
            registering: 0,
          },
          nodes: [],
        })
      )
    );
  }

  /**
   * Get specific node details
   */
  getNode(nodeId: string): Observable<NodeDetails | null> {
    return this.http
      .get<NodeDetails>(`${this.baseUrl}/admin/nodes/${nodeId}`)
      .pipe(
        timeout(this.timeout),
        retry(2),
        catchError(this.handleError<NodeDetails | null>('getNode', null))
      );
  }

  /**
   * Get cluster-wide aggregated metrics
   */
  getClusterMetrics(): Observable<ClusterMetrics | null> {
    return this.http
      .get<ClusterMetrics>(`${this.baseUrl}/admin/cluster`)
      .pipe(
        timeout(this.timeout),
        retry(2),
        catchError(this.handleError<ClusterMetrics | null>('getClusterMetrics', null))
      );
  }

  /**
   * Get resource utilization summary
   */
  getResources(): Observable<ResourceSummary | null> {
    return this.http
      .get<ResourceSummary>(`${this.baseUrl}/admin/resources`)
      .pipe(
        timeout(this.timeout),
        retry(2),
        catchError(this.handleError<ResourceSummary | null>('getResources', null))
      );
  }

  /**
   * Get custodian network overview
   */
  getCustodians(): Observable<CustodianNetwork | null> {
    return this.http
      .get<CustodianNetwork>(`${this.baseUrl}/admin/custodians`)
      .pipe(
        timeout(this.timeout),
        retry(2),
        catchError(this.handleError<CustodianNetwork | null>('getCustodians', null))
      );
  }

  // ============================================================================
  // WebSocket Methods
  // ============================================================================

  /**
   * Connect to the real-time dashboard WebSocket
   */
  connect(): void {
    if (this.ws$) {
      return; // Already connected
    }

    this._connectionState.set('connecting');

    // Determine WebSocket URL
    const wsProtocol = globalThis.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = this.baseUrl ? new URL(this.baseUrl).host : globalThis.location.host;
    const wsUrl = `${wsProtocol}//${host}/admin/ws`;

    this.ws$ = webSocket<DashboardMessage | ClientMessage>({
      url: wsUrl,
      openObserver: {
        next: () => {
          this._connectionState.set('connected');
        },
      },
      closeObserver: {
        next: () => {
          this._connectionState.set('disconnected');
          this.ws$ = null;
        },
      },
    });

    // Subscribe to messages
    this.ws$.subscribe({
      next: msg => {
        if (this.isDashboardMessage(msg)) {
          this.handleMessage(msg);
          this.wsMessages$.next(msg);
        }
      },
      error: () => {
        this._connectionState.set('error');
        this.ws$ = null;
      },
    });
  }

  /**
   * Disconnect from WebSocket
   */
  disconnect(): void {
    if (this.ws$) {
      this.ws$.complete();
      this.ws$ = null;
    }
    this._connectionState.set('disconnected');
  }

  /**
   * Send a ping to keep connection alive
   */
  ping(): void {
    if (this.ws$ && this._connectionState() === 'connected') {
      this.ws$.next({ type: 'ping' });
    }
  }

  /**
   * Get observable of WebSocket messages
   */
  get messages$(): Observable<DashboardMessage> {
    return this.wsMessages$.asObservable();
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  /**
   * Type guard for dashboard messages
   */
  private isDashboardMessage(msg: unknown): msg is DashboardMessage {
    return typeof msg === 'object' && msg !== null && 'type' in msg;
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(msg: DashboardMessage): void {
    switch (msg.type) {
      case 'initial_state':
        this._nodes.set(msg.nodes);
        this._cluster.set(msg.cluster);
        break;

      case 'node_update':
        // Update specific node in list
        this._nodes.update(nodes => {
          const idx = nodes.findIndex(n => n.nodeId === msg.nodeId);
          if (idx >= 0) {
            const updated = [...nodes];
            updated[idx] = {
              ...updated[idx],
              status: msg.status,
              combinedScore: msg.combinedScore,
            };
            return updated;
          }
          return nodes;
        });
        break;

      case 'cluster_update':
        this._cluster.set({
          onlineNodes: msg.onlineNodes,
          totalNodes: msg.totalNodes,
          healthRatio: msg.healthRatio,
          avgTrustScore: msg.avgTrustScore,
          avgImpactScore: msg.avgImpactScore,
        });
        break;

      case 'heartbeat':
        // eslint-disable-next-line no-console
        console.debug('[DoorwayAdmin] Heartbeat received');
        break;

      case 'pong':
        // Ping response received
        break;

      case 'error':
        console.error('[DoorwayAdmin] Server error:', msg.message);
        break;
    }
  }

  /**
   * Handle HTTP errors
   */
  private handleError<T>(operation: string, fallback: T) {
    return (error: HttpErrorResponse): Observable<T> => {
      // Log specific warnings for known error conditions
      if (operation === 'getCustodians' && error.status === 503) {
        console.warn(
          `[DoorwayAdmin] ${operation} failed with 503: Orchestrator not enabled or unavailable`
        );
      }

      return of(fallback);
    };
  }
}
