/**
 * RecoveryRequestComponent - Identity recovery request flow.
 *
 * For users who have lost access to their identity, this component
 * guides them through initiating a recovery request, tracking
 * attestation progress, and completing recovery when approved.
 *
 * Flow:
 * 1. Claim identity (enter known identifier/display name)
 * 2. Await interviewers (show progress toward threshold)
 * 3. Complete recovery (receive new credentials)
 */

import { CommonModule } from '@angular/common';
import { Component, inject, OnInit, OnDestroy, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';

// @coverage: 29.8% (2026-02-04)

import { getRecoveryStatusDisplay } from '../../models/recovery.model';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { RecoveryCoordinatorService } from '../../services/recovery-coordinator.service';

type RecoveryStep = 'claim' | 'awaiting' | 'complete';

@Component({
  selector: 'app-recovery-request',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './recovery-request.component.html',
  styleUrls: ['./recovery-request.component.css'],
})
export class RecoveryRequestComponent implements OnInit, OnDestroy {
  private readonly recoveryService = inject(RecoveryCoordinatorService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly router = inject(Router);

  // ===========================================================================
  // State Delegation
  // ===========================================================================

  readonly activeRequest = this.recoveryService.activeRequest;
  readonly progress = this.recoveryService.progress;
  readonly credential = this.recoveryService.credential;
  readonly isLoading = this.recoveryService.isLoading;
  readonly error = this.recoveryService.error;
  readonly hasDoorway = this.doorwayRegistry.hasSelection;

  // ===========================================================================
  // Component State
  // ===========================================================================

  readonly currentStep = signal<RecoveryStep>('claim');

  // Form data
  claimedIdentity = '';
  additionalContext = '';

  // Polling
  private pollInterval: ReturnType<typeof setInterval> | null = null;

  // ===========================================================================
  // Template Helpers
  // ===========================================================================

  readonly getRecoveryStatusDisplay = getRecoveryStatusDisplay;

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    // Check if there's already an active request
    if (this.activeRequest()) {
      this.currentStep.set('awaiting');
      this.startPolling();
    }

    // Check if recovery is complete
    if (this.credential()) {
      this.currentStep.set('complete');
    }
  }

  ngOnDestroy(): void {
    this.stopPolling();
  }

  // ===========================================================================
  // Actions
  // ===========================================================================

  async initiateRecovery(): Promise<void> {
    if (!this.claimedIdentity.trim()) {
      return;
    }

    const success = await this.recoveryService.initiateRecovery(
      this.claimedIdentity.trim(),
      this.additionalContext.trim() || undefined
    );

    if (success) {
      this.currentStep.set('awaiting');
      this.startPolling();
    }
  }

  async cancelRecovery(): Promise<void> {
    await this.recoveryService.cancelRecovery();
    this.stopPolling();
    this.currentStep.set('claim');
    this.claimedIdentity = '';
    this.additionalContext = '';
  }

  async completeRecovery(): Promise<void> {
    const success = await this.recoveryService.completeRecovery();
    if (success) {
      // Navigate to login after recovery
      void this.router.navigate(['/identity/login']);
    }
  }

  selectDoorway(): void {
    // Navigate to doorway picker
    void this.router.navigate(['/identity/login'], {
      queryParams: { step: 'doorway' },
    });
  }

  clearError(): void {
    this.recoveryService.clearError();
  }

  // ===========================================================================
  // Polling
  // ===========================================================================

  private startPolling(): void {
    if (this.pollInterval) return;

    // Poll every 10 seconds for status updates
    this.pollInterval = setInterval(() => {
      void this.recoveryService.refreshRequestStatus();

      // Check if recovery completed or denied
      const request = this.activeRequest();
      if (request) {
        if (request.status === 'completed' || request.status === 'attested') {
          this.currentStep.set('complete');
          this.stopPolling();
        } else if (request.status === 'denied' || request.status === 'expired') {
          this.stopPolling();
        }
      }
    }, 10_000);

    // Initial fetch
    void this.recoveryService.refreshRequestStatus();
  }

  private stopPolling(): void {
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = null;
    }
  }

  // ===========================================================================
  // Computed Helpers
  // ===========================================================================

  getProgressPercentage(): number {
    return this.progress()?.progressPercent ?? 0;
  }

  getAttestationText(): string {
    const p = this.progress();
    if (!p) return '0 of 0';
    return `${p.affirmCount} of ${p.requiredCount}`;
  }

  isThresholdMet(): boolean {
    return this.progress()?.thresholdMet ?? false;
  }

  isDenied(): boolean {
    return this.progress()?.isDenied ?? false;
  }
}
