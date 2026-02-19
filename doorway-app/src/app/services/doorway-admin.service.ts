import { Injectable, inject, signal, computed } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Observable, Subject, catchError, of, retry, timeout } from 'rxjs';
import { webSocket, WebSocketSubject } from 'rxjs/webSocket';
import { environment } from '../../environments/environment';
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
  // User admin models
  UserDetails,
  UsersResponse,
  ListUsersParams,
  UpdateQuotaRequest,
  UserMutationResponse,
  UserPermissionLevel,
  // Pipeline, federation, graduation models
  PipelineResponse,
  FederationDoorwaysAdminResponse,
  FederationPeersConfigResponse,
  P2PPeersResponse,
  GraduationPendingResponse,
  GraduationCompletedResponse,
  // Account models
  AccountResponse,
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
      catchError(this.handleError<NodesResponse>('getNodes', {
        total: 0,
        byStatus: { online: 0, degraded: 0, offline: 0, failed: 0, discovering: 0, registering: 0 },
        nodes: [],
      }))
    );
  }

  /**
   * Get specific node details
   */
  getNode(nodeId: string): Observable<NodeDetails | null> {
    return this.http.get<NodeDetails>(`${this.baseUrl}/admin/nodes/${nodeId}`).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<NodeDetails | null>('getNode', null))
    );
  }

  /**
   * Get cluster-wide aggregated metrics
   */
  getClusterMetrics(): Observable<ClusterMetrics | null> {
    return this.http.get<ClusterMetrics>(`${this.baseUrl}/admin/cluster`).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<ClusterMetrics | null>('getClusterMetrics', null))
    );
  }

  /**
   * Get resource utilization summary
   */
  getResources(): Observable<ResourceSummary | null> {
    return this.http.get<ResourceSummary>(`${this.baseUrl}/admin/resources`).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<ResourceSummary | null>('getResources', null))
    );
  }

  /**
   * Get custodian network overview
   */
  getCustodians(): Observable<CustodianNetwork | null> {
    return this.http.get<CustodianNetwork>(`${this.baseUrl}/admin/custodians`).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<CustodianNetwork | null>('getCustodians', null))
    );
  }

  // ============================================================================
  // Pipeline API Methods
  // ============================================================================

  /**
   * Get agency pipeline stage counts
   */
  getPipeline(): Observable<PipelineResponse> {
    return this.http.get<PipelineResponse>(`${this.baseUrl}/admin/pipeline`).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<PipelineResponse>('getPipeline', {
        registered: 0,
        hosted: 0,
        graduating: 0,
        steward: 0,
      }))
    );
  }

  // ============================================================================
  // Federation Admin API Methods
  // ============================================================================

  /**
   * Get federated doorways for admin dashboard
   */
  getFederationDoorways(): Observable<FederationDoorwaysAdminResponse> {
    return this.http.get<FederationDoorwaysAdminResponse>(
      `${this.baseUrl}/api/v1/federation/doorways`
    ).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<FederationDoorwaysAdminResponse>('getFederationDoorways', {
        doorways: [],
        total: 0,
      }))
    );
  }

  /**
   * Get P2P peer connections
   */
  getP2PPeers(): Observable<P2PPeersResponse> {
    return this.http.get<P2PPeersResponse>(
      `${this.baseUrl}/api/v1/federation/p2p-peers`
    ).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<P2PPeersResponse>('getP2PPeers', {
        peers: [],
        total: 0,
      }))
    );
  }

  /**
   * Get configured federation peer URLs with enriched status
   */
  getFederationPeerConfig(): Observable<FederationPeersConfigResponse> {
    return this.http.get<FederationPeersConfigResponse>(
      `${this.baseUrl}/admin/federation/peers`
    ).pipe(
      timeout(this.timeout),
      retry(2),
      catchError(this.handleError<FederationPeersConfigResponse>('getFederationPeerConfig', {
        peers: [],
        total: 0,
        selfId: null,
      }))
    );
  }

  /**
   * Add a new federation peer URL
   */
  addFederationPeer(url: string): Observable<UserMutationResponse> {
    return this.http.post<UserMutationResponse>(
      `${this.baseUrl}/admin/federation/peers`,
      { url }
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('addFederationPeer'))
    );
  }

  /**
   * Remove a federation peer URL
   */
  removeFederationPeer(url: string): Observable<UserMutationResponse> {
    return this.http.request<UserMutationResponse>(
      'DELETE',
      `${this.baseUrl}/admin/federation/peers`,
      { body: { url } }
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('removeFederationPeer'))
    );
  }

  /**
   * Force refresh of all federation peers
   */
  refreshFederationPeers(): Observable<UserMutationResponse> {
    return this.http.post<UserMutationResponse>(
      `${this.baseUrl}/admin/federation/peers/refresh`,
      {}
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('refreshFederationPeers'))
    );
  }

  // ============================================================================
  // Graduation API Methods
  // ============================================================================

  /**
   * Get users pending graduation
   */
  getGraduationPending(): Observable<GraduationPendingResponse> {
    return this.http.get<GraduationPendingResponse>(
      `${this.baseUrl}/admin/graduation/pending`
    ).pipe(
      timeout(this.timeout),
      retry(1),
      catchError(this.handleError<GraduationPendingResponse>('getGraduationPending', {
        users: [],
        total: 0,
      }))
    );
  }

  /**
   * Get users who have completed graduation
   */
  getGraduationCompleted(): Observable<GraduationCompletedResponse> {
    return this.http.get<GraduationCompletedResponse>(
      `${this.baseUrl}/admin/graduation/completed`
    ).pipe(
      timeout(this.timeout),
      retry(1),
      catchError(this.handleError<GraduationCompletedResponse>('getGraduationCompleted', {
        users: [],
        total: 0,
      }))
    );
  }

  /**
   * Force-graduate a user to steward
   */
  forceGraduate(agentKey: string): Observable<UserMutationResponse> {
    return this.http.post<UserMutationResponse>(
      `${this.baseUrl}/admin/graduation/force/${agentKey}`,
      {}
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('forceGraduate'))
    );
  }

  // ============================================================================
  // Account API Methods (authenticated user self-service)
  // ============================================================================

  /**
   * Get current user's account details
   */
  getAccount(): Observable<AccountResponse | null> {
    return this.http.get<AccountResponse>(`${this.baseUrl}/auth/account`).pipe(
      timeout(this.timeout),
      retry(1),
      catchError(this.handleError<AccountResponse | null>('getAccount', null))
    );
  }

  // ============================================================================
  // User Admin API Methods
  // ============================================================================

  /**
   * List users with pagination and filtering
   */
  listUsers(params: ListUsersParams = {}): Observable<UsersResponse> {
    const queryParams = new URLSearchParams();
    if (params.page) queryParams.set('page', params.page.toString());
    if (params.limit) queryParams.set('limit', params.limit.toString());
    if (params.search) queryParams.set('search', params.search);
    if (params.permissionLevel) queryParams.set('permissionLevel', params.permissionLevel);
    if (params.isActive !== undefined) queryParams.set('isActive', params.isActive.toString());
    if (params.overQuota !== undefined) queryParams.set('overQuota', params.overQuota.toString());
    if (params.sortBy) queryParams.set('sortBy', params.sortBy);
    if (params.sortDir) queryParams.set('sortDir', params.sortDir);

    const queryString = queryParams.toString();
    const url = `${this.baseUrl}/admin/users${queryString ? '?' + queryString : ''}`;

    return this.http.get<UsersResponse>(url).pipe(
      timeout(this.timeout),
      retry(1),
      catchError(this.handleError<UsersResponse>('listUsers', {
        users: [],
        total: 0,
        page: 1,
        limit: 20,
        totalPages: 0,
      }))
    );
  }

  /**
   * Get user details by ID
   */
  getUser(userId: string): Observable<UserDetails | null> {
    return this.http.get<UserDetails>(`${this.baseUrl}/admin/users/${userId}`).pipe(
      timeout(this.timeout),
      retry(1),
      catchError(this.handleError<UserDetails | null>('getUser', null))
    );
  }

  /**
   * Update user active status
   */
  updateUserStatus(userId: string, isActive: boolean): Observable<UserMutationResponse> {
    return this.http.put<UserMutationResponse>(
      `${this.baseUrl}/admin/users/${userId}/status`,
      { isActive }
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('updateUserStatus'))
    );
  }

  /**
   * Force logout user (invalidate all tokens)
   */
  forceLogout(userId: string): Observable<UserMutationResponse> {
    return this.http.post<UserMutationResponse>(
      `${this.baseUrl}/admin/users/${userId}/force-logout`,
      {}
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('forceLogout'))
    );
  }

  /**
   * Soft delete user
   */
  deleteUser(userId: string): Observable<UserMutationResponse> {
    return this.http.delete<UserMutationResponse>(
      `${this.baseUrl}/admin/users/${userId}`
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('deleteUser'))
    );
  }

  /**
   * Reset user password
   */
  resetPassword(userId: string, newPassword: string): Observable<UserMutationResponse> {
    return this.http.post<UserMutationResponse>(
      `${this.baseUrl}/admin/users/${userId}/reset-password`,
      { newPassword }
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('resetPassword'))
    );
  }

  /**
   * Update user permission level
   */
  updatePermission(userId: string, permissionLevel: UserPermissionLevel): Observable<UserMutationResponse> {
    return this.http.put<UserMutationResponse>(
      `${this.baseUrl}/admin/users/${userId}/permission`,
      { permissionLevel }
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('updatePermission'))
    );
  }

  /**
   * Update user quota limits
   */
  updateQuota(userId: string, quota: UpdateQuotaRequest): Observable<UserMutationResponse> {
    return this.http.put<UserMutationResponse>(
      `${this.baseUrl}/admin/users/${userId}/quota`,
      quota
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('updateQuota'))
    );
  }

  /**
   * Reset user usage counters
   */
  resetUsage(userId: string): Observable<UserMutationResponse> {
    return this.http.post<UserMutationResponse>(
      `${this.baseUrl}/admin/users/${userId}/usage/reset`,
      {}
    ).pipe(
      timeout(this.timeout),
      catchError(this.handleMutationError('resetUsage'))
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
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = this.baseUrl
      ? new URL(this.baseUrl).host
      : window.location.host;
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
      next: (msg) => {
        if (this.isDashboardMessage(msg)) {
          this.handleMessage(msg);
          this.wsMessages$.next(msg);
        }
      },
      error: (err) => {
        console.error('[DoorwayAdmin] WebSocket error:', err);
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
        // Could trigger a UI pulse or log
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
   * Handle HTTP errors with fallback
   */
  private handleError<T>(operation: string, fallback: T) {
    return (error: HttpErrorResponse): Observable<T> => {
      console.error(`[DoorwayAdmin] ${operation} failed:`, error.message);

      // Could emit to an error service here
      if (error.status === 503) {
        console.warn('[DoorwayAdmin] Orchestrator not enabled on this doorway');
      }

      return of(fallback);
    };
  }

  /**
   * Handle mutation errors (return failure response instead of fallback)
   */
  private handleMutationError(operation: string) {
    return (error: HttpErrorResponse): Observable<UserMutationResponse> => {
      console.error(`[DoorwayAdmin] ${operation} failed:`, error.message);

      // Extract error message from response if available
      const message = error.error?.error ?? error.message ?? 'Operation failed';

      return of({
        success: false,
        message,
      });
    };
  }
}
