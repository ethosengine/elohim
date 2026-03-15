import { Injectable, inject, signal, computed } from '@angular/core';

import type { GateEvaluationView } from '@elohim/storage-client';

import { GateService } from './gate.service';

export type GateArtifactState =
  | 'draft'
  | 'evaluating'
  | 'affirm'
  | 'dialogue'
  | 'settled'
  | 'posted';

export type ReachTier = 'private' | 'close' | 'community' | 'network' | 'constitutional';

export interface MutationContext {
  contentId?: string;
  [key: string]: unknown;
}

/**
 * Component-scoped gate interaction state machine.
 * Must be provided in the host component's `providers` array —
 * each artifact card gets its own isolated instance.
 */
@Injectable()
export class GateInteractionService {
  private readonly gateService = inject(GateService);

  private readonly _state = signal<GateArtifactState>('draft');
  private readonly _draftText = signal('');
  private readonly _gateResult = signal<GateEvaluationView | null>(null);
  private _mutationType = '';
  private _context: MutationContext = {};

  readonly state = this._state.asReadonly();
  readonly draftText = this._draftText.asReadonly();
  readonly gateResult = this._gateResult.asReadonly();

  readonly reachTier = computed<ReachTier>(() => {
    const result = this._gateResult();
    if (!result) {
      return 'close';
    }
    if (result.settlementBoundary != null) {
      return 'private';
    }
    const trust = result.trustContext.compositeTrust;
    if (trust < 0.3) {
      return 'close';
    }
    if (trust < 0.6) {
      return 'community';
    }
    if (trust < 0.85) {
      return 'network';
    }
    return 'constitutional';
  });

  submit(text: string, mutationType: string, context: MutationContext): void {
    if (this._state() === 'evaluating') {
      return;
    }
    this._draftText.set(text);
    this._mutationType = mutationType;
    this._context = context;
    this._state.set('evaluating');
  }

  handleGateEvaluation(gate: GateEvaluationView): void {
    this._gateResult.set(gate);
    this.gateService.handleGateResponse(gate);

    if (gate.settlementBoundary != null) {
      this._state.set('settled');
    } else if (gate.pausePrompt != null) {
      this._state.set('dialogue');
    } else {
      this._state.set('affirm');
    }
  }

  affirm(): void {
    if (this._state() !== 'affirm') {
      return;
    }
    const token = this._gateResult()?.confirmToken;
    if (token) {
      this.gateService.confirmPause(token).subscribe({
        next: () => this._state.set('posted'),
        error: () => {
          // Stay in affirm state on failure — user can retry
        },
      });
    } else {
      // No token means passthrough — post directly
      this._state.set('posted');
    }
  }

  revise(newText: string): void {
    this._draftText.set(newText);
    this._state.set('draft');
  }

  resubmit(): void {
    if (this._state() !== 'draft') {
      return;
    }
    this._state.set('evaluating');
  }

  reset(): void {
    this._state.set('draft');
    this._draftText.set('');
    this._gateResult.set(null);
    this._mutationType = '';
    this._context = {};
    this.gateService.clearState();
  }
}
