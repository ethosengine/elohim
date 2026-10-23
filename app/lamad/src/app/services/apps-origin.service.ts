import { DOCUMENT } from '@angular/common';
import { Injectable, inject, signal } from '@angular/core';

import type { AuthDiscovery } from '../../generated/auth-discovery';

/**
 * The discovery document as the doorway answers it on an apps-paired name.
 *
 * `apps: true` is added by the doorway (`routes/auth_discovery.rs`) only when
 * the public name is paired; it is omitted otherwise. The document names no
 * origin — the client derives it. Delete this intersection once the generated
 * AuthDiscovery carries the field.
 */
type DiscoveryWithApps = Partial<AuthDiscovery> & { apps?: unknown };

/** Where the page asks its own doorway which origin serves raw EPR content. */
export const AUTH_DISCOVERY_PATH = '/.well-known/elohim-auth';

const SESSION_KEY_PREFIX = 'elohim-apps-origin:';
const READ_TIMEOUT_MS = 3000;

/**
 * AppsOriginService — which origin a wrapper frames raw `/apps/<slug>/<entry>`
 * bytes from.
 *
 * On an apps-paired public name (`elohim.localhost:8888` ↔
 * `apps.elohim.localhost:8888`) the raw bytes live on the apps origin, so the
 * chrome and the person's sign-in stay on a different origin from any
 * content. A wrapper (lamad's viewer, the EPR page's focal view) frames the
 * apps origin's URL; the doorway serves raw bytes to a same-site frame, so the
 * wrapper shows the bare content with no second chrome.
 *
 * UX-surface only: this decides which URL to put in a frame. Pairing itself is
 * the doorway's fact, read from `GET /.well-known/elohim-auth` once per browser
 * session (cached in sessionStorage per page origin, so the shell and the
 * lamad bundle share one read). When the document carries `apps: true`, the
 * apps origin is derived by the protocol rule: this page's scheme, `apps.` +
 * this page's own host and port. When the field is absent or there is no
 * document — an unpaired name, peer-native storage, Tauri, or a doorway that
 * predates the field — `null` keeps today's same-origin `/apps/...` URLs.
 * The document can only switch pairing on or off; it cannot name an origin.
 */
@Injectable({ providedIn: 'root' })
export class AppsOriginService {
  private readonly document = inject(DOCUMENT);

  /** `undefined` = not read yet; `null` = frame same-origin; string = apps origin. */
  private readonly known = signal<string | null | undefined>(this.readSessionCache());
  readonly appsOrigin = this.known.asReadonly();

  private pending: Promise<string | null> | null = null;

  /** The apps origin, reading discovery at most once per session. */
  resolve(): Promise<string | null> {
    const cached = this.known();
    if (cached !== undefined) return Promise.resolve(cached);
    this.pending ??= this.readDiscovery().then(origin => {
      this.known.set(origin);
      this.writeSessionCache(origin);
      return origin;
    });
    return this.pending;
  }

  /**
   * The URL to frame for an app bundle's entry: on the apps origin when the
   * name is paired, else same-origin relative.
   */
  appUrl(slug: string, entryPoint: string, appsOrigin: string | null): string {
    return `${appsOrigin ?? ''}/apps/${slug}/${entryPoint}`;
  }

  private async readDiscovery(): Promise<string | null> {
    const win = this.document.defaultView;
    if (!win || typeof win.fetch !== 'function') return null;
    const controller = new AbortController();
    const timer = win.setTimeout(() => controller.abort(), READ_TIMEOUT_MS);
    try {
      const resp = await win.fetch(AUTH_DISCOVERY_PATH, {
        credentials: 'omit',
        signal: controller.signal,
      });
      if (!resp.ok) return null;
      const doc = (await resp.json()) as DiscoveryWithApps | null;
      return doc?.apps === true ? this.derivedAppsOrigin() : null;
    } catch {
      return null;
    } finally {
      win.clearTimeout(timer);
    }
  }

  /** The protocol rule: same scheme, `apps.` + this page's own host and port. */
  private derivedAppsOrigin(): string | null {
    const here = this.document.defaultView?.location;
    return here ? `${here.protocol}//apps.${here.host}` : null;
  }

  private sessionKey(): string | null {
    const origin = this.document.defaultView?.location?.origin;
    return origin ? SESSION_KEY_PREFIX + origin : null;
  }

  private readSessionCache(): string | null | undefined {
    const key = this.sessionKey();
    if (!key) return undefined;
    try {
      const stored = this.document.defaultView?.sessionStorage.getItem(key);
      if (stored === null || stored === undefined) return undefined;
      // '1' records "paired", '0' "read, and not paired".
      if (stored === '1') return this.derivedAppsOrigin();
      return stored === '0' ? null : undefined;
    } catch {
      return undefined;
    }
  }

  private writeSessionCache(origin: string | null): void {
    const key = this.sessionKey();
    if (!key) return;
    try {
      this.document.defaultView?.sessionStorage.setItem(key, origin ? '1' : '0');
    } catch {
      // Storage unavailable (private mode, quota): the read repeats next page load.
    }
  }
}
