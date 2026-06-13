import { Injectable, isDevMode, signal } from '@angular/core';

const KEY = 'elohim-debug';

/** Controls whether the /debug NAV entry is shown. The /debug route itself always
 *  resolves by URL (chrome://flags model) — this gates discoverability only, not
 *  access. Sticky via localStorage so a flip survives reload. */
@Injectable({ providedIn: 'root' })
export class DebugModeService {
  private readonly sticky = signal(this.readSticky());

  /** Nav entry visible when dev-mode OR the user flipped the sticky flag. */
  readonly navVisible = () => isDevMode() || this.sticky();

  enable(): void {
    try {
      localStorage.setItem(KEY, 'on');
    } catch {
      /* storage unavailable */
    }
    this.sticky.set(true);
  }

  disable(): void {
    try {
      localStorage.removeItem(KEY);
    } catch {
      /* storage unavailable */
    }
    this.sticky.set(false);
  }

  private readSticky(): boolean {
    try {
      return localStorage.getItem(KEY) === 'on';
    } catch {
      return false;
    }
  }
}
