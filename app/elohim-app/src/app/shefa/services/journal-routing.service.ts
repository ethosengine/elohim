import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { Observable } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

import type {
  JournalRoutingState,
  IntentAnalysis,
  RoutingSuggestion,
} from '../models/journal-routing.model';

/**
 * JournalRoutingService — state machine for the journal routing flow.
 *
 * When a human finishes writing a journal entry, this service:
 *   1. Analyzes intent (Round 1 — POST to /api/v1/journal/analyze)
 *   2. Generates routing suggestions (Round 2 — POST to /api/v1/journal/suggest)
 *   3. Posts or dismisses each card
 *
 * State machine: writing -> confirming -> routing -> routed
 *   with edit() returning to writing from confirming or routing.
 *
 * Component-scoped — no providedIn, provided by host component.
 */
@Injectable()
export class JournalRoutingService {
  private readonly http = inject(HttpClient);
  private readonly storageApi = inject(StorageApiService);

  // ---------------------------------------------------------------------------
  // Internal state
  // ---------------------------------------------------------------------------

  private readonly _state = signal<JournalRoutingState>('writing');
  private readonly _intentSummary = signal('');
  private readonly _suggestions = signal<RoutingSuggestion[]>([]);
  private readonly _journalText = signal('');
  private readonly _contentId = signal('');
  private readonly _intentAnalysis = signal<IntentAnalysis | null>(null);

  // ---------------------------------------------------------------------------
  // Public signals
  // ---------------------------------------------------------------------------

  readonly state = this._state.asReadonly();
  readonly intentSummary = this._intentSummary.asReadonly();
  readonly suggestions = this._suggestions.asReadonly();
  readonly journalText = this._journalText.asReadonly();

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /** Set the journal content ID for posting (used in PATCH for filing). */
  setContentId(id: string): void {
    this._contentId.set(id);
  }

  /** Round 1: analyze the journal text for intent, transition to confirming. */
  finish(text: string): void {
    this._journalText.set(text);

    this.analyzeIntent(text).subscribe(analysis => {
      this._intentAnalysis.set(analysis);
      this._intentSummary.set(analysis.summary);
      this._state.set('confirming');
    });
  }

  /** Round 2: generate suggestions from analyzed intent, transition to routing. */
  confirm(): void {
    const text = this._journalText();
    const analysis = this._intentAnalysis();
    if (!analysis) return;

    this.generateSuggestions(text, analysis).subscribe(suggestions => {
      this._suggestions.set(suggestions);
      this._state.set('routing');
    });
  }

  /** Return to writing state for editing. */
  edit(): void {
    this._state.set('writing');
  }

  /** Post an individual card (filing = PATCH, derivative = POST). */
  postCard(id: string): void {
    const card = this._suggestions().find(s => s.id === id);
    if (!card) return;

    this.updateCardStatus(id, 'posting');

    if (card.kind === 'filing') {
      this.storageApi
        .updateContent(this._contentId(), {
          metadata: { journalFolder: card.suggestedPath },
        })
        .subscribe(() => {
          this.updateCardStatus(id, 'posted');
          this.checkAllResolved();
        });
    } else {
      this.storageApi
        .createContent({
          id: card.id,
          title: card.title,
          schemaVersion: 1,
          description: card.summary,
          contentType: card.destinationType,
          contentFormat: 'markdown',
          contentBody: this._journalText(),
          blobHash: null,
          blobCid: null,
          contentSizeBytes: null,
          metadata: { sourceJournalId: this._contentId() },
          reach: card.reach,
          createdBy: null,
          tags: [],
        })
        .subscribe(() => {
          this.updateCardStatus(id, 'posted');
          this.checkAllResolved();
        });
    }
  }

  /** Dismiss a card without posting. */
  dismissCard(id: string): void {
    this.updateCardStatus(id, 'dismissed');
    this.checkAllResolved();
  }

  // ---------------------------------------------------------------------------
  // Backend API calls
  // ---------------------------------------------------------------------------

  private analyzeIntent(text: string): Observable<IntentAnalysis> {
    return this.http.post<IntentAnalysis>('/api/v1/journal/analyze', {
      text,
      contentId: this._contentId(),
    });
  }

  private generateSuggestions(
    text: string,
    intent: IntentAnalysis
  ): Observable<RoutingSuggestion[]> {
    return this.http.post<RoutingSuggestion[]>('/api/v1/journal/suggest', {
      text,
      contentId: this._contentId(),
      intent,
    });
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private updateCardStatus(id: string, status: RoutingSuggestion['status']): void {
    this._suggestions.update(cards => cards.map(c => (c.id === id ? { ...c, status } : c)));
  }

  private checkAllResolved(): void {
    const allResolved = this._suggestions().every(
      c => c.status === 'posted' || c.status === 'dismissed'
    );
    if (allResolved) {
      this._state.set('routed');
    }
  }
}
