/**
 * RecoveryCoordinatorService - Orchestrate social recovery process.
 *
 * Philosophy:
 * - Recovery is mediated by the Elohim network, not automation
 * - Interviewers use network history to verify identity
 * - Trust is built through human-to-human interaction
 *
 * For the person recovering:
 * - Initiate recovery request
 * - Track attestation progress
 * - Complete recovery when threshold met
 *
 * For Elohim interviewers:
 * - View pending recovery requests
 * - Conduct interviews with question generation
 * - Submit attestations based on confidence
 */

import { Injectable, inject, signal, computed } from '@angular/core';

import {
  type RecoveryRequest,
  type RecoveryProgress,
  type RecoveryInterview,
  type InterviewQuestion,
  type InterviewResponse,
  type PendingRecoveryRequest,
  type RecoveryCredential,
  type AttestationDecision,
  calculateProgress,
} from '../models/recovery.model';

import { DoorwayRegistryService } from './doorway-registry.service';
import { IdentityService } from './identity.service';

/** Default attestation requirements */
const DEFAULT_REQUIRED_ATTESTATIONS = 3;
const DEFAULT_DENY_THRESHOLD = 2;

@Injectable({
  providedIn: 'root',
})
export class RecoveryCoordinatorService {
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly identityService = inject(IdentityService);

  // ===========================================================================
  // State - Claimant
  // ===========================================================================

  /** Active recovery request (if claimant has one) */
  private readonly _activeRequest = signal<RecoveryRequest | null>(null);

  /** Active interview session (for claimant) */
  private readonly _activeInterview = signal<RecoveryInterview | null>(null);

  /** Recovery credential (after successful recovery) */
  private readonly _credential = signal<RecoveryCredential | null>(null);

  // ===========================================================================
  // State - Interviewer
  // ===========================================================================

  /** Pending requests needing interviews */
  private readonly _pendingRequests = signal<PendingRecoveryRequest[]>([]);

  /** Current interview being conducted (as interviewer) */
  private readonly _conductingInterview = signal<RecoveryInterview | null>(null);

  // ===========================================================================
  // State - General
  // ===========================================================================

  private readonly _isLoading = signal(false);
  private readonly _error = signal<string | null>(null);

  // ===========================================================================
  // Public Signals
  // ===========================================================================

  readonly activeRequest = this._activeRequest.asReadonly();
  readonly activeInterview = this._activeInterview.asReadonly();
  readonly credential = this._credential.asReadonly();
  readonly pendingRequests = this._pendingRequests.asReadonly();
  readonly conductingInterview = this._conductingInterview.asReadonly();
  readonly isLoading = this._isLoading.asReadonly();
  readonly error = this._error.asReadonly();

  /** Whether claimant has an active recovery request */
  readonly hasActiveRequest = computed(() => this._activeRequest() !== null);

  /** Progress of active request */
  readonly progress = computed<RecoveryProgress | null>(() => {
    const request = this._activeRequest();
    if (!request) return null;
    return calculateProgress(
      request.attestations,
      request.requiredAttestations,
      request.denyThreshold
    );
  });

  /** Whether recovery was successful */
  readonly isRecovered = computed(() => this._credential() !== null);

  /** Count of pending requests needing interviewers */
  readonly pendingCount = computed(() => this._pendingRequests().length);

  // ===========================================================================
  // Claimant Methods
  // ===========================================================================

  /**
   * Initiate a recovery request.
   */
  async initiateRecovery(claimedIdentity: string, context?: string): Promise<boolean> {
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) {
      this._error.set('No doorway selected. Please select a doorway first.');
      return false;
    }

    this._isLoading.set(true);
    this._error.set(null);

    try {
      const response = await fetch(`${doorwayUrl}/api/recovery/initiate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          claimedIdentity,
          context,
          requiredAttestations: DEFAULT_REQUIRED_ATTESTATIONS,
          denyThreshold: DEFAULT_DENY_THRESHOLD,
        }),
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message ?? 'Failed to initiate recovery');
      }

      const request: RecoveryRequest = await response.json();
      this._activeRequest.set(request);

      console.log('[Recovery] Initiated recovery request:', request.id);
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to initiate recovery';
      this._error.set(message);
      console.error('[Recovery] Error initiating recovery:', err);
      return false;
    } finally {
      this._isLoading.set(false);
    }
  }

  /**
   * Check status of active recovery request.
   */
  async refreshRequestStatus(): Promise<void> {
    const request = this._activeRequest();
    if (!request) return;

    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return;

    try {
      const response = await fetch(`${doorwayUrl}/api/recovery/${request.id}/status`);
      if (!response.ok) throw new Error('Failed to fetch status');

      const updated: RecoveryRequest = await response.json();
      this._activeRequest.set(updated);

      // Check if recovery completed
      if (updated.status === 'attested' || updated.status === 'completed') {
        await this.fetchCredential(request.id);
      }
    } catch (err) {
      console.error('[Recovery] Error refreshing status:', err);
    }
  }

  /**
   * Fetch recovery credential after successful attestation.
   */
  private async fetchCredential(requestId: string): Promise<void> {
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return;

    try {
      const response = await fetch(`${doorwayUrl}/api/recovery/${requestId}/credential`);
      if (!response.ok) return;

      const credential: RecoveryCredential = await response.json();
      this._credential.set(credential);

      console.log('[Recovery] Credential received:', credential.id);
    } catch (err) {
      console.error('[Recovery] Error fetching credential:', err);
    }
  }

  /**
   * Cancel active recovery request.
   */
  async cancelRecovery(): Promise<void> {
    const request = this._activeRequest();
    if (!request) return;

    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return;

    try {
      await fetch(`${doorwayUrl}/api/recovery/${request.id}/cancel`, {
        method: 'POST',
      });

      this._activeRequest.set(null);
      console.log('[Recovery] Cancelled recovery request');
    } catch (err) {
      console.error('[Recovery] Error cancelling recovery:', err);
    }
  }

  /**
   * Complete recovery using credential.
   */
  async completeRecovery(): Promise<boolean> {
    const credential = this._credential();
    if (!credential || credential.claimed) {
      this._error.set('No valid credential available');
      return false;
    }

    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return false;

    this._isLoading.set(true);
    this._error.set(null);

    try {
      const response = await fetch(`${doorwayUrl}/api/recovery/${credential.requestId}/complete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          claimToken: credential.claimToken,
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to complete recovery');
      }

      // Credential is now claimed
      this._credential.update(c => (c ? { ...c, claimed: true } : null));

      // Clear request
      this._activeRequest.set(null);

      console.log('[Recovery] Recovery completed successfully');
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to complete recovery';
      this._error.set(message);
      return false;
    } finally {
      this._isLoading.set(false);
    }
  }

  // ===========================================================================
  // Interviewer Methods
  // ===========================================================================

  /**
   * Load pending recovery requests needing interviews.
   */
  async loadPendingRequests(): Promise<void> {
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return;

    this._isLoading.set(true);

    try {
      const response = await fetch(`${doorwayUrl}/api/recovery/queue`, {
        credentials: 'include',
      });

      if (!response.ok) throw new Error('Failed to load recovery queue');

      const data = await response.json();
      this._pendingRequests.set(data.requests ?? []);
    } catch (err) {
      console.error('[Recovery] Error loading pending requests:', err);
    } finally {
      this._isLoading.set(false);
    }
  }

  /**
   * Start an interview for a recovery request.
   */
  async startInterview(requestId: string): Promise<boolean> {
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return false;

    this._isLoading.set(true);
    this._error.set(null);

    try {
      const response = await fetch(`${doorwayUrl}/api/recovery/${requestId}/interview/start`, {
        method: 'POST',
        credentials: 'include',
      });

      if (!response.ok) {
        throw new Error('Failed to start interview');
      }

      const interview: RecoveryInterview = await response.json();
      this._conductingInterview.set(interview);

      console.log('[Recovery] Started interview:', interview.id);
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to start interview';
      this._error.set(message);
      return false;
    } finally {
      this._isLoading.set(false);
    }
  }

  /**
   * Generate interview questions based on claimant's network history.
   */
  async generateQuestions(requestId: string): Promise<InterviewQuestion[]> {
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return [];

    try {
      const response = await fetch(`${doorwayUrl}/api/recovery/${requestId}/interview/questions`, {
        credentials: 'include',
      });

      if (!response.ok) return [];

      const data = await response.json();
      return data.questions ?? [];
    } catch (err) {
      console.error('[Recovery] Error generating questions:', err);
      return [];
    }
  }

  /**
   * Submit a response during interview.
   */
  async submitResponse(questionId: string, answer: string): Promise<InterviewResponse | null> {
    const interview = this._conductingInterview();
    if (!interview) return null;

    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return null;

    try {
      const response = await fetch(
        `${doorwayUrl}/api/recovery/${interview.requestId}/interview/respond`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          credentials: 'include',
          body: JSON.stringify({ questionId, answer }),
        }
      );

      if (!response.ok) return null;

      const data = await response.json();

      // Update interview with new response
      this._conductingInterview.update(i =>
        i
          ? {
              ...i,
              responses: [...i.responses, data.response],
            }
          : null
      );

      return data.response;
    } catch (err) {
      console.error('[Recovery] Error submitting response:', err);
      return null;
    }
  }

  /**
   * Submit attestation decision after interview.
   */
  async submitAttestation(
    decision: AttestationDecision,
    confidence: number,
    notes?: string
  ): Promise<boolean> {
    const interview = this._conductingInterview();
    if (!interview) {
      this._error.set('No active interview');
      return false;
    }

    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) return false;

    this._isLoading.set(true);
    this._error.set(null);

    try {
      const response = await fetch(
        `${doorwayUrl}/api/recovery/${interview.requestId}/attestation`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          credentials: 'include',
          body: JSON.stringify({
            interviewId: interview.id,
            decision,
            confidence,
            notes,
          }),
        }
      );

      if (!response.ok) {
        throw new Error('Failed to submit attestation');
      }

      // Clear conducting interview
      this._conductingInterview.set(null);

      // Refresh pending requests
      await this.loadPendingRequests();

      console.log('[Recovery] Submitted attestation:', decision);
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to submit attestation';
      this._error.set(message);
      return false;
    } finally {
      this._isLoading.set(false);
    }
  }

  /**
   * Abandon interview without submitting attestation.
   */
  abandonInterview(): void {
    this._conductingInterview.set(null);
  }

  // ===========================================================================
  // Utility Methods
  // ===========================================================================

  /**
   * Clear error state.
   */
  clearError(): void {
    this._error.set(null);
  }

  /**
   * Reset all recovery state.
   */
  reset(): void {
    this._activeRequest.set(null);
    this._activeInterview.set(null);
    this._credential.set(null);
    this._pendingRequests.set([]);
    this._conductingInterview.set(null);
    this._error.set(null);
  }
}
