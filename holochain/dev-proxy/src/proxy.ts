/**
 * Bidirectional WebSocket proxy for Holochain dev connections.
 * Simplified from admin-proxy - no permission filtering for local development.
 */

import WebSocket, { RawData } from 'ws';
import { RouteResult, describeRoute } from './router.js';

export interface ProxyOptions {
  /** Client WebSocket connection */
  clientWs: WebSocket;
  /** Resolved route with target URL */
  route: RouteResult;
  /** Unique client identifier for logging */
  clientId: string;
  /** Origin header from client request */
  clientOrigin?: string;
  /** Callback when proxy closes */
  onClose?: () => void;
}

/**
 * Creates a bidirectional WebSocket proxy between client and Holochain conductor.
 * No filtering - all messages are forwarded directly (dev mode).
 */
export function createProxy(options: ProxyOptions): void {
  const { clientWs, route, clientId, clientOrigin, onClose } = options;

  console.log(`[${clientId}] Creating proxy: ${describeRoute(route)}`);

  // Connect to conductor - must include Origin header (Holochain requirement)
  const conductorWs = new WebSocket(route.target, {
    origin: clientOrigin ?? 'http://localhost:8888',
  });

  let conductorReady = false;
  const pendingMessages: RawData[] = [];

  conductorWs.on('open', () => {
    console.log(`[${clientId}] Connected to conductor at ${route.target}`);
    conductorReady = true;

    // Send any messages that arrived before conductor was ready
    for (const msg of pendingMessages) {
      conductorWs.send(msg);
    }
    pendingMessages.length = 0;
  });

  conductorWs.on('error', (err) => {
    console.error(`[${clientId}] Conductor connection error:`, err.message);
    clientWs.close(1011, 'Conductor connection error');
  });

  // Client -> Conductor (no filtering in dev mode)
  clientWs.on('message', (data: RawData) => {
    if (conductorReady) {
      conductorWs.send(data);
    } else {
      // Queue message until conductor is ready
      pendingMessages.push(data);
    }
  });

  // Conductor -> Client (passthrough)
  conductorWs.on('message', (data: RawData) => {
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.send(data);
    }
  });

  // Handle disconnections
  clientWs.on('close', (code, reason) => {
    console.log(
      `[${clientId}] Client disconnected: ${code} ${reason.toString()}`
    );
    conductorWs.close();
    onClose?.();
  });

  clientWs.on('error', (err) => {
    console.error(`[${clientId}] Client error:`, err.message);
    conductorWs.close();
  });

  conductorWs.on('close', (code, reason) => {
    console.log(
      `[${clientId}] Conductor disconnected: ${code} ${reason.toString()}`
    );
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.close(1011, 'Conductor disconnected');
    }
    onClose?.();
  });
}
