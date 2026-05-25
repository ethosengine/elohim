/**
 * Session — reactive identity/capability state for the current browser.
 *
 * Reads the elohim_session cookie set by doorway-side auth flows.
 * Pure TS; no DOM rendering. Elements that need session state
 * subscribe and re-render on change.
 *
 * Cookie shape (set by doorway after successful imagodei auth):
 *   elohim_session={"humanId":"...","capabilities":[...],"reach":"..."}
 *
 * In practice the cookie is signed/encrypted by doorway; this class
 * parses the public JSON portion. Verification of authenticity is
 * doorway's responsibility — by the time the cookie reaches us, it
 * is trusted within the browser's origin boundary.
 */

export interface CurrentUserView {
  humanId: string;
  capabilities: string[];
  reach: string;
}

type Subscriber = (user: CurrentUserView | null) => void;

export class Session {
  private _currentUser: CurrentUserView | null = null;
  private subscribers = new Set<Subscriber>();

  constructor() {
    this.refreshFromCookies();
  }

  get currentUser(): CurrentUserView | null {
    return this._currentUser;
  }

  get isAuthenticated(): boolean {
    return this._currentUser !== null;
  }

  /**
   * Re-read the elohim_session cookie and notify subscribers if the
   * resolved CurrentUserView changes. Call this after any auth-related
   * event (login completed, logout, cookie set externally).
   */
  refreshFromCookies(): void {
    const prev = this._currentUser;
    const cookie = this.readCookie('elohim_session');
    if (!cookie) {
      this._currentUser = null;
    } else {
      try {
        this._currentUser = JSON.parse(decodeURIComponent(cookie)) as CurrentUserView;
      } catch {
        this._currentUser = null;
      }
    }
    if (this.shallowEquals(prev, this._currentUser)) return;
    this.subscribers.forEach((s) => s(this._currentUser));
  }

  /** Subscribe to session changes. Returns an unsubscribe function. */
  subscribe(fn: Subscriber): () => void {
    this.subscribers.add(fn);
    return () => {
      this.subscribers.delete(fn);
    };
  }

  private readCookie(name: string): string | null {
    if (typeof document === 'undefined') return null;
    const all = document.cookie.split(';').map((c) => c.trim());
    const found = all.find((c) => c.startsWith(`${name}=`));
    return found ? found.slice(name.length + 1) : null;
  }

  private shallowEquals(a: CurrentUserView | null, b: CurrentUserView | null): boolean {
    if (a === b) return true;
    if (!a || !b) return false;
    return (
      a.humanId === b.humanId &&
      a.reach === b.reach &&
      a.capabilities.length === b.capabilities.length &&
      a.capabilities.every((cap, i) => cap === b.capabilities[i])
    );
  }
}
