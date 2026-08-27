/**
 * Where does the doorway live when the app is served from a development
 * workspace runtime?
 *
 * A workspace runtime publishes each declared service port at its OWN
 * hostname, derived from the workspace name plus the endpoint name. The browser
 * therefore reaches the doorway at a SIBLING ORIGIN, which the app can only
 * work out at runtime from its own URL.
 *
 * ## Why this is not in the library
 *
 * Everything below is a fact about the workspace product we happen to develop
 * inside, not about the elohim protocol. The protocol core (`@elohim/service`,
 * `elohim-elements`, doorway-service) must not know that any particular
 * workspace vendor exists — it accepts a doorway origin as configuration and
 * uses it. This module is the ONE place that turns a vendor's hostname
 * convention into that configuration. Swapping the workspace runtime (for the
 * elohim-native peer runtime, say) is an edit to this file and nothing else.
 *
 * The co-located `.epr-meta` guards that boundary.
 *
 * ## Why this is not called a "proxy"
 *
 * `useLocalProxy` already means the OPPOSITE topology: "address the doorway at
 * SAME origin and let the dev server reverse-proxy it" — how `/db/*`, `/blob/*`
 * and `/apps/*` are reached. This module resolves a DIFFERENT origin, because a
 * WebSocket upgrade does not survive that reverse proxy. Both are live at once,
 * so naming both "the proxy" is exactly how they get confused.
 */

/**
 * Host substrings that indicate a per-endpoint-hostname workspace runtime.
 * DATA, not logic — a new runtime is a new entry.
 */
const WORKSPACE_HOSTS: readonly string[] = ['.devspaces.', '.code.ethosengine.com'];

/**
 * The endpoint-name segment the app itself is served from, and the segments of
 * the endpoints it needs to reach. These names come from the workspace
 * descriptor (`devfile.yaml` `endpoints:`).
 */
const APP_ENDPOINT = /-angular-dev\./;
const ENDPOINTS = {
  doorway: '-hc-dev.',
  storage: '-hc-storage.',
} as const;

export type WorkspaceEndpoint = keyof typeof ENDPOINTS;

/**
 * Does an arbitrary origin/hostname belong to a workspace runtime?
 *
 * The string-taking form, for callers that already hold an origin rather than
 * reading `globalThis.location` (HTTP interceptors, tests).
 */
export function isWorkspaceOrigin(origin: string): boolean {
  return WORKSPACE_HOSTS.some(fragment => origin.includes(fragment));
}

/** Is this page being served by a per-endpoint-hostname workspace runtime? */
export function isWorkspaceRuntime(): boolean {
  if (globalThis.window === undefined || !globalThis.location) return false;
  return isWorkspaceOrigin(globalThis.location.hostname);
}

/**
 * Absolute origin of one of this workspace's endpoints, or `null` when not in a
 * workspace runtime (deployed, plain localhost, Tauri, SSR).
 *
 * @param endpoint which service to address
 * @param scheme `'https'` for HTTP surfaces (auth, chaperone, storage),
 *   `'wss'` for WebSocket surfaces (conductor admin/app sockets)
 */
export function workspaceEndpointOrigin(
  endpoint: WorkspaceEndpoint,
  scheme: 'https' | 'wss' = 'https'
): string | null {
  if (!isWorkspaceRuntime()) return null;
  const hostname = globalThis.location.hostname.replace(APP_ENDPOINT, ENDPOINTS[endpoint]);
  return `${scheme}://${hostname}`;
}

/**
 * Convenience: the doorway origin for HTTP use, or `null`. This is what gets
 * handed to the library as `ConnectionConfig.doorwayOrigin` / the client
 * provider's `doorwayUrl`.
 */
export function workspaceDoorwayUrl(): string | null {
  return workspaceEndpointOrigin('doorway', 'https');
}
