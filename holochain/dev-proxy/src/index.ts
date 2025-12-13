/**
 * Holochain Dev Proxy - Path-based WebSocket proxy for Eclipse Che development.
 *
 * Routes:
 *   /admin     → Conductor admin interface (localhost:4444)
 *   /app/:port → Conductor app interface (localhost:port)
 *   /health    → Health check endpoint
 *   /status    → Proxy status and active connections
 */

import { createServer, IncomingMessage, ServerResponse } from 'http';
import { WebSocketServer, WebSocket } from 'ws';
import { loadConfig, isCheEnvironment, getCheInfo, Config } from './config.js';
import { resolveRoute } from './router.js';
import { createProxy } from './proxy.js';
import type { Duplex } from 'stream';

let config: Config;
let connectionCounter = 0;
const activeConnections = new Map<string, { route: string; startedAt: Date }>();

/**
 * Generate a unique client ID for logging
 */
function generateClientId(): string {
  return `client-${++connectionCounter}`;
}

/**
 * Health check handler for Kubernetes/Che probes
 */
function handleHealthCheck(res: ServerResponse): void {
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(
    JSON.stringify({
      status: 'ok',
      timestamp: new Date().toISOString(),
      che: isCheEnvironment(),
    })
  );
}

/**
 * Status endpoint showing active connections
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
  res.end(
    JSON.stringify({
      activeConnections: connections.length,
      connections,
      config: {
        port: config.port,
        adminPort: config.adminPort,
        appPortRange: `${config.appPortMin}-${config.appPortMax}`,
      },
      che: getCheInfo(),
    })
  );
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
  res.end(
    JSON.stringify({
      error: 'Not Found',
      hint: 'Use WebSocket connection to /admin or /app/:port',
    })
  );
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
  const url = new URL(request.url ?? '/', `http://${request.headers.host}`);

  console.log(
    `[${clientId}] Upgrade request: ${url.pathname} from ${request.socket.remoteAddress}`
  );

  // Resolve route based on path
  const route = resolveRoute(url.pathname, {
    adminPort: config.adminPort,
    appPortMin: config.appPortMin,
    appPortMax: config.appPortMax,
    conductorUrl: config.conductorUrl,
  });

  if (!route) {
    console.warn(`[${clientId}] No route found for path: ${url.pathname}`);
    socket.write('HTTP/1.1 404 Not Found\r\n\r\n');
    socket.destroy();
    return;
  }

  console.log(`[${clientId}] Routing to ${route.target}`);

  // Accept the WebSocket connection
  const clientOrigin = request.headers.origin;

  wss.handleUpgrade(request, socket, head, (ws: WebSocket) => {
    // Track active connection
    activeConnections.set(clientId, {
      route: route.target,
      startedAt: new Date(),
    });

    createProxy({
      clientWs: ws,
      route,
      clientId,
      clientOrigin,
      onClose: () => {
        activeConnections.delete(clientId);
        console.log(`[${clientId}] Proxy closed`);
      },
    });
  });
}

/**
 * Main entry point
 */
function main(): void {
  config = loadConfig();

  console.log('Holochain Dev Proxy starting...');
  console.log(`  Port: ${config.port}`);
  if (config.conductorUrl) {
    console.log(`  Mode: REMOTE`);
    console.log(`  Conductor URL: ${config.conductorUrl}`);
  } else {
    console.log(`  Mode: LOCAL`);
    console.log(`  Admin port: ${config.adminPort}`);
    console.log(`  App port range: ${config.appPortMin}-${config.appPortMax}`);
  }
  console.log(`  Log level: ${config.logLevel}`);

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
      console.error(`   Another dev-proxy instance may be running.`);
      console.error(`\n   To fix this, either:`);
      console.error(`   1. Kill the existing process: pkill -f 'node.*dev-proxy'`);
      console.error(`   2. Or use a different port: PORT=8889 npm start\n`);
      process.exit(1);
    }
    console.error('❌ Server error:', err.message);
    process.exit(1);
  });

  // Start listening
  server.listen(config.port, () => {
    console.log(`\n✅ Holochain Dev Proxy listening on port ${config.port}`);
    console.log('Routes:');
    console.log(`  /admin     → ws://localhost:${config.adminPort}`);
    console.log(
      `  /app/:port → ws://localhost:port (range ${config.appPortMin}-${config.appPortMax})`
    );
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
