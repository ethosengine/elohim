import WebSocket, { RawData } from 'ws';
import { parseMessage, encodeError } from './message-parser.js';
import {
  PermissionLevel,
  isOperationAllowed,
  getPermissionLevelName,
} from './permissions.js';

export interface ProxyOptions {
  clientWs: WebSocket;
  permissionLevel: PermissionLevel;
  conductorUrl: string;
  clientId: string;
  clientOrigin?: string;
  onClose?: () => void;
}

/**
 * Creates a bidirectional WebSocket proxy between client and Holochain conductor.
 * Filters operations based on the client's permission level.
 */
export function createProxy(options: ProxyOptions): void {
  const { clientWs, permissionLevel, conductorUrl, clientId, clientOrigin, onClose } =
    options;

  const levelName = getPermissionLevelName(permissionLevel);
  console.log(
    `[${clientId}] Creating proxy to ${conductorUrl} with ${levelName} access (origin: ${clientOrigin ?? 'none'})`
  );

  // Connect to conductor - must include Origin header (Holochain requirement)
  // Pass through client's origin, or fall back to proxy origin
  const conductorWs = new WebSocket(conductorUrl, {
    origin: clientOrigin ?? 'http://localhost:8080',
  });

  let conductorReady = false;
  const pendingMessages: RawData[] = [];

  conductorWs.on('open', () => {
    console.log(`[${clientId}] Connected to conductor`);
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

  // Client -> Conductor (with filtering)
  clientWs.on('message', (data: RawData) => {
    const buffer = toBuffer(data);
    const message = parseMessage(buffer);

    if (!message) {
      console.warn(`[${clientId}] Failed to parse message, blocking`);
      clientWs.send(encodeError('Invalid message format'));
      return;
    }

    const { type } = message;

    if (!isOperationAllowed(type, permissionLevel)) {
      console.warn(
        `[${clientId}] BLOCKED operation '${type}' (requires higher than ${levelName})`
      );
      clientWs.send(encodeError(`Operation '${type}' not permitted`));
      return;
    }

    console.log(`[${clientId}] ALLOWED operation '${type}'`);

    if (conductorReady) {
      conductorWs.send(data);
    } else {
      // Queue message until conductor is ready
      pendingMessages.push(data);
    }
  });

  // Conductor -> Client (no filtering needed for responses)
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

/**
 * Convert WebSocket raw data to Buffer
 */
function toBuffer(data: RawData): Buffer {
  if (Buffer.isBuffer(data)) {
    return data;
  }
  if (data instanceof ArrayBuffer) {
    return Buffer.from(data);
  }
  if (Array.isArray(data)) {
    return Buffer.concat(data);
  }
  return Buffer.from(data);
}

export interface AppProxyOptions {
  clientWs: WebSocket;
  appPort: number;
  clientId: string;
  clientOrigin?: string;
  onClose?: () => void;
}

/**
 * Creates a simple passthrough proxy for app interface connections.
 * No message filtering - app interfaces don't need permission checks.
 */
export function createAppProxy(options: AppProxyOptions): void {
  const { clientWs, appPort, clientId, clientOrigin, onClose } = options;

  const appUrl = `ws://localhost:${appPort}`;
  console.log(`[${clientId}] Creating app proxy to ${appUrl} (origin: ${clientOrigin ?? 'none'})`);

  // Connect to conductor app interface
  const conductorWs = new WebSocket(appUrl, {
    origin: clientOrigin ?? 'http://localhost:8080',
  });

  conductorWs.on('open', () => {
    console.log(`[${clientId}] Connected to app interface on port ${appPort}`);
  });

  conductorWs.on('error', (err) => {
    console.error(`[${clientId}] App interface connection error:`, err.message);
    clientWs.close(1011, 'App interface connection error');
  });

  // Client -> Conductor (passthrough)
  clientWs.on('message', (data: RawData) => {
    if (conductorWs.readyState === WebSocket.OPEN) {
      conductorWs.send(data);
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
    console.log(`[${clientId}] App client disconnected: ${code} ${reason.toString()}`);
    conductorWs.close();
    onClose?.();
  });

  clientWs.on('error', (err) => {
    console.error(`[${clientId}] App client error:`, err.message);
    conductorWs.close();
  });

  conductorWs.on('close', (code, reason) => {
    console.log(`[${clientId}] App interface disconnected: ${code} ${reason.toString()}`);
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.close(1011, 'App interface disconnected');
    }
    onClose?.();
  });
}
