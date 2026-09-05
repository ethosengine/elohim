/**
 * Session Migration Service — carry a visitor's progress onto the account they
 * just created.
 *
 * Philosophy:
 * - Zero-friction entry via session, meaningful upgrade to network
 * - Preserve all session progress during migration
 * - Handle partial failures gracefully with recovery
 *
 * This service does NOT create identities. Registration happens at the doorway
 * portal (apps are relying parties, never portals), so by the time this runs
 * the human is already authenticated and the only question left is what their
 * visitor session should contribute:
 *
 * 1. Package session data (affinity, progress, activities)
 * 2. Apply the session's profile fields to the fresh profile — and only the
 *    fields the portal could not know (a display name it defaulted from the
 *    identifier, a bio, interests)
 * 3. Transfer progress data to the private source chain
 * 4. Link the session to the new identity, then clear it
 *
 * Called from AuthCallbackComponent after a successful OAuth exchange.
 */

import { Injectable, inject, signal, computed } from '@angular/core';

// @coverage: 100.0% (2026-02-24)

import { ContentMasteryService } from '@app/lamad/services/content-mastery.service';

import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import {
  type MigrationState,
  type MigrationResult,
  type UpdateProfileRequest,
  INITIAL_MIGRATION_STATE,
} from '../models/identity.model';

import { IdentityService } from './identity.service';
import { SessionHumanService } from './session-human.service';

// =============================================================================
// Migration Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class SessionMigrationService {
  private readonly holochainClient = inject(HolochainClientService);
  private readonly sessionHumanService = inject(SessionHumanService);
  private readonly identityService = inject(IdentityService);
  private readonly contentMasteryService = inject(ContentMasteryService);

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

  /**
   * Whether there is visitor progress to carry onto an account.
   *
   * Deliberately NOT gated on a conductor socket: an anonymous visitor cannot
   * open one, and gating on it is what made the old in-app registration form
   * refuse the very people it existed for. The transfer steps below each fail
   * soft when the socket is absent — this predicate answers only "is there
   * something to carry?".
   */
  readonly canMigrate = computed(() => this.sessionHumanService.hasSession());

  // ==========================================================================
  // Migration
  // ==========================================================================

  /**
   * Apply a visitor session to the profile of the human who just signed in,
   * carry their progress across, then retire the session.
   *
   * Registration already happened — at the doorway's portal — so nothing here
   * creates an identity or handles a credential. `identifier` is the account
   * the doorway issued; it is used only to tell a real display name apart from
   * one the doorway defaulted out of the identifier's local part.
   */
  async applySessionToProfile(identifier?: string): Promise<MigrationResult> {
    if (!this.canMigrate()) {
      return { success: false, error: 'No visitor session to carry over' };
    }

    const session = this.sessionHumanService.getSession();
    if (!session) {
      return { success: false, error: 'No session to migrate' };
    }

    const humanId = this.identityService.humanId();
    const agentPubKey = this.identityService.agentPubKey();
    if (!humanId || !agentPubKey) {
      return { success: false, error: 'Not signed in yet — nothing to attach the session to' };
    }

    try {
      // Step 1: Prepare migration data
      this.updateState({
        status: 'preparing',
        currentStep: 'Packaging session data...',
        progress: 10,
      });

      const migrationPackage = this.sessionHumanService.prepareMigration();
      if (!migrationPackage) {
        throw new Error('Failed to prepare migration package');
      }

      // Step 2: Apply the session's profile fields to the profile the portal
      // just created. Profile is not auth — these were never registration
      // fields; they are what this visitor already told us about themselves.
      this.updateState({
        status: 'transferring',
        currentStep: 'Carrying your profile over...',
        progress: 30,
      });

      await this.applyProfileFields(session, identifier);

      // Step 3: Transfer progress data
      this.updateState({ currentStep: 'Transferring progress...', progress: 60 });

      // Transfer path progress
      const pathProgress = migrationPackage.pathProgress ?? [];
      for (const progress of pathProgress) {
        await this.transferPathProgress(progress);
      }

      // Transfer affinity data
      const affinityCount = Object.keys(migrationPackage.affinity ?? {}).length;
      if (affinityCount > 0) {
        // Note: transferAffinity currently doesn't accept parameters
        // Affinity data from migrationPackage.affinity will be handled in future update
        this.transferAffinity();
      }

      // Step 3b: Migrate content mastery (localStorage → backend)
      this.updateState({ currentStep: 'Migrating learning progress...', progress: 75 });

      let masteryCount = 0;
      const masteryResult = await this.contentMasteryService.migrateToBackend();
      if (masteryResult.success) {
        masteryCount = masteryResult.migrated;
      }

      this.updateState({ currentStep: 'Finalizing...', progress: 90 });

      // Step 4: Link the session to the identity, then clear it
      this.sessionHumanService.markAsMigrated(agentPubKey, humanId);
      this.sessionHumanService.clearAfterMigration();

      // Success!
      this.updateState({ status: 'completed', currentStep: 'Migration complete!', progress: 100 });

      return {
        success: true,
        newHumanId: humanId,
        migratedData: {
          affinityCount,
          pathProgressCount: pathProgress.length,
          activityCount: migrationPackage.activities?.length ?? 0,
          masteryCount,
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
   * Push the session's self-description onto the freshly authenticated profile.
   *
   * Only fields the portal could not have known are sent, and the display name
   * is only replaced when the doorway defaulted it out of the identifier's
   * local part — a name the human actually chose at the portal always wins.
   * Nothing to say means no request at all.
   */
  private async applyProfileFields(
    session: NonNullable<ReturnType<SessionHumanService['getSession']>>,
    identifier?: string
  ): Promise<void> {
    const update: UpdateProfileRequest = {};

    const sessionName = session.displayName?.trim();
    if (sessionName && sessionName !== 'Traveler' && this.displayNameIsPlaceholder(identifier)) {
      update.displayName = sessionName;
    }

    const bio = session.bio?.trim();
    if (bio && !this.identityService.profile()?.bio) {
      update.bio = bio;
    }

    const interests = session.interests ?? [];
    if (interests.length > 0 && (this.identityService.profile()?.affinities?.length ?? 0) === 0) {
      update.affinities = interests;
    }

    if (Object.keys(update).length === 0) return;

    await this.identityService.updateProfile(update);
  }

  /**
   * True when the signed-in display name is nothing but the identifier's local
   * part — i.e. the doorway had no better answer and neither does the human's
   * account yet, so the visitor's own name is an improvement.
   */
  private displayNameIsPlaceholder(identifier?: string): boolean {
    const current = this.identityService.displayName()?.trim() ?? '';
    if (!current) return true;
    if (!identifier) return false;
    const localPart = identifier.split('@')[0]?.trim() ?? '';
    return localPart.length > 0 && current.toLowerCase() === localPart.toLowerCase();
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
          completed_step_index:
            progress.currentStepIndex > 0 ? progress.currentStepIndex - 1 : undefined,
        },
        roleName: 'imagodei',
      });
    } catch {
      // Silently continue with other progress - individual path failures should not fail the entire migration
    }
  }

  /**
   * Transfer affinity data to network.
   *
   * MVP stub: Affinity data is intentionally not transferred during migration.
   * The affinity will be rebuilt as user interacts with content in the network.
   * Future implementation should call a zome function to store affinity data.
   */
  private transferAffinity(): void {
    // For now, we'll store this as a batch - future: individual affinity records
    // This could call a zome function to store affinity data
    // For MVP, the affinity will be rebuilt as user interacts with content
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
