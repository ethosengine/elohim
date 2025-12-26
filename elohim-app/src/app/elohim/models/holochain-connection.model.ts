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

  /** Use local dev-proxy for Che environment (auto-detected, but can be forced) */
  useLocalProxy?: boolean;
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

// =============================================================================
// Content Entry Types (Extended Schema - matches Rust DNA structures)
// =============================================================================

/**
 * Content entry as stored in Holochain
 * Matches Content struct in integrity zome (extended schema)
 */
export interface HolochainContent {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  author_id: string | null;
  reach: string;
  trust_score: number;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/**
 * Output from content retrieval zome calls
 */
export interface HolochainContentOutput {
  action_hash: Uint8Array;
  entry_hash: Uint8Array;
  content: HolochainContent;
}

/**
 * Input for creating content via zome call
 * Matches CreateContentInput in coordinator zome
 */
export interface CreateContentInput {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  metadata_json: string;
}

/**
 * Input for bulk content creation
 * Matches BulkCreateContentInput in coordinator zome
 */
export interface BulkCreateContentInput {
  import_id: string;
  contents: CreateContentInput[];
}

/**
 * Output from bulk content creation
 * Matches BulkCreateContentOutput in coordinator zome
 */
export interface BulkCreateContentOutput {
  import_id: string;
  created_count: number;
  action_hashes: Uint8Array[];
  errors: string[];
}

// =============================================================================
// Query Types
// =============================================================================

/**
 * Input for querying content by type
 */
export interface QueryByTypeInput {
  content_type: string;
  limit?: number;
}

/**
 * Input for querying content by ID
 */
export interface QueryByIdInput {
  id: string;
}

/**
 * Content statistics from get_content_stats
 */
export interface ContentStats {
  total_count: number;
  by_type: Record<string, number>;
}

// =============================================================================
// Learning Path Types
// =============================================================================

/**
 * Input for creating a learning path
 */
export interface CreatePathInput {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose?: string;
  difficulty: string;
  estimated_duration?: string;
  visibility: string;
  path_type: string;
  tags: string[];
}

/**
 * Input for adding a step to a path
 */
export interface AddPathStepInput {
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title?: string;
  step_narrative?: string;
  is_optional: boolean;
}

/**
 * Learning path as stored in Holochain
 */
export interface HolochainLearningPath {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  created_by: string;
  difficulty: string;
  estimated_duration: string | null;
  visibility: string;
  path_type: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

/**
 * Path step as stored in Holochain
 */
export interface HolochainPathStep {
  id: string;
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title: string | null;
  step_narrative: string | null;
  is_optional: boolean;
}

/**
 * Output for path step retrieval
 */
export interface PathStepOutput {
  action_hash: Uint8Array;
  step: HolochainPathStep;
}

/**
 * Output for path with steps retrieval
 */
export interface PathWithSteps {
  action_hash: Uint8Array;
  path: HolochainLearningPath;
  steps: PathStepOutput[];
}

// =============================================================================
// Relationship Types
// =============================================================================

/**
 * Content relationship as stored in Holochain
 */
export interface HolochainContentRelationship {
  id: string;
  source_node_id: string;
  target_node_id: string;
  relationship_type: string;
  confidence: number;
  metadata_json: string | null;
}

// =============================================================================
// Constants
// =============================================================================

/** Valid content types matching Rust validation */
export const VALID_CONTENT_TYPES = [
  'source',
  'epic',
  'feature',
  'scenario',
  'concept',
  'role',
  'video',
  'organization',
  'book-chapter',
  'tool',
  'path',
  'assessment',
  'reference',
  'example',
] as const;

/** Valid content formats matching Rust validation */
export const VALID_CONTENT_FORMATS = [
  'markdown',
  'gherkin',
  'html',
  'plaintext',
  'video-embed',
  'external-link',
  'quiz-json',
  'assessment-json',
] as const;

/** Valid reach levels matching Rust validation */
export const VALID_REACH_LEVELS = [
  'private',
  'invited',
  'local',
  'community',
  'federated',
  'commons',
] as const;

/** Valid relationship types matching Rust validation */
export const VALID_RELATIONSHIP_TYPES = [
  'CONTAINS',
  'BELONGS_TO',
  'DESCRIBES',
  'IMPLEMENTS',
  'VALIDATES',
  'RELATES_TO',
  'REFERENCES',
  'DEPENDS_ON',
  'REQUIRES',
  'FOLLOWS',
  'DERIVED_FROM',
  'SOURCE_OF',
] as const;

/** Valid difficulty levels for learning paths */
export const VALID_DIFFICULTY_LEVELS = [
  'beginner',
  'intermediate',
  'advanced',
] as const;

// Type aliases from constants
export type ValidContentType = typeof VALID_CONTENT_TYPES[number];
export type ValidContentFormat = typeof VALID_CONTENT_FORMATS[number];
export type ValidReachLevel = typeof VALID_REACH_LEVELS[number];
export type ValidRelationshipType = typeof VALID_RELATIONSHIP_TYPES[number];
export type ValidDifficultyLevel = typeof VALID_DIFFICULTY_LEVELS[number];

/**
 * Default configuration from environment
 */
export const DEFAULT_HOLOCHAIN_CONFIG: HolochainConfig = {
  adminUrl: holochainConfig?.adminUrl ?? 'ws://localhost:4444',
  appUrl: holochainConfig?.appUrl ?? 'ws://localhost:4445',
  origin: 'elohim-app',
  appId: 'elohim',
  happPath: '/opt/holochain/elohim.happ',
  proxyApiKey: holochainConfig?.proxyApiKey,
  useLocalProxy: holochainConfig?.useLocalProxy ?? true, // Auto-detect Che and use dev-proxy
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
