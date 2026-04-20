import { Injectable } from '@angular/core';

/**
 * Bridge between Angular app and the apps Service Worker.
 * Framework-agnostic BroadcastChannel — works in any browser context.
 */
@Injectable({ providedIn: 'root' })
export class SwBridgeService {
  private readonly channel: BroadcastChannel | null =
    typeof BroadcastChannel === 'undefined' ? null : new BroadcastChannel('apps-sw');

  /** Tell the SW to evict all cached files for an app */
  invalidateApp(slug: string): void {
    this.channel?.postMessage({ type: 'invalidate', slug });
  }
}
