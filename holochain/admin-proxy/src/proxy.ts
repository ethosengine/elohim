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
  /** Skip message parsing and filtering (dev mode) */
  passthrough?: boolean;
  onClose?: () => void;
}

/**
 * Creates a bidirectional WebSocket proxy between client and Holochain conductor.
 * Filters operations based on the client's permission level.
 *
 * In passthrough mode (dev), skips message parsing and filtering entirely.
 */
export function createProxy(options: ProxyOptions): void {
  const { clientWs, permissionLevel, conductorUrl, clientId, clientOrigin, passthrough, onClose } =
    options;

  const levelName = getPermissionLevelName(permissionLevel);
  const mode = passthrough ? 'passthrough' : 'filtered';
  console.log(
    `[${clientId}] Creating ${mode} proxy to ${conductorUrl} with ${levelName} access (origin: ${clientOrigin ?? 'none'})`
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

  // Client -> Conductor
  if (passthrough) {
    // Dev mode: simple passthrough without parsing/filtering
    clientWs.on('message', (data: RawData) => {
      if (conductorReady) {
        conductorWs.send(data);
      } else {
        pendingMessages.push(data);
      }
    });
  } else {
    // Production mode: parse and filter messages
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
  }

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
  /** Original request URL (to forward query params like token) */
  originalUrl?: string;
  onClose?: () => void;
}

/**
 * Creates a simple passthrough proxy for app interface connections.
 * No message filtering - app interfaces don't need permission checks.
 * Forwards query parameters (like auth token) to the conductor.
 */
export function createAppProxy(options: AppProxyOptions): void {
  const { clientWs, appPort, clientId, clientOrigin, originalUrl, onClose } = options;

  // Build app URL, forwarding query params (except apiKey which is for our proxy)
  let appUrl = `ws://localhost:${appPort}`;
  if (originalUrl) {
    try {
      const url = new URL(originalUrl, 'http://localhost');
      // Remove our apiKey param, keep others (like token)
      url.searchParams.delete('apiKey');
      const queryString = url.searchParams.toString();
      if (queryString) {
        appUrl += `?${queryString}`;
      }
    } catch {
      // Ignore URL parsing errors
    }
  }

  console.log(`[${clientId}] Creating app proxy to ${appUrl} (origin: ${clientOrigin ?? 'none'})`);

  // Connect to conductor app interface
  const conductorWs = new WebSocket(appUrl, {
    origin: clientOrigin ?? 'http://localhost:8080',
  });

  // Buffer messages until conductor connection is ready
  // This is critical because the client sends authenticate message immediately
  let conductorReady = false;
  const pendingMessages: RawData[] = [];

  conductorWs.on('open', () => {
    console.log(`[${clientId}] Connected to app interface on port ${appPort}`);
    conductorReady = true;

    // Send any messages that arrived before conductor was ready
    for (const msg of pendingMessages) {
      conductorWs.send(msg);
    }
    pendingMessages.length = 0;
  });

  conductorWs.on('error', (err) => {
    console.error(`[${clientId}] App interface connection error:`, err.message);
    clientWs.close(1011, 'App interface connection error');
  });

  // Client -> Conductor (passthrough with buffering)
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
