/**
 * Ambient type declarations for browser APIs in dual-environment library.
 *
 * elohim-service targets both Node.js (CLI) and browser (Angular).
 * The tsconfig uses lib: ["ES2022"] without "DOM" to keep Node-side
 * code free of browser-only types. These declarations cover the
 * minimal browser surface used by connection strategies.
 *
 * At runtime, connection strategies guard every access with existence
 * checks (e.g. `if (globalThis.window !== undefined)`).
 */

/* eslint-disable no-var */

/** Minimal Location — subset of DOM Location used for URL inspection */
interface BrowserLocation {
  hostname: string;
  href: string;
  origin: string;
  protocol: string;
}

/** Minimal Window — subset of DOM Window used for environment detection */
interface BrowserWindow {
  location: BrowserLocation;
}

/** Minimal Storage — subset of DOM Storage used for token persistence */
interface BrowserStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

// Browser globals — may be undefined in Node.js environments.
// Connection strategies check existence before use.
declare var window: BrowserWindow | undefined;
declare var location: BrowserLocation | undefined;
declare var localStorage: BrowserStorage | undefined;

// Tauri runtime injection
declare var __TAURI__: Record<string, unknown> | undefined;
