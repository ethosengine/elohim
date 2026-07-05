import { Injectable, isDevMode, signal } from '@angular/core';

const KEY = 'elohim-debug';

/** Controls whether the /debug NAV entry is shown. The /debug route itself always
 *  resolves by URL (chrome://flags model) — this gates discoverability only, not
 *  access. Sticky via localStorage so a flip survives reload. */
@Injectable({ providedIn: 'root' })
export class DebugModeService {
  private readonly sticky = signal(this.readSticky());

  /** The persisted (sticky) "pin debug to nav" preference, distinct from
   *  isDevMode(). Read by the shell's pin toggle so deployed-build users who
   *  reach /debug by URL can make the nav entry stick. */
  readonly pinned = this.sticky.asReadonly();

  /** Nav entry visible when dev-mode OR the user pinned it (sticky flag). */
  readonly navVisible = () => isDevMode() || this.sticky();

  enable(): void {
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      localStorage.setItem(KEY, 'on');
    } catch {
      /* storage unavailable */
    }
    this.sticky.set(true);
  }

  disable(): void {
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      localStorage.removeItem(KEY);
    } catch {
      /* storage unavailable */
    }
    this.sticky.set(false);
  }

  private readSticky(): boolean {
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      return localStorage.getItem(KEY) === 'on';
    } catch {
      return false;
    }
  }
}
