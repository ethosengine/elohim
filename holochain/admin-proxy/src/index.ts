/**
 * Elohim Holochain Proxy
 *
 * Unified proxy for both development and production use.
 *
 * Routes:
 *   /          → Conductor admin interface
 *   /admin     → Conductor admin interface
 *   /app/:port → Conductor app interface (dynamic port)
 *   /health    → Health check endpoint
 *   /status    → Proxy status and active connections
 *
 * Modes:
 *   DEV_MODE=true  → No auth, passthrough all operations
 *   DEV_MODE=false → API key auth, operation filtering
 */

import { createServer, IncomingMessage, ServerResponse } from 'http';
import { WebSocketServer, WebSocket } from 'ws';
import { loadConfig, Config, isCheEnvironment, getCheInfo } from './config.js';
import { validateApiKey, extractApiKey } from './auth.js';
import { createProxy, createAppProxy } from './proxy.js';
import { getPermissionLevelName } from './permissions.js';
import type { Duplex } from 'stream';

let config: Config;
let connectionCounter = 0;
const activeConnections = new Map<string, { route: string; startedAt: Date }>();

/**
 * Parse URL path to determine route type
 */
function parseRoute(url: string): { type: 'admin' } | { type: 'app'; port: number } | null {
  // Match /app/:port pattern
  const appMatch = url.match(/^\/app\/(\d+)/);
  if (appMatch) {
    const port = parseInt(appMatch[1], 10);
    if (port >= config.appPortMin && port <= config.appPortMax) {
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
  res.end(JSON.stringify({
    status: 'ok',
    timestamp: new Date().toISOString(),
    mode: config.devMode ? 'development' : 'production',
    che: isCheEnvironment(),
  }));
}

/**
 * Status endpoint showing active connections and configuration
 */
function handleStatus(res: ServerResponse): void {
  const connections = Array.from(activeConnections.entries()).map(
    ([id, info]) => ({
      id,
      route: info.route,
      startedAt: info.startedAt.toISOString(),
    })
  );

  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({
    activeConnections: connections.length,
    connections,
    config: {
      mode: config.devMode ? 'development' : 'production',
      port: config.port,
      conductorUrl: config.conductorUrl,
      appPortRange: `${config.appPortMin}-${config.appPortMax}`,
    },
    che: getCheInfo(),
  }));
}

/**
 * Handle HTTP requests (health checks, status)
 */
function handleRequest(req: IncomingMessage, res: ServerResponse): void {
  if (req.url === '/health' || req.url === '/healthz') {
    handleHealthCheck(res);
    return;
  }

  if (req.url === '/status') {
    handleStatus(res);
    return;
  }

  // Return 404 for all other HTTP requests
  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({
    error: 'Not Found',
    hint: 'Use WebSocket connection to /admin or /app/:port',
  }));
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

  // Extract and validate API key (skipped in dev mode)
  const apiKey = extractApiKey(url, host);
  const permissionLevel = validateApiKey(apiKey, config);

  if (permissionLevel === null) {
    console.warn(`[${clientId}] Invalid API key, rejecting connection`);
    socket.write('HTTP/1.1 401 Unauthorized\r\n\r\n');
    socket.destroy();
    return;
  }

  const levelName = getPermissionLevelName(permissionLevel);
  const routeDesc = route.type === 'app' ? `app:${route.port}` : 'admin';
  console.log(`[${clientId}] Authenticated with ${levelName} access, route: ${routeDesc}`);

  // Pass through client's Origin header to conductor
  const clientOrigin = request.headers.origin;

  // Accept the WebSocket connection and create appropriate proxy
  wss.handleUpgrade(request, socket, head, (ws: WebSocket) => {
    // Track active connection
    activeConnections.set(clientId, {
      route: routeDesc,
      startedAt: new Date(),
    });

    const onClose = () => {
      activeConnections.delete(clientId);
      console.log(`[${clientId}] Proxy closed`);
    };

    if (route.type === 'app') {
      // App interface - simple passthrough proxy
      // Forward original URL to preserve query params (like token)
      createAppProxy({
        clientWs: ws,
        appPort: route.port,
        clientId,
        clientOrigin,
        originalUrl: url,
        onClose,
      });
    } else {
      // Admin interface - filtered proxy (or passthrough in dev mode)
      createProxy({
        clientWs: ws,
        permissionLevel,
        conductorUrl: config.conductorUrl,
        clientId,
        clientOrigin,
        passthrough: config.devMode,
        onClose,
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

  const mode = config.devMode ? 'DEVELOPMENT' : 'PRODUCTION';
  console.log('Elohim Holochain Proxy starting...');
  console.log(`  Mode: ${mode}`);
  console.log(`  Port: ${config.port}`);
  console.log(`  Conductor URL: ${config.conductorUrl}`);
  console.log(`  App port range: ${config.appPortMin}-${config.appPortMax}`);
  console.log(`  Log level: ${config.logLevel}`);

  if (config.devMode) {
    console.log('  ⚠️  Dev mode: Auth disabled, all operations allowed');
  }

  if (isCheEnvironment()) {
    const cheInfo = getCheInfo();
    console.log(`  Eclipse Che detected: ${cheInfo?.workspaceName ?? 'unknown'}`);
  }

  // Create HTTP server for health checks
  const server = createServer(handleRequest);

  // Create WebSocket server (noServer mode for manual upgrade handling)
  const wss = new WebSocketServer({ noServer: true });

  // Handle WebSocket upgrade
  server.on('upgrade', (request, socket, head) => {
    handleUpgrade(wss, request, socket as Duplex, head);
  });

  // Handle server errors (e.g., port already in use)
  server.on('error', (err: NodeJS.ErrnoException) => {
    if (err.code === 'EADDRINUSE') {
      console.error(`\n❌ Port ${config.port} is already in use.`);
      console.error(`   Another proxy instance may be running.`);
      process.exit(1);
    }
    console.error('❌ Server error:', err.message);
    process.exit(1);
  });

  // Start listening
  server.listen(config.port, () => {
    console.log(`\n✅ Elohim Holochain Proxy listening on port ${config.port}`);
    console.log('Routes:');
    console.log(`  /admin     → ${config.conductorUrl}`);
    console.log(`  /app/:port → ws://localhost:port (range ${config.appPortMin}-${config.appPortMax})`);
    console.log('  /health    → Health check');
    console.log('  /status    → Active connections');
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
