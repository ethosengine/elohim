/**
 * HolochainSourceChainService — Thin HTTP read client for agent source-chain endpoints.
 *
 * M-AGGR-2: Holochain source-chain cutover.
 *
 * Replaces the 595-line `LocalSourceChainService` localStorage simulation.
 * The substrate (imagodei DNA) is the source of truth for agent source-chain
 * data. This service is the read path: it wraps the two doorway routes:
 *
 *   GET /api/v1/source-chain/{agentId}/entries  → SourceChainEntryView[]
 *   GET /api/v1/source-chain/{agentId}/links    → EntryLinkView[]
 *
 * Write paths (createEntry, createLink) are substrate-native coordinator
 * calls. They migrate to dedicated services as M-REA-1 and M-AGGR-1 land.
 *
 * Source of truth: Holochain agent source chain in imagodei DNA (Category B,
 * agent-scoped private source chain — entries never gossiped to DHT).
 * Reads are proxied through elohim-storage → imagodei conductor.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { firstValueFrom, of } from 'rxjs';
import { catchError } from 'rxjs/operators';

import type { SourceChainEntryView, EntryLinkView } from '@elohim/storage-client/generated';

@Injectable({ providedIn: 'root' })
export class HolochainSourceChainService {
  private readonly http = inject(HttpClient);

  /**
   * Query all source-chain entries for the given agent.
   *
   * Wraps `GET /api/v1/source-chain/{agentId}/entries`.
   * Returns an empty array on error (graceful degradation when conductor is
   * unavailable or the agent has no entries yet).
   *
   * Entries are returned in ascending sequence order (chain order, seq 0 first).
   *
   * @param agentId - Hex-encoded AgentPubKey
   */
  async getEntries(agentId: string): Promise<SourceChainEntryView[]> {
    return firstValueFrom(
      this.http
        .get<SourceChainEntryView[]>(`/api/v1/source-chain/${encodeURIComponent(agentId)}/entries`)
        .pipe(catchError(() => of([])))
    );
  }

  /**
   * Query all source-chain links authored by the given agent.
   *
   * Wraps `GET /api/v1/source-chain/{agentId}/links`.
   * Returns an empty array on error (graceful degradation).
   *
   * @param agentId - Hex-encoded AgentPubKey
   */
  async getLinks(agentId: string): Promise<EntryLinkView[]> {
    return firstValueFrom(
      this.http
        .get<EntryLinkView[]>(`/api/v1/source-chain/${encodeURIComponent(agentId)}/links`)
        .pipe(catchError(() => of([])))
    );
  }

  /**
   * Filter entries returned from `getEntries` by entry type label.
   *
   * The `entryType` field on `SourceChainEntryView` is a human-readable label
   * produced by the coordinator (e.g. `"Dna"`, `"AgentValidationPkg"`,
   * `"Create:App(0)"`). This helper is a pure client-side filter over an
   * already-fetched slice — it does NOT make a second network call.
   *
   * @param entries - Slice already retrieved via `getEntries`
   * @param entryTypeLabel - Label prefix or exact match to filter on
   */
  filterByType(entries: SourceChainEntryView[], entryTypeLabel: string): SourceChainEntryView[] {
    return entries.filter(e => e.entryType.startsWith(entryTypeLabel));
  }
}
