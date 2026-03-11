import { Injectable, Injector, inject } from '@angular/core';

import { type MasteryLevel, compareMasteryLevels } from '@app/elohim/models/agent.model';

import { DiscoveryAttestationService } from '../quiz-engine/services/discovery-attestation.service';
import { PathAdaptationService } from '../quiz-engine/services/path-adaptation.service';

import { ContentMasteryService } from './content-mastery.service';

import type { DiscoveryProfile } from '../quiz-engine/models/discovery-assessment.model';
import type { PathAdaptationState } from '../quiz-engine/services/path-adaptation.service';

/**
 * The mastery level threshold at which a step becomes accessible
 * regardless of sequential position in the fog-of-war.
 *
 * 'understand' (Bloom level 3) means the learner can explain/summarize
 * the concept — sufficient prior knowledge to skip sequential reading.
 * This is intentionally lower than ATTESTATION_GATE_LEVEL ('apply'),
 * because unlocking navigation is a lower bar than granting privileges.
 */
const MASTERY_UNLOCK_THRESHOLD: MasteryLevel = 'understand';

/**
 * LearnerContextService — Aggregates mastery, discovery, and adaptation state.
 *
 * This is the "missing intelligence" layer between the learner's profile
 * and the fog-of-war gate. It answers:
 * - Does this learner already know this content? (mastery unlock)
 * - Has a pre-assessment proved they can skip this section? (skip-ahead)
 * - What does their discovery profile look like? (recommendations)
 *
 * Consumed by PathService.isStepAccessible() to make the fog-of-war
 * adaptive rather than purely sequential.
 */
@Injectable({ providedIn: 'root' })
export class LearnerContextService {
  private readonly contentMastery = inject(ContentMasteryService);
  private readonly injector = inject(Injector);
  private readonly discoveryAttestation = inject(DiscoveryAttestationService);

  /** Lazily resolved to break circular DI: PathAdaptation → QuestionPool → Path → LearnerContext */
  private get pathAdaptation(): PathAdaptationService {
    return this.injector.get(PathAdaptationService);
  }

  // =========================================================================
  // Mastery-Aware Fog-of-War
  // =========================================================================

  /**
   * Check if a content node's mastery level qualifies for fog-of-war bypass.
   *
   * A learner who has demonstrated 'understand' or higher mastery on
   * content — from another path, prior exploration, or direct engagement —
   * should not be blocked by sequential progression.
   *
   * Returns false for null/undefined resourceId (defensive).
   */
  isMasteryUnlocked(resourceId: string | undefined): boolean {
    if (!resourceId) return false;

    const level = this.contentMastery.getMasteryLevelSync(resourceId);
    return compareMasteryLevels(level, MASTERY_UNLOCK_THRESHOLD) >= 0;
  }

  /**
   * Get a human-readable reason for mastery-based access.
   * Returns null if the content is not mastery-unlocked.
   */
  getMasteryAccessReason(resourceId: string | undefined): string | null {
    if (!resourceId) return null;

    const level = this.contentMastery.getMasteryLevelSync(resourceId);
    if (compareMasteryLevels(level, MASTERY_UNLOCK_THRESHOLD) >= 0) {
      return 'Prior mastery';
    }
    return null;
  }

  // =========================================================================
  // Skip-Ahead Integration
  // =========================================================================

  /**
   * Check if a step belongs to a section that was skipped via pre-assessment.
   *
   * For hierarchical paths, steps belong to sections via their chapter
   * structure. For flat paths (no chapters), skip-ahead does not apply.
   *
   * @param pathId - The path being checked
   * @param sectionId - The section this step belongs to (from path structure)
   * @param humanId - The learner's ID
   */
  isInSkippedSection(pathId: string, sectionId: string | undefined, humanId: string): boolean {
    if (!sectionId || !humanId) return false;

    const state = this.pathAdaptation.getState(pathId, humanId);
    return state.skippedSections.has(sectionId);
  }

  /**
   * Get the full adaptation state for a path.
   */
  getAdaptationState(pathId: string, humanId: string): PathAdaptationState {
    return this.pathAdaptation.getState(pathId, humanId);
  }

  // =========================================================================
  // Discovery Profile
  // =========================================================================

  /**
   * Get the current discovery profile for path recommendations.
   * Returns null if no discovery assessments have been completed.
   */
  getDiscoveryProfile(): DiscoveryProfile | null {
    return this.discoveryAttestation.getDiscoveryProfile();
  }
}
