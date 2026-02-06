/**
 * HTML5 App Plugin - Type Definitions
 *
 * Framework-agnostic types for HTML5 app loading and serving.
 */

// =============================================================================
// Configuration
// =============================================================================

/**
 * Configuration for Html5AppEngine.
 */
export interface Html5AppConfig {
  /** Path to Service Worker script. Default: '/html5-app-sw.js' */
  swPath?: string;

  /** IndexedDB database name. Default: 'elohim-html5-apps' */
  dbName?: string;

  /** URL prefix for app routes. Default: '/apps' */
  urlPrefix?: string;

  /** Maximum cache size in MB. Default: 500 */
  maxCacheSizeMB?: number;

  /** Enable debug logging. Default: false */
  debug?: boolean;
}

/**
 * Default configuration values.
 */
export const DEFAULT_CONFIG: Required<Html5AppConfig> = {
  swPath: '/html5-app-sw.js',
  dbName: 'elohim-html5-apps',
  urlPrefix: '/apps',
  maxCacheSizeMB: 500,
  debug: false,
};

// =============================================================================
// App Metadata
// =============================================================================

/**
 * Information about a loaded HTML5 app.
 */
export interface Html5AppInfo {
  /** Unique app identifier */
  appId: string;

  /** Entry point file (e.g., 'index.html') */
  entryPoint: string;

  /** List of all files in the app */
  files: string[];

  /** Total size in bytes */
  sizeBytes: number;

  /** Unix timestamp when loaded */
  loadedAt: number;

  /** SHA256 hash of the source zip blob */
  blobHash?: string;

  /** App manifest if present */
  manifest?: Html5AppManifest;
}

/**
 * Optional manifest file (elohim-app.json) in the zip.
 */
export interface Html5AppManifest {
  /** App name */
  name?: string;

  /** App version */
  version?: string;

  /** Entry point override */
  entryPoint?: string;

  /** Required capabilities */
  capabilities?: string[];

  /** Content Security Policy */
  csp?: string;

  /** Sandbox flags for iframe */
  sandbox?: string[];

  /** Author info */
  author?: {
    name?: string;
    url?: string;
  };

  /** License */
  license?: string;
}

// =============================================================================
// Storage Types
// =============================================================================

/**
 * A single file stored in IndexedDB.
 */
export interface StoredFile {
  /** App ID this file belongs to */
  appId: string;

  /** File path within the app */
  path: string;

  /** File content as ArrayBuffer */
  data: ArrayBuffer;

  /** MIME type */
  mimeType: string;

  /** File size in bytes */
  sizeBytes: number;

  /** Unix timestamp when stored */
  storedAt: number;
}

/**
 * App metadata stored in IndexedDB.
 */
export interface StoredAppMeta {
  /** App ID */
  appId: string;

  /** App info */
  info: Html5AppInfo;

  /** Unix timestamp when stored */
  storedAt: number;

  /** Last accessed timestamp */
  lastAccessedAt: number;
}

// =============================================================================
// Service Worker Messages
// =============================================================================

/**
 * Message types for main thread <-> Service Worker communication.
 */
export type SwMessageType =
  | 'REGISTER_APP'
  | 'UNREGISTER_APP'
  | 'GET_LOADED_APPS'
  | 'CLEAR_ALL'
  | 'PING';

/**
 * Message from main thread to Service Worker.
 */
export interface SwMessage {
  type: SwMessageType;
  payload?: unknown;
  messageId?: string;
}

/**
 * Register app message payload.
 */
export interface RegisterAppPayload {
  appId: string;
  entryPoint: string;
  files: string[];
}

/**
 * Response from Service Worker to main thread.
 */
export interface SwResponse {
  type: 'SUCCESS' | 'ERROR';
  messageId?: string;
  payload?: unknown;
  error?: string;
}

// =============================================================================
// Events
// =============================================================================

/**
 * Event types emitted by Html5AppEngine.
 */
export type Html5AppEventType = 'load' | 'unload' | 'error' | 'ready';

/**
 * Event payload for 'load' event.
 */
export interface LoadEvent {
  type: 'load';
  appId: string;
  info: Html5AppInfo;
}

/**
 * Event payload for 'unload' event.
 */
export interface UnloadEvent {
  type: 'unload';
  appId: string;
}

/**
 * Event payload for 'error' event.
 */
export interface ErrorEvent {
  type: 'error';
  appId?: string;
  error: Error;
}

/**
 * Event payload for 'ready' event.
 */
export interface ReadyEvent {
  type: 'ready';
  swActive: boolean;
}

/**
 * Union of all event types.
 */
export type Html5AppEvent = LoadEvent | UnloadEvent | ErrorEvent | ReadyEvent;

// =============================================================================
// MIME Types
// =============================================================================

/**
 * Common MIME types for web content.
 */
export const MIME_TYPES: Record<string, string> = {
  // HTML
  '.html': 'text/html',
  '.htm': 'text/html',

  // CSS
  '.css': 'text/css',

  // JavaScript
  '.js': 'application/javascript',
  '.mjs': 'application/javascript',

  // JSON
  '.json': 'application/json',

  // Images
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon',

  // Fonts
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.eot': 'application/vnd.ms-fontobject',

  // Audio
  '.mp3': 'audio/mpeg',
  '.wav': 'audio/wav',
  '.ogg': 'audio/ogg',

  // Video
  '.mp4': 'video/mp4',
  '.webm': 'video/webm',

  // Other
  '.xml': 'application/xml',
  '.txt': 'text/plain',
  '.md': 'text/markdown',
  '.wasm': 'application/wasm',
};

/**
 * Get MIME type for a file path.
 */
export function getMimeType(filePath: string): string {
  const ext = filePath.substring(filePath.lastIndexOf('.')).toLowerCase();
  return MIME_TYPES[ext] || 'application/octet-stream';
}
