import { Injectable, signal } from '@angular/core';

const STORAGE_KEY = 'elohim.session-nav-stack.v1';
const MAX_ENTRIES = 32;

export interface NavStackEntry {
  url: string;
  cid: string;
  label?: string;
  ts: number;
}

/**
 * Visitor's in-protocol route history. sessionStorage-backed.
 * Powers the back affordance on ProtocolOmniComponent when the visitor
 * walked through the protocol to get here; empty on cold anonymous hit.
 */
@Injectable({ providedIn: 'root' })
export class SessionNavStackService {
  private readonly _stack = signal<NavStackEntry[]>([]);

  readonly entries = this._stack.asReadonly();
  readonly length = (): number => this._stack().length;
  /**
   * The entry to navigate back to. Only defined when the stack has 2+
   * entries — returns the second-to-last (the stop before the current one).
   * Returns null for 0 or 1 entries so nav.back() falls through to the
   * EPR-derived substrate signal rather than looping to the current page.
   */
  readonly previous = (): NavStackEntry | null => {
    const s = this._stack();
    return s.length >= 2 ? (s[s.length - 2] ?? null) : null;
  };

  constructor() {
    this.hydrate();
  }

  record(entry: Omit<NavStackEntry, 'ts'>): void {
    const stamped: NavStackEntry = { ...entry, ts: Date.now() };
    const current = this._stack();
    const top = current.at(-1);
    if (top?.url === stamped.url && top?.cid === stamped.cid) return;
    const next = [...current, stamped].slice(-MAX_ENTRIES);
    this._stack.set(next);
    this.persist(next);
  }

  pop(): NavStackEntry | null {
    const current = this._stack();
    if (current.length === 0) return null;
    const top = current.at(-1) ?? null;
    const next = current.slice(0, -1);
    this._stack.set(next);
    this.persist(next);
    return top;
  }

  clear(): void {
    this._stack.set([]);
    this.persist([]);
  }

  private hydrate(): void {
    try {
      const raw = sessionStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as NavStackEntry[];
      if (Array.isArray(parsed)) this._stack.set(parsed);
    } catch {
      // sessionStorage unavailable or corrupted — start empty
    }
  }

  private persist(stack: NavStackEntry[]): void {
    try {
      sessionStorage.setItem(STORAGE_KEY, JSON.stringify(stack));
    } catch {
      // quota / privacy mode — best-effort persistence
    }
  }
}
