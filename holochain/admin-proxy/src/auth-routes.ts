/**
 * HTTP Routes for Authentication.
 *
 * Provides REST API endpoints for hosted human authentication:
 * - POST /auth/register - Create credentials after Holochain registration
 * - POST /auth/login    - Authenticate and get JWT token
 * - POST /auth/logout   - Invalidate token (optional, client-side mainly)
 * - POST /auth/refresh  - Refresh an expiring token
 * - GET  /auth/me       - Get current user info from token
 *
 * All endpoints return JSON responses with appropriate HTTP status codes.
 */

import type { IncomingMessage, ServerResponse } from 'http';
import type { Config } from './config.js';
import {
  getAuthService,
  type RegisterAuthInput,
  type LoginInput,
} from './auth-service.js';
import { extractTokenFromHeader } from './jwt.js';

// =============================================================================
// Types
// =============================================================================

/** HTTP method type */
type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'OPTIONS';

/** JSON response helper */
interface JsonResponse {
  statusCode: number;
  body: Record<string, unknown>;
}

// =============================================================================
// Response Helpers
// =============================================================================

/**
 * Send a JSON response.
 */
function sendJson(res: ServerResponse, statusCode: number, body: Record<string, unknown>): void {
  res.writeHead(statusCode, {
    'Content-Type': 'application/json',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
  });
  res.end(JSON.stringify(body));
}

/**
 * Send a CORS preflight response.
 */
function sendCorsPrelight(res: ServerResponse): void {
  res.writeHead(204, {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    'Access-Control-Max-Age': '86400',
  });
  res.end();
}

/**
 * Parse JSON body from request.
 */
async function parseJsonBody<T>(req: IncomingMessage): Promise<T | null> {
  return new Promise((resolve) => {
    const chunks: Buffer[] = [];

    req.on('data', (chunk: Buffer) => {
      chunks.push(chunk);
      // Limit body size to 10KB
      if (Buffer.concat(chunks).length > 10240) {
        resolve(null);
      }
    });

    req.on('end', () => {
      try {
        const body = Buffer.concat(chunks).toString('utf-8');
        if (!body) {
          resolve(null);
          return;
        }
        const parsed = JSON.parse(body) as T;
        resolve(parsed);
      } catch {
        resolve(null);
      }
    });

    req.on('error', () => {
      resolve(null);
    });
  });
}

/**
 * Extract Authorization header from request.
 */
function getAuthHeader(req: IncomingMessage): string | null {
  const auth = req.headers.authorization;
  return typeof auth === 'string' ? auth : null;
}

// =============================================================================
// Route Handlers
// =============================================================================

/**
 * POST /auth/register
 *
 * Create authentication credentials for an existing Holochain identity.
 * Called after successful register_human zome call.
 */
async function handleRegister(
  req: IncomingMessage,
  res: ServerResponse,
  config: Config
): Promise<void> {
  const body = await parseJsonBody<RegisterAuthInput>(req);

  if (!body) {
    sendJson(res, 400, { error: 'Invalid JSON body' });
    return;
  }

  // Validate required fields
  if (!body.humanId || !body.agentPubKey || !body.identifier || !body.password) {
    sendJson(res, 400, {
      error: 'Missing required fields: humanId, agentPubKey, identifier, password',
    });
    return;
  }

  // Default identifier type to email
  if (!body.identifierType) {
    body.identifierType = 'email';
  }

  const authService = getAuthService(config);
  const result = await authService.register(body);

  if (result.success) {
    sendJson(res, 201, {
      token: result.token,
      humanId: result.humanId,
      agentPubKey: result.agentPubKey,
      expiresAt: result.expiresAt,
      identifier: result.identifier,
    });
  } else {
    const statusCode =
      result.code === 'USER_EXISTS' || result.code === 'IDENTITY_EXISTS'
        ? 409
        : result.code === 'NOT_ENABLED'
        ? 501
        : 400;

    sendJson(res, statusCode, {
      error: result.error,
      code: result.code,
    });
  }
}

/**
 * POST /auth/login
 *
 * Authenticate with identifier and password.
 */
async function handleLogin(
  req: IncomingMessage,
  res: ServerResponse,
  config: Config
): Promise<void> {
  const body = await parseJsonBody<LoginInput>(req);

  if (!body) {
    sendJson(res, 400, { error: 'Invalid JSON body' });
    return;
  }

  if (!body.identifier || !body.password) {
    sendJson(res, 400, { error: 'Missing required fields: identifier, password' });
    return;
  }

  const authService = getAuthService(config);
  const result = await authService.login(body);

  if (result.success) {
    sendJson(res, 200, {
      token: result.token,
      humanId: result.humanId,
      agentPubKey: result.agentPubKey,
      expiresAt: result.expiresAt,
      identifier: result.identifier,
    });
  } else {
    const statusCode =
      result.code === 'NOT_ENABLED' ? 501 : 401;

    sendJson(res, statusCode, {
      error: result.error,
      code: result.code,
    });
  }
}

/**
 * POST /auth/logout
 *
 * Logout (primarily client-side, but can be used for token blacklisting).
 * For now, this is a no-op as tokens are stateless.
 */
async function handleLogout(
  req: IncomingMessage,
  res: ServerResponse,
  _config: Config
): Promise<void> {
  // In the future, we could implement token blacklisting here
  // For now, logout is handled client-side by removing the token

  sendJson(res, 200, {
    success: true,
    message: 'Logged out successfully',
  });
}

/**
 * POST /auth/refresh
 *
 * Refresh an existing token.
 */
async function handleRefresh(
  req: IncomingMessage,
  res: ServerResponse,
  config: Config
): Promise<void> {
  const authHeader = getAuthHeader(req);
  const token = extractTokenFromHeader(authHeader);

  if (!token) {
    sendJson(res, 401, { error: 'No token provided' });
    return;
  }

  const authService = getAuthService(config);
  const result = await authService.refresh(token);

  if (result.success) {
    sendJson(res, 200, {
      token: result.token,
      humanId: result.humanId,
      agentPubKey: result.agentPubKey,
      expiresAt: result.expiresAt,
      identifier: result.identifier,
    });
  } else {
    sendJson(res, 401, {
      error: result.error,
      code: result.code,
    });
  }
}

/**
 * GET /auth/me
 *
 * Get current user info from token.
 */
async function handleMe(
  req: IncomingMessage,
  res: ServerResponse,
  config: Config
): Promise<void> {
  const authHeader = getAuthHeader(req);
  const token = extractTokenFromHeader(authHeader);

  if (!token) {
    sendJson(res, 401, { error: 'No token provided' });
    return;
  }

  const authService = getAuthService(config);
  const user = await authService.getCurrentUser(token);

  if (user) {
    sendJson(res, 200, {
      humanId: user.humanId,
      agentPubKey: user.agentPubKey,
      identifier: user.identifier,
    });
  } else {
    sendJson(res, 401, { error: 'Invalid or expired token' });
  }
}

// =============================================================================
// Main Router
// =============================================================================

/**
 * Handle auth-related HTTP requests.
 *
 * @param req - Incoming HTTP request
 * @param res - Server response
 * @param config - Server configuration
 * @returns true if request was handled, false if not an auth route
 */
export async function handleAuthRequest(
  req: IncomingMessage,
  res: ServerResponse,
  config: Config
): Promise<boolean> {
  const url = req.url ?? '';
  const method = req.method?.toUpperCase() as HttpMethod;

  // Only handle /auth/* routes
  if (!url.startsWith('/auth')) {
    return false;
  }

  // Handle CORS preflight
  if (method === 'OPTIONS') {
    sendCorsPrelight(res);
    return true;
  }

  // Parse the auth route
  const path = url.split('?')[0]; // Remove query string

  try {
    switch (path) {
      case '/auth/register':
        if (method !== 'POST') {
          sendJson(res, 405, { error: 'Method not allowed' });
          return true;
        }
        await handleRegister(req, res, config);
        return true;

      case '/auth/login':
        if (method !== 'POST') {
          sendJson(res, 405, { error: 'Method not allowed' });
          return true;
        }
        await handleLogin(req, res, config);
        return true;

      case '/auth/logout':
        if (method !== 'POST') {
          sendJson(res, 405, { error: 'Method not allowed' });
          return true;
        }
        await handleLogout(req, res, config);
        return true;

      case '/auth/refresh':
        if (method !== 'POST') {
          sendJson(res, 405, { error: 'Method not allowed' });
          return true;
        }
        await handleRefresh(req, res, config);
        return true;

      case '/auth/me':
        if (method !== 'GET') {
          sendJson(res, 405, { error: 'Method not allowed' });
          return true;
        }
        await handleMe(req, res, config);
        return true;

      default:
        sendJson(res, 404, { error: 'Auth endpoint not found' });
        return true;
    }
  } catch (err) {
    console.error('[AuthRoutes] Error handling request:', err);
    sendJson(res, 500, { error: 'Internal server error' });
    return true;
  }
}
