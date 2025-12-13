import { createServer, IncomingMessage, ServerResponse } from 'http';
import { WebSocketServer, WebSocket } from 'ws';
import { loadConfig, Config } from './config.js';
import { validateApiKey, extractApiKey } from './auth.js';
import { createProxy, createAppProxy } from './proxy.js';
import { getPermissionLevelName, PermissionLevel } from './permissions.js';
import type { Duplex } from 'stream';

// App interface port range (matches dev-proxy)
const APP_PORT_MIN = 4445;
const APP_PORT_MAX = 65535;

/**
 * Parse URL path to determine route type
 */
function parseRoute(url: string): { type: 'admin' } | { type: 'app'; port: number } | null {
  // Match /app/:port pattern
  const appMatch = url.match(/^\/app\/(\d+)/);
  if (appMatch) {
    const port = parseInt(appMatch[1], 10);
    if (port >= APP_PORT_MIN && port <= APP_PORT_MAX) {
      return { type: 'app', port };
    }
    return null; // Invalid port
  }

  // Default to admin interface (root path or /admin)
  if (url === '/' || url.startsWith('/?') || url.startsWith('/admin')) {
    return { type: 'admin' };
  }

  return null;
}

let config: Config;
let connectionCounter = 0;

/**
 * Generate a unique client ID for logging
 */
function generateClientId(): string {
  return `client-${++connectionCounter}`;
}

/**
 * Health check handler for Kubernetes probes
 */
function handleHealthCheck(res: ServerResponse): void {
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ status: 'ok', timestamp: new Date().toISOString() }));
}

/**
 * Handle HTTP requests (health checks only)
 */
function handleRequest(req: IncomingMessage, res: ServerResponse): void {
  if (req.url === '/health' || req.url === '/healthz') {
    handleHealthCheck(res);
    return;
  }

  // Return 404 for all other HTTP requests
  res.writeHead(404);
  res.end('Not Found');
}

/**
 * Handle WebSocket upgrade requests
 */
function handleUpgrade(
  wss: WebSocketServer,
  request: IncomingMessage,
  socket: Duplex,
  head: Buffer
): void {
  const clientId = generateClientId();
  const host = request.headers.host ?? 'localhost';
  const url = request.url ?? '/';

  console.log(`[${clientId}] Upgrade request from ${request.socket.remoteAddress} for ${url}`);

  // Parse route to determine admin vs app interface
  const route = parseRoute(url);
  if (!route) {
    console.warn(`[${clientId}] Invalid route: ${url}`);
    socket.write('HTTP/1.1 404 Not Found\r\n\r\n');
    socket.destroy();
    return;
  }

  // Extract and validate API key
  const apiKey = extractApiKey(url, host);
  const permissionLevel = validateApiKey(apiKey, config);

  if (permissionLevel === null) {
    console.warn(`[${clientId}] Invalid API key, rejecting connection`);
    socket.write('HTTP/1.1 401 Unauthorized\r\n\r\n');
    socket.destroy();
    return;
  }

  const levelName = getPermissionLevelName(permissionLevel);
  console.log(`[${clientId}] Authenticated with ${levelName} access, route: ${route.type}`);

  // Pass through client's Origin header to conductor
  const clientOrigin = request.headers.origin;

  // Accept the WebSocket connection and create appropriate proxy
  wss.handleUpgrade(request, socket, head, (ws: WebSocket) => {
    if (route.type === 'app') {
      // App interface - simple passthrough proxy
      createAppProxy({
        clientWs: ws,
        appPort: route.port,
        clientId,
        clientOrigin,
        onClose: () => {
          console.log(`[${clientId}] App proxy closed`);
        },
      });
    } else {
      // Admin interface - filtered proxy with permission checks
      createProxy({
        clientWs: ws,
        permissionLevel,
        conductorUrl: config.conductorUrl,
        clientId,
        clientOrigin,
        onClose: () => {
          console.log(`[${clientId}] Admin proxy closed`);
        },
      });
    }
  });
}

/**
 * Main entry point
 */
function main(): void {
  try {
    config = loadConfig();
  } catch (err) {
    console.error('Failed to load configuration:', err);
    process.exit(1);
  }

  console.log('Elohim Admin Proxy starting...');
  console.log(`  Conductor URL: ${config.conductorUrl}`);
  console.log(`  Port: ${config.port}`);
  console.log(`  Log level: ${config.logLevel}`);

  // Create HTTP server for health checks
  const server = createServer(handleRequest);

  // Create WebSocket server (noServer mode for manual upgrade handling)
  const wss = new WebSocketServer({ noServer: true });

  // Handle WebSocket upgrade
  server.on('upgrade', (request, socket, head) => {
    handleUpgrade(wss, request, socket as Duplex, head);
  });

  // Start listening
  server.listen(config.port, () => {
    console.log(`Elohim Admin Proxy listening on port ${config.port}`);
  });

  // Graceful shutdown
  process.on('SIGTERM', () => {
    console.log('SIGTERM received, shutting down...');
    server.close(() => {
      console.log('Server closed');
      process.exit(0);
    });
  });

  process.on('SIGINT', () => {
    console.log('SIGINT received, shutting down...');
    server.close(() => {
      console.log('Server closed');
      process.exit(0);
    });
  });
}

main();
