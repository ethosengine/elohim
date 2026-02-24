/**
 * Instrument Loader Service - Bridges ContentService and the instrument registry.
 *
 * Loads InstrumentDefinition content nodes from the content pipeline and
 * registers them in the instrument registry. Static instrument files serve
 * as fallbacks when content pipeline is unavailable (dev, offline).
 *
 * Flow: ContentService -> InstrumentDefinition parse -> registerFromDefinition()
 */

import { Injectable, inject, signal, computed } from '@angular/core';

// @coverage: 100.0% (2026-02-24)

import { firstValueFrom } from 'rxjs';

import { ContentService } from '@app/elohim/services/content.service';

import {
  registerFromDefinition,
  getRegisteredInstrumentIds,
} from '../instruments/instrument-registry';

import type { InstrumentDefinition } from '../models/instrument-definition.model';
import type { ContentNode } from '@app/lamad/models/content-node.model';

// ─────────────────────────────────────────────────────────────────────────────
// Service
// ─────────────────────────────────────────────────────────────────────────────

@Injectable({ providedIn: 'root' })
export class InstrumentLoaderService {
  private readonly contentService = inject(ContentService);

  private readonly loadingSignal = signal(false);
  private readonly loadedSignal = signal(false);
  private readonly errorSignal = signal<string | null>(null);

  /** Whether instruments are currently being loaded */
  readonly loading = this.loadingSignal.asReadonly();

  /** Whether instruments have been loaded from the content pipeline */
  readonly loaded = this.loadedSignal.asReadonly();

  /** Error message if loading failed */
  readonly error = this.errorSignal.asReadonly();

  /** Number of registered instruments (includes static fallbacks) */
  readonly instrumentCount = computed(() => getRegisteredInstrumentIds().length);

  // ---------------------------------------------------------------------------
  // Public Methods
  // ---------------------------------------------------------------------------

  /**
   * Load all instrument definitions from the content pipeline.
   * Instruments from the pipeline overwrite static fallbacks with matching IDs.
   */
  async loadInstruments(): Promise<number> {
    if (this.loadingSignal()) return 0;

    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      const nodes = await firstValueFrom(
        this.contentService.queryContent({ contentType: 'instrument' })
      );

      let loaded = 0;
      for (const node of nodes) {
        const def = this.parseInstrumentNode(node);
        if (def) {
          registerFromDefinition(def);
          loaded++;
        }
      }

      this.loadedSignal.set(true);
      return loaded;
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      this.errorSignal.set(message);
      return 0;
    } finally {
      this.loadingSignal.set(false);
    }
  }

  /**
   * Load a single instrument by content node ID.
   */
  async loadInstrument(nodeId: string): Promise<boolean> {
    try {
      const node = await firstValueFrom(this.contentService.getContent(nodeId));
      if (!node) return false;

      const def = this.parseInstrumentNode(node);
      if (!def) return false;

      registerFromDefinition(def);
      return true;
    } catch {
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Private Helpers
  // ---------------------------------------------------------------------------

  private parseInstrumentNode(node: ContentNode): InstrumentDefinition | null {
    if (node.contentType !== 'instrument') return null;

    try {
      const raw = node.content;
      const body: unknown = typeof raw === 'string' ? (JSON.parse(raw) as unknown) : raw;

      if (!this.isValidInstrumentBody(body)) return null;

      return body;
    } catch {
      return null;
    }
  }

  private isValidInstrumentBody(body: unknown): body is InstrumentDefinition {
    if (body === null || body === undefined || typeof body !== 'object') return false;
    const obj = body as Record<string, unknown>;
    return !!(
      obj['id'] &&
      obj['name'] &&
      obj['subscales'] &&
      obj['scoringConfig'] &&
      obj['display']
    );
  }
}
