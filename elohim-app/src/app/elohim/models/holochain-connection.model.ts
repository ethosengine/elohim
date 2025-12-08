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
  /** Admin WebSocket URL (e.g., ws://localhost:4444) */
  adminUrl: string;

  /** App WebSocket URL (e.g., ws://localhost:4445) */
  appUrl: string;

  /** Origin for CORS (must match conductor config) */
  origin: string;

  /** Installed app ID (e.g., 'elohim-lamad') */
  appId: InstalledAppId;

  /** Path to hApp file (for installation) */
  happPath?: string;
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
  adminUrl: environment.holochain?.adminUrl ?? 'ws://localhost:4444',
  appUrl: environment.holochain?.appUrl ?? 'ws://localhost:4445',
  origin: 'elohim-app',
  appId: 'lamad-spike',
  happPath: '/opt/holochain/lamad-spike.happ',
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
