import { Injectable, computed, signal } from '@angular/core';
import { detectConnectionMode } from '@elohim/service/connection';
import { environment } from '../../../environments/environment';

/** Active deployment-context descriptor for the debug surface. The single place
 *  lenses consult to source data per context and degrade honestly. */
@Injectable({ providedIn: 'root' })
export class DebugContextService {
  /** 'doorway' (web→doorway) | 'tauri' (native) | 'direct' (CLI/node). */
  readonly mode = signal(detectConnectionMode());

  /** True in the native desktop shell. */
  readonly isTauri = computed(() => this.mode() === 'tauri');

  /** True when this context talks to elohim-storage directly (no doorway). */
  readonly isDirectStorage = computed(() => this.mode() !== 'doorway');

  /** Environment name (development / alpha / production). */
  readonly environmentName = environment.environment;

  /**
   * Base URL for HTTP reads:
   *  - doorway: '' (same-origin; in dev the proxy forwards to :8888) — used for /admin/*.
   *  - tauri/direct: the elohim-storage sidecar at :8090 (per elohim-storage/CLAUDE.md
   *    "Tauri/Direct … localhost:8090 … same HTTP routes as the proxied path") — used for
   *    /api/v1/status/* and /p2p/status.
   */
  readonly storageBaseUrl = computed(() =>
    this.mode() === 'doorway' ? (environment.doorwayUrl ?? '') : 'http://localhost:8090'
  );
}
