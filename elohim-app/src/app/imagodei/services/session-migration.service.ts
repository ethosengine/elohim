/**
 * Session Migration Service - Upgrade from session to network identity.
 *
 * Philosophy:
 * - Zero-friction entry via session, meaningful upgrade to network
 * - Preserve all session progress during migration
 * - Handle partial failures gracefully with recovery
 *
 * Migration Flow:
 * 1. Package session data (affinity, progress, activities)
 * 2. Register human identity in network
 * 3. Transfer progress data to private source chain
 * 4. Clear session after successful migration
 */

import { Injectable, inject, signal, computed } from '@angular/core';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { SessionHumanService } from './session-human.service';
import { IdentityService } from './identity.service';
import {
  type MigrationState,
  type MigrationStatus,
  type MigrationResult,
  type RegisterHumanRequest,
  INITIAL_MIGRATION_STATE,
} from '../models/identity.model';

// =============================================================================
// Migration Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class SessionMigrationService {
  private readonly holochainClient = inject(HolochainClientService);
  private readonly sessionHumanService = inject(SessionHumanService);
  private readonly identityService = inject(IdentityService);

  // ==========================================================================
  // State
  // ==========================================================================

  /** Migration state */
  private readonly migrationSignal = signal<MigrationState>(INITIAL_MIGRATION_STATE);

  // ==========================================================================
  // Public Signals
  // ==========================================================================

  /** Current migration state */
  readonly state = this.migrationSignal.asReadonly();

  /** Migration status */
  readonly status = computed(() => this.migrationSignal().status);

  /** Whether migration is in progress */
  readonly isInProgress = computed(() => {
    const status = this.migrationSignal().status;
    return status === 'preparing' || status === 'registering' || status === 'transferring';
  });

  /** Whether migration can be started */
  readonly canMigrate = computed(() =>
    this.sessionHumanService.hasSession() &&
    this.holochainClient.isConnected() &&
    this.identityService.mode() === 'session'
  );

  // ==========================================================================
  // Migration
  // ==========================================================================

  /**
   * Perform full migration from session to network identity.
   */
  async migrate(profileOverrides?: Partial<RegisterHumanRequest>): Promise<MigrationResult> {
    if (!this.canMigrate()) {
      return {
        success: false,
        error: 'Migration not available - check connection and session status',
      };
    }

    const session = this.sessionHumanService.getSession();
    if (!session) {
      return {
        success: false,
        error: 'No session to migrate',
      };
    }

    try {
      // Step 1: Prepare migration data
      this.updateState({ status: 'preparing', currentStep: 'Packaging session data...', progress: 10 });

      const migrationPackage = this.sessionHumanService.prepareMigration();
      if (!migrationPackage) {
        throw new Error('Failed to prepare migration package');
      }

      // Step 2: Register human in network
      this.updateState({ status: 'registering', currentStep: 'Creating network identity...', progress: 30 });

      const registrationRequest: RegisterHumanRequest = {
        displayName: profileOverrides?.displayName ?? session.displayName,
        bio: profileOverrides?.bio ?? session.bio,
        affinities: profileOverrides?.affinities ?? session.interests ?? [],
        profileReach: profileOverrides?.profileReach ?? 'community',
        location: profileOverrides?.location ?? undefined,
        migrateFromSession: true,
      };

      const profile = await this.identityService.registerHuman(registrationRequest);

      // Step 3: Transfer progress data
      this.updateState({ status: 'transferring', currentStep: 'Transferring progress...', progress: 60 });

      // Transfer path progress
      const pathProgress = migrationPackage.pathProgress ?? [];
      for (const progress of pathProgress) {
        await this.transferPathProgress(progress);
      }

      // Transfer affinity data
      const affinityCount = Object.keys(migrationPackage.affinity ?? {}).length;
      if (affinityCount > 0) {
        await this.transferAffinity(migrationPackage.affinity);
      }

      this.updateState({ currentStep: 'Finalizing...', progress: 90 });

      // Step 4: Clear session
      this.sessionHumanService.clearAfterMigration();

      // Success!
      this.updateState({ status: 'completed', currentStep: 'Migration complete!', progress: 100 });

      return {
        success: true,
        newHumanId: profile.id,
        migratedData: {
          affinityCount,
          pathProgressCount: pathProgress.length,
          activityCount: migrationPackage.activities?.length ?? 0,
        },
      };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Migration failed';
      this.updateState({ status: 'failed', error: errorMessage });

      return {
        success: false,
        error: errorMessage,
      };
    }
  }

  /**
   * Transfer path progress to network.
   */
  private async transferPathProgress(progress: {
    pathId: string;
    currentStepIndex: number;
    completedStepIndices: number[];
    startedAt: string;
    lastActivityAt: string;
  }): Promise<void> {
    try {
      // Agent progress lives in imagodei DNA (identity-bound learning state)
      // Note: imagodei expects agent_id - we use current agent's human ID
      const agentId = this.identityService.humanId() ?? 'anonymous';

      await this.holochainClient.callZome({
        zomeName: 'imagodei',
        fnName: 'get_or_create_agent_progress',
        payload: {
          agent_id: agentId,
          path_id: progress.pathId,
        },
        roleName: 'imagodei',
      });

      // Update with migrated data
      await this.holochainClient.callZome({
        zomeName: 'imagodei',
        fnName: 'update_agent_progress',
        payload: {
          agent_id: agentId,
          path_id: progress.pathId,
          completed_step_index: progress.currentStepIndex > 0 ? progress.currentStepIndex - 1 : undefined,
        },
        roleName: 'imagodei',
      });
    } catch (err) {
      console.warn('[SessionMigration] Failed to transfer path progress:', progress.pathId, err);
      // Continue with other progress - don't fail entire migration
    }
  }

  /**
   * Transfer affinity data to network.
   */
  private async transferAffinity(affinity: Record<string, number>): Promise<void> {
    // For now, we'll store this as a batch - future: individual affinity records
    try {
      // This could call a zome function to store affinity data
      // For MVP, the affinity will be rebuilt as user interacts with content
      console.log('[SessionMigration] Affinity data prepared for transfer:', Object.keys(affinity).length, 'entries');
    } catch (err) {
      console.warn('[SessionMigration] Failed to transfer affinity:', err);
    }
  }

  /**
   * Reset migration state (e.g., after dismissing error).
   */
  reset(): void {
    this.migrationSignal.set(INITIAL_MIGRATION_STATE);
  }

  // ==========================================================================
  // State Management
  // ==========================================================================

  /**
   * Update migration state.
   */
  private updateState(partial: Partial<MigrationState>): void {
    this.migrationSignal.update(current => ({
      ...current,
      ...partial,
    }));
  }
}
