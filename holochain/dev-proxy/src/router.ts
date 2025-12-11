/**
 * Path-based routing for Holochain dev proxy.
 * Routes incoming WebSocket connections to the appropriate conductor interface.
 */

export interface RouteResult {
  /** Target WebSocket URL (e.g., ws://localhost:4444) */
  target: string;
  /** Type of interface being accessed */
  type: 'admin' | 'app';
  /** For app routes, the port number */
  port?: number;
}

/** Default port ranges */
const ADMIN_PORT = 4444;
const APP_PORT_MIN = 4445;
const APP_PORT_MAX = 4500;

/**
 * Resolve a URL pathname to a conductor target.
 *
 * Routes:
 *   /admin        → ws://localhost:4444 (admin interface) or conductorUrl
 *   /app/:port    → ws://localhost:port (app interface)
 *
 * @param pathname - The URL pathname (e.g., "/admin" or "/app/4445")
 * @param config - Optional configuration overrides
 * @returns RouteResult if matched, null otherwise
 */
export function resolveRoute(
  pathname: string,
  config?: {
    adminPort?: number;
    appPortMin?: number;
    appPortMax?: number;
    conductorUrl?: string;
  }
): RouteResult | null {
  const adminPort = config?.adminPort ?? ADMIN_PORT;
  const appPortMin = config?.appPortMin ?? APP_PORT_MIN;
  const appPortMax = config?.appPortMax ?? APP_PORT_MAX;
  const conductorUrl = config?.conductorUrl;

  // Normalize pathname (remove trailing slash)
  const normalizedPath = pathname.replace(/\/$/, '') || '/';

  // Admin interface route
  if (normalizedPath === '/admin') {
    // If conductorUrl is set, use it directly (for remote proxy mode)
    // Otherwise, use localhost with adminPort
    const target = conductorUrl ?? `ws://localhost:${adminPort}`;
    return {
      target,
      type: 'admin',
    };
  }

  // App interface route with port
  const appMatch = normalizedPath.match(/^\/app\/(\d+)$/);
  if (appMatch) {
    const port = parseInt(appMatch[1], 10);

    // Validate port range (only for local mode)
    if (!conductorUrl && (port < appPortMin || port > appPortMax)) {
      console.warn(
        `Port ${port} outside allowed range [${appPortMin}-${appPortMax}]`
      );
      return null;
    }

    // For remote mode, we can't route to specific ports on the remote
    // This only works for local conductor
    const target = conductorUrl
      ? conductorUrl // Remote mode: all traffic to same URL
      : `ws://localhost:${port}`;

    return {
      target,
      type: 'app',
      port,
    };
  }

  return null;
}

/**
 * Get a human-readable description of a route.
 */
export function describeRoute(route: RouteResult): string {
  if (route.type === 'admin') {
    return `Admin interface → ${route.target}`;
  }
  return `App interface (port ${route.port}) → ${route.target}`;
}
