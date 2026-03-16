import { Injectable, inject, signal } from '@angular/core';

import { map, take } from 'rxjs/operators';

import { Observable, timer } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

import type {
  JournalRoutingState,
  DestinationType,
  IntentAnalysis,
  RoutingSuggestion,
} from '../models/journal-routing.model';

let nextId = 0;
function generateId(prefix: string): string {
  return `${prefix}-${Date.now()}-${++nextId}`;
}

/**
 * JournalRoutingService — state machine for the journal routing flow.
 *
 * When a human finishes writing a journal entry, this service:
 *   1. Analyzes intent (Round 1 — keyword stub, future: inference sidecar)
 *   2. Generates routing suggestions (Round 2 — derivative cards)
 *   3. Posts or dismisses each card
 *
 * State machine: writing -> confirming -> routing -> routed
 *   with edit() returning to writing from confirming or routing.
 *
 * Component-scoped — no providedIn, provided by host component.
 */
@Injectable()
export class JournalRoutingService {
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
          id: generateId('derivative'),
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
  // Sidecar seams (stubbed with keyword matching)
  // ---------------------------------------------------------------------------

  /**
   * Analyze journal text for intent.
   * SIDECAR SEAM: Replace with inference sidecar call when available.
   */
  private analyzeIntent(text: string): Observable<IntentAnalysis> {
    return timer(800).pipe(
      take(1),
      map(() => {
        const lower = text.toLowerCase();
        const detectedTypes: DestinationType[] = [];

        const exchangeKeywords = [
          'need',
          'want',
          'broken',
          'repair',
          'help',
          'fix',
          'hire',
          'find',
        ];
        const governanceKeywords = [
          'should',
          'vote',
          'policy',
          'propose',
          'fund',
          'community',
          'rule',
        ];
        const contentKeywords = ['learned', 'discovered', 'guide', 'how-to', 'tutorial', 'explain'];

        if (exchangeKeywords.some(kw => lower.includes(kw))) {
          detectedTypes.push('exchange-request');
        }
        if (governanceKeywords.some(kw => lower.includes(kw))) {
          detectedTypes.push('governance-proposal');
        }
        if (contentKeywords.some(kw => lower.includes(kw))) {
          detectedTypes.push('content');
        }

        const summaryParts: string[] = [];
        if (detectedTypes.includes('exchange-request')) {
          summaryParts.push('exchange request');
        }
        if (detectedTypes.includes('governance-proposal')) {
          summaryParts.push('governance proposal');
        }
        if (detectedTypes.includes('content')) {
          summaryParts.push('shareable content');
        }

        const summary =
          summaryParts.length > 0
            ? `This entry looks like it could become: ${summaryParts.join(', ')}.`
            : 'This entry can be filed in your journal.';

        const suggestedPath = this.guessFilingPath(lower);

        return { summary, detectedTypes, suggestedPath };
      })
    );
  }

  /**
   * Generate routing suggestions based on intent analysis.
   * SIDECAR SEAM: Replace with inference sidecar call when available.
   */
  private generateSuggestions(
    text: string,
    intent: IntentAnalysis
  ): Observable<RoutingSuggestion[]> {
    return timer(800).pipe(
      take(1),
      map(() => {
        const suggestions: RoutingSuggestion[] = [];

        // Always include a filing card
        suggestions.push({
          id: generateId('filing'),
          kind: 'filing',
          destinationType: 'journal-filing',
          title: 'File in journal',
          summary: `File this entry under ${intent.suggestedPath}`,
          suggestedPath: intent.suggestedPath,
          reach: 'private',
          contextMetadata: {},
          status: 'suggested',
        });

        // Add derivative cards for each detected type
        for (const dtype of intent.detectedTypes) {
          const card = this.buildDerivativeCard(dtype, intent.suggestedPath);
          suggestions.push(card);
        }

        return suggestions;
      })
    );
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private buildDerivativeCard(dtype: DestinationType, suggestedPath: string): RoutingSuggestion {
    const titles: Record<DestinationType, string> = {
      'exchange-request': 'Post to exchange',
      'governance-proposal': 'Draft governance proposal',
      content: 'Share as learning content',
    };

    const summaries: Record<DestinationType, string> = {
      'exchange-request': 'Post a request or offer on the community exchange.',
      'governance-proposal': 'Draft a governance proposal for community review.',
      content: 'Share this as learning content for others.',
    };

    const reaches: Record<DestinationType, 'community' | 'network'> = {
      'exchange-request': 'community',
      'governance-proposal': 'community',
      content: 'network',
    };

    return {
      id: generateId('derivative'),
      kind: 'derivative',
      destinationType: dtype,
      title: titles[dtype],
      summary: summaries[dtype],
      suggestedPath,
      reach: reaches[dtype],
      contextMetadata: {},
      status: 'suggested',
    };
  }

  private guessFilingPath(lowerText: string): string {
    const homeKeywords = ['faucet', 'roof', 'plumbing', 'kitchen', 'garage', 'repair', 'fix'];
    const learningKeywords = ['learned', 'studied', 'course', 'tutorial', 'formula', 'equation'];
    const workKeywords = ['meeting', 'project', 'deadline', 'client', 'office', 'salary'];

    if (homeKeywords.some(kw => lowerText.includes(kw))) return 'home-maintenance';
    if (learningKeywords.some(kw => lowerText.includes(kw))) return 'learning';
    if (workKeywords.some(kw => lowerText.includes(kw))) return 'work';
    return 'general';
  }

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
