/**
 * WebSocket Connection Manager
 *
 * Manages WebSocket lifecycle with automatic reconnection, heartbeat monitoring,
 * and proper cleanup. Abstracts RxJS WebSocket implementation.
 */

import { Subject, Observable, Subscription, timer } from 'rxjs';
import { webSocket, WebSocketSubject } from 'rxjs/webSocket';
import { WebSocketConfig, DEFAULT_WEBSOCKET_CONFIG } from './websocket-config';
import { WebSocketError } from '../errors/doorway-errors';

/**
 * Connection state for WebSocket
 */
export enum ConnectionState {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  RECONNECTING = 'reconnecting',
  ERROR = 'error',
}

/**
 * WebSocket connection manager with reconnection and heartbeat
 */
export class WebSocketManager<TMessage = unknown> {
  private ws$: WebSocketSubject<TMessage> | null = null;
  private readonly messages$ = new Subject<TMessage>();
  private readonly stateSubject = new Subject<ConnectionState>();
  private readonly errorSubject = new Subject<WebSocketError>();

  private currentState: ConnectionState = ConnectionState.DISCONNECTED;
  private reconnectAttempts = 0;
  private reconnectTimer: Subscription | null = null;
  private heartbeatTimer: Subscription | null = null;
  private heartbeatTimeoutTimer: Subscription | null = null;
  private lastPongReceived = 0;

  constructor(
    private readonly url: string,
    private readonly config: WebSocketConfig = DEFAULT_WEBSOCKET_CONFIG
  ) {}

  /**
   * Connect to WebSocket
   */
  connect(): void {
    if (this.currentState !== ConnectionState.DISCONNECTED) {
      return;
    }

    this.setState(ConnectionState.CONNECTING);
    this.reconnectAttempts = 0;
    this.createConnection();
  }

  /**
   * Disconnect from WebSocket
   */
  disconnect(): void {
    this.cleanup();
    this.setState(ConnectionState.DISCONNECTED);
  }

  /**
   * Send message through WebSocket
   */
  send(message: TMessage): void {
    if (!this.ws$ || this.currentState !== ConnectionState.CONNECTED) {
      throw new WebSocketError('Cannot send message: WebSocket not connected');
    }
    this.ws$.next(message);
  }

  /**
   * Get observable of incoming messages
   */
  get messages(): Observable<TMessage> {
    return this.messages$.asObservable();
  }

  /**
   * Get observable of connection state changes
   */
  get state(): Observable<ConnectionState> {
    return this.stateSubject.asObservable();
  }

  /**
   * Get observable of errors
   */
  get errors(): Observable<WebSocketError> {
    return this.errorSubject.asObservable();
  }

  /**
   * Get current connection state
   */
  getCurrentState(): ConnectionState {
    return this.currentState;
  }

  /**
   * Check if currently connected
   */
  isConnected(): boolean {
    return this.currentState === ConnectionState.CONNECTED;
  }

  /**
   * Create WebSocket connection
   */
  private createConnection(): void {
    this.ws$ = webSocket<TMessage>({
      url: this.url,
      openObserver: {
        next: () => {
          this.reconnectAttempts = 0;
          this.setState(ConnectionState.CONNECTED);
          this.startHeartbeat();
        },
      },
      closeObserver: {
        next: () => {
          this.handleDisconnect();
        },
      },
    });

    // Subscribe to messages
    this.ws$.subscribe({
      next: (msg) => {
        this.handleMessage(msg);
      },
      error: (err) => {
        console.error('[WebSocketManager] Error:', err);
        const wsError = new WebSocketError('WebSocket error occurred', undefined, err);
        this.errorSubject.next(wsError);
        this.handleDisconnect();
      },
    });
  }

  /**
   * Handle incoming message
   */
  private handleMessage(msg: TMessage): void {
    // Check if it's a pong response
    if (this.isPongMessage(msg)) {
      this.lastPongReceived = Date.now();
      this.clearHeartbeatTimeout();
      return;
    }

    // Emit message to subscribers
    this.messages$.next(msg);
  }

  /**
   * Check if message is a pong response
   */
  private isPongMessage(msg: TMessage): boolean {
    return (
      typeof msg === 'object' &&
      msg !== null &&
      'type' in msg &&
      (msg as { type: string }).type === 'pong'
    );
  }

  /**
   * Handle disconnection
   */
  private handleDisconnect(): void {
    this.cleanup();

    // Attempt reconnection if enabled
    if (
      this.config.reconnection.enabled &&
      (this.config.reconnection.maxAttempts === 0 ||
        this.reconnectAttempts < this.config.reconnection.maxAttempts)
    ) {
      this.scheduleReconnect();
    } else {
      this.setState(ConnectionState.ERROR);
    }
  }

  /**
   * Schedule reconnection attempt
   */
  private scheduleReconnect(): void {
    this.setState(ConnectionState.RECONNECTING);

    // Calculate delay with exponential backoff
    const delay = Math.min(
      this.config.reconnection.initialDelayMs *
        Math.pow(
          this.config.reconnection.backoffMultiplier,
          this.reconnectAttempts
        ),
      this.config.reconnection.maxDelayMs
    );

    this.reconnectTimer = timer(delay).subscribe(() => {
      this.reconnectAttempts++;
      this.setState(ConnectionState.CONNECTING);
      this.createConnection();
    });
  }

  /**
   * Start heartbeat/ping mechanism
   */
  private startHeartbeat(): void {
    if (!this.config.heartbeat.enabled) {
      return;
    }

    this.stopHeartbeat();

    this.heartbeatTimer = timer(
      this.config.heartbeat.intervalMs,
      this.config.heartbeat.intervalMs
    ).subscribe(() => {
      this.sendPing();
    });
  }

  /**
   * Send ping message
   */
  private sendPing(): void {
    try {
      const pingMessage = { type: 'ping' } as TMessage;
      this.send(pingMessage);

      // Set timeout for pong response
      this.heartbeatTimeoutTimer = timer(this.config.heartbeat.timeoutMs).subscribe(
        () => {
          const timeSincePong = Date.now() - this.lastPongReceived;
          if (timeSincePong > this.config.heartbeat.timeoutMs) {
            console.warn(
              '[WebSocketManager] Heartbeat timeout - no pong received'
            );
            this.handleDisconnect();
          }
        }
      );
    } catch (error) {
      console.error('[WebSocketManager] Failed to send ping:', error);
    }
  }

  /**
   * Clear heartbeat timeout
   */
  private clearHeartbeatTimeout(): void {
    if (this.heartbeatTimeoutTimer) {
      this.heartbeatTimeoutTimer.unsubscribe();
      this.heartbeatTimeoutTimer = null;
    }
  }

  /**
   * Stop heartbeat mechanism
   */
  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      this.heartbeatTimer.unsubscribe();
      this.heartbeatTimer = null;
    }
    this.clearHeartbeatTimeout();
  }

  /**
   * Set connection state
   */
  private setState(state: ConnectionState): void {
    this.currentState = state;
    this.stateSubject.next(state);
  }

  /**
   * Cleanup resources
   */
  private cleanup(): void {
    if (this.ws$) {
      this.ws$.complete();
      this.ws$ = null;
    }

    if (this.reconnectTimer) {
      this.reconnectTimer.unsubscribe();
      this.reconnectTimer = null;
    }

    this.stopHeartbeat();
  }

  /**
   * Cleanup on destroy
   */
  destroy(): void {
    this.cleanup();
    this.messages$.complete();
    this.stateSubject.complete();
    this.errorSubject.complete();
  }
}
