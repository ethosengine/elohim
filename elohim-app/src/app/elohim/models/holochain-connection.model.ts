/**
 * Holochain Connection Models
 *
 * Types for managing WebSocket connections to Holochain conductor.
 * Part of the Edge Node spike for web-first P2P architecture.
 *
 * @see https://github.com/holochain/holochain-client-js
 */

import type {
  AdminWebsocket,
  AppWebsocket,
  AgentPubKey,
  CellId,
  AppInfo,
  InstalledAppId,
} from '@holochain/client';
import { environment } from '../../../environments/environment';
import type { HolochainEnvironmentConfig } from '../../../environments/environment.types';

// Cast to include optional properties from the type definition
const holochainConfig = environment.holochain as HolochainEnvironmentConfig | undefined;

/**
 * Connection state machine
 */
export type HolochainConnectionState =
  | 'disconnected'    // Not connected to conductor
  | 'connecting'      // WebSocket connection in progress
  | 'authenticating'  // Generating keys / getting token
  | 'connected'       // Ready for zome calls
  | 'error';          // Connection failed

/**
 * Configuration for connecting to Holochain conductor
 */
export interface HolochainConfig {
  /** Admin WebSocket URL (e.g., wss://holochain-dev.elohim.host for proxy) */
  adminUrl: string;

  /** App WebSocket URL (same as adminUrl when using proxy, localhost when local) */
  appUrl: string;

  /** Origin for CORS (must match conductor config) */
  origin: string;

  /** Installed app ID (e.g., 'elohim-lamad') */
  appId: InstalledAppId;

  /** Path to hApp file (for installation) */
  happPath?: string;

  /** API key for admin proxy authentication (if using proxy) */
  proxyApiKey?: string;
}

/**
 * Stored signing credentials for browser persistence
 */
export interface StoredSigningCredentials {
  /** Capability secret for signing */
  capSecret: Uint8Array;

  /** Key pair for signing (private + public) */
  keyPair: {
    publicKey: Uint8Array;
    privateKey: Uint8Array;
  };

  /** Signing key (public) */
  signingKey: Uint8Array;
}

/**
 * Connection context with WebSocket handles
 */
export interface HolochainConnection {
  /** Current connection state */
  state: HolochainConnectionState;

  /** Admin WebSocket (for conductor management) */
  adminWs: AdminWebsocket | null;

  /** App WebSocket (for zome calls) */
  appWs: AppWebsocket | null;

  /** Current agent's public key */
  agentPubKey: AgentPubKey | null;

  /** Cell ID for the installed app */
  cellId: CellId | null;

  /** App info after installation */
  appInfo: AppInfo | null;

  /** Error message if state is 'error' */
  error?: string;

  /** When connection was established */
  connectedAt?: Date;
}

/**
 * Result of a zome call
 */
export interface ZomeCallResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

/**
 * Input for making a zome call
 */
export interface ZomeCallInput {
  /** Zome name (e.g., 'content_store') */
  zomeName: string;

  /** Function name (e.g., 'create_content') */
  fnName: string;

  /** Payload to send */
  payload: unknown;
}

/**
 * Content entry from the spike DNA
 */
export interface HolochainContent {
  id: string;
  title: string;
  body: string;
  created_at: string;
  author: string;
}

/**
 * Output from get_content zome call
 */
export interface HolochainContentOutput {
  action_hash: Uint8Array;
  content: HolochainContent;
}

/**
 * Default configuration from environment
 */
export const DEFAULT_HOLOCHAIN_CONFIG: HolochainConfig = {
  adminUrl: holochainConfig?.adminUrl ?? 'ws://localhost:4444',
  appUrl: holochainConfig?.appUrl ?? 'ws://localhost:4445',
  origin: 'elohim-app',
  appId: 'lamad-spike',
  happPath: '/opt/holochain/lamad-spike.happ',
  proxyApiKey: holochainConfig?.proxyApiKey,
};

/**
 * LocalStorage key for signing credentials
 */
export const SIGNING_CREDENTIALS_KEY = 'holochain-signing-credentials';

/**
 * Initial disconnected state
 */
export const INITIAL_CONNECTION_STATE: HolochainConnection = {
  state: 'disconnected',
  adminWs: null,
  appWs: null,
  agentPubKey: null,
  cellId: null,
  appInfo: null,
};

/**
 * Display-friendly connection information for UI rendering
 * Used by the Edge Node settings section in the profile tray
 */
export interface EdgeNodeDisplayInfo {
  /** Current connection state */
  state: HolochainConnectionState;

  /** Admin WebSocket URL from config */
  adminUrl: string;

  /** App WebSocket URL from config */
  appUrl: string;

  /** Agent public key (full base64 encoded) */
  agentPubKey: string | null;

  /** Cell ID display [DnaHash, AgentPubKey] */
  cellId: { dnaHash: string; agentPubKey: string } | null;

  /** App ID from config */
  appId: string;

  /** DNA hash extracted from cell ID */
  dnaHash: string | null;

  /** When connection was established */
  connectedAt: Date | null;

  /** Whether signing credentials are stored in localStorage */
  hasStoredCredentials: boolean;

  /** Network seed if available from appInfo */
  networkSeed: string | null;

  /** Error message if in error state */
  error: string | null;
}
