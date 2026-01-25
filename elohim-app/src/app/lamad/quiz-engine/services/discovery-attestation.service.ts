/**
 * Discovery Attestation Service - Links discovery assessments to ImagoDei.
 *
 * This service:
 * - Creates attestations from completed discovery assessments
 * - Stores discovery results as attestations
 * - Provides formatted display for profile/ImagoDei
 * - Syncs with Holochain when connected
 *
 * Discovery assessments differ from mastery quizzes:
 * - They reveal something about the user (Enneagram type, learning style, etc.)
 * - Results become part of the user's identity profile
 * - They are visible path steps, not hidden attestation signals
 */

import { Injectable, inject, signal, computed } from '@angular/core';

import { type Attestation } from '@app/imagodei/models/attestations.model';

import {
  type DiscoveryAssessment,
  type DiscoveryResult,
  type DiscoveryAttestation,
  type DiscoveryFramework,
  type DiscoveryCategory,
  type DiscoveryResultSummary,
  formatDiscoveryResult,
  formatDiscoveryShort,
  getFrameworkDisplayName,
  getCategoryIcon,
} from '../models/discovery-assessment.model';

// =============================================================================
// Storage Keys
// =============================================================================

const STORAGE_KEYS = {
  DISCOVERY_RESULTS: 'elohim:discovery-results',
  DISCOVERY_ATTESTATIONS: 'elohim:discovery-attestations',
} as const;

// =============================================================================
// Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class DiscoveryAttestationService {
  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  /** All discovery results for current user */
  private readonly resultsSignal = signal<DiscoveryResult[]>([]);

  /** Discovery attestations */
  private readonly attestationsSignal = signal<DiscoveryAttestation[]>([]);

  // ---------------------------------------------------------------------------
  // Public Signals
  // ---------------------------------------------------------------------------

  /** All discovery results */
  readonly results = this.resultsSignal.asReadonly();

  /** All discovery attestations */
  readonly attestations = this.attestationsSignal.asReadonly();

  /** Discovery results grouped by category */
  readonly resultsByCategory = computed(() => {
    const results = this.resultsSignal();
    const grouped: Record<DiscoveryCategory, DiscoveryResult[]> = {
      personality: [],
      strengths: [],
      values: [],
      learning: [],
      emotional: [],
      relational: [],
      vocational: [],
      spiritual: [],
    };

    for (const result of results) {
      grouped[result.category].push(result);
    }

    return grouped;
  });

  /** Discovery results grouped by framework */
  readonly resultsByFramework = computed(() => {
    const results = this.resultsSignal();
    const grouped = new Map<DiscoveryFramework, DiscoveryResult[]>();

    for (const result of results) {
      const existing = grouped.get(result.framework) || [];
      existing.push(result);
      grouped.set(result.framework, existing);
    }

    return grouped;
  });

  /** Featured discovery results for profile display */
  readonly featuredResults = computed(() => {
    const attestations = this.attestationsSignal();
    return attestations
      .filter(a => a.featured)
      .sort((a, b) => a.displayOrder - b.displayOrder)
      .map(a => a.result);
  });

  /** Public discovery results */
  readonly publicResults = computed(() => {
    const results = this.resultsSignal();
    return results.filter(r => r.isPublic);
  });

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  constructor() {
    this.loadFromStorage();
  }

  // ---------------------------------------------------------------------------
  // Public Methods
  // ---------------------------------------------------------------------------

  /**
   * Record a discovery assessment completion and create an attestation.
   */
  recordDiscoveryResult(
    assessment: DiscoveryAssessment,
    subscaleScores: Record<string, number>,
    primaryType: DiscoveryResultSummary,
    secondaryTypes?: DiscoveryResultSummary[]
  ): DiscoveryResult {
    const now = new Date().toISOString();
    const id = `discovery-${assessment.id}-${Date.now()}`;

    // Create the discovery result
    const result: DiscoveryResult = {
      id,
      assessmentId: assessment.id,
      assessmentTitle: assessment.title,
      framework: assessment.framework,
      category: assessment.category,
      humanId: this.getCurrentHumanId(),
      completedAt: now,
      primaryType,
      secondaryTypes,
      subscaleScores,
      displayString: '', // Will be set below
      shortDisplay: '', // Will be set below
      isPublic: true, // Default to public, user can change
    };

    // Generate display strings
    result.displayString = formatDiscoveryResult(result);
    result.shortDisplay = formatDiscoveryShort(result);

    // Create attestation wrapper
    const attestation: DiscoveryAttestation = {
      id: `attest-${id}`,
      humanId: result.humanId,
      type: 'discovery',
      result,
      earnedAt: now,
      visibility: 'community', // Default visibility
      featured: this.shouldAutoFeature(assessment.framework),
      displayOrder: this.getNextDisplayOrder(),
    };

    // Update state
    this.resultsSignal.update(results => {
      // Replace existing result for same assessment, or add new
      const filtered = results.filter(r => r.assessmentId !== assessment.id);
      return [...filtered, result];
    });

    this.attestationsSignal.update(attestations => {
      const filtered = attestations.filter(a => a.result.assessmentId !== assessment.id);
      return [...filtered, attestation];
    });

    // Persist
    this.saveToStorage();

    return result;
  }

  /**
   * Get discovery result for a specific assessment.
   */
  getResultForAssessment(assessmentId: string): DiscoveryResult | undefined {
    return this.resultsSignal().find(r => r.assessmentId === assessmentId);
  }

  /**
   * Get discovery result for a specific framework.
   */
  getResultForFramework(framework: DiscoveryFramework): DiscoveryResult | undefined {
    return this.resultsSignal().find(r => r.framework === framework);
  }

  /**
   * Check if user has completed a specific assessment.
   */
  hasCompletedAssessment(assessmentId: string): boolean {
    return this.resultsSignal().some(r => r.assessmentId === assessmentId);
  }

  /**
   * Update visibility for a discovery result.
   */
  updateVisibility(
    resultId: string,
    visibility: 'private' | 'trusted' | 'community' | 'public'
  ): void {
    // Update result
    this.resultsSignal.update(results =>
      results.map(r =>
        r.id === resultId
          ? { ...r, isPublic: visibility === 'public' || visibility === 'community' }
          : r
      )
    );

    // Update attestation
    this.attestationsSignal.update(attestations =>
      attestations.map(a =>
        a.result.id === resultId
          ? {
              ...a,
              visibility,
              result: {
                ...a.result,
                isPublic: visibility === 'public' || visibility === 'community',
              },
            }
          : a
      )
    );

    this.saveToStorage();
  }

  /**
   * Toggle featured status for a discovery result.
   */
  toggleFeatured(resultId: string): void {
    this.attestationsSignal.update(attestations =>
      attestations.map(a => (a.result.id === resultId ? { ...a, featured: !a.featured } : a))
    );

    this.saveToStorage();
  }

  /**
   * Update display order for featured results.
   */
  updateDisplayOrder(resultId: string, newOrder: number): void {
    this.attestationsSignal.update(attestations =>
      attestations.map(a => (a.result.id === resultId ? { ...a, displayOrder: newOrder } : a))
    );

    this.saveToStorage();
  }

  /**
   * Add a reflection to a discovery result.
   */
  addReflection(resultId: string, reflection: string): void {
    this.resultsSignal.update(results =>
      results.map(r => (r.id === resultId ? { ...r, reflection } : r))
    );

    this.attestationsSignal.update(attestations =>
      attestations.map(a =>
        a.result.id === resultId ? { ...a, result: { ...a.result, reflection } } : a
      )
    );

    this.saveToStorage();
  }

  /**
   * Convert discovery attestation to standard attestation format.
   * This is used when integrating with the main attestation system.
   */
  toStandardAttestation(discoveryAttestation: DiscoveryAttestation): Attestation {
    const result = discoveryAttestation.result;

    return {
      id: discoveryAttestation.id,
      name: `${getFrameworkDisplayName(result.framework)}: ${result.displayString}`,
      description: `Completed ${result.assessmentTitle} on ${new Date(result.completedAt).toLocaleDateString()}`,
      type: 'discovery',
      earnedAt: discoveryAttestation.earnedAt,
      revocable: false,
      metadata: {
        framework: result.framework,
        category: result.category,
        displayString: result.displayString,
        shortDisplay: result.shortDisplay,
        subscaleScores: result.subscaleScores,
        primaryType: result.primaryType,
        secondaryTypes: result.secondaryTypes,
        reflection: result.reflection,
        icon: getCategoryIcon(result.category),
      },
    };
  }

  /**
   * Get all discovery attestations as standard attestations.
   */
  getAsStandardAttestations(): Attestation[] {
    return this.attestationsSignal().map(a => this.toStandardAttestation(a));
  }

  /**
   * Clear all discovery data (for testing/reset).
   */
  clearAll(): void {
    this.resultsSignal.set([]);
    this.attestationsSignal.set([]);
    localStorage.removeItem(STORAGE_KEYS.DISCOVERY_RESULTS);
    localStorage.removeItem(STORAGE_KEYS.DISCOVERY_ATTESTATIONS);
  }

  // ---------------------------------------------------------------------------
  // Display Helpers
  // ---------------------------------------------------------------------------

  /**
   * Get formatted display card for a discovery result.
   */
  getDisplayCard(result: DiscoveryResult): {
    icon: string;
    framework: string;
    title: string;
    shortCode: string;
    category: string;
    completedAt: string;
  } {
    return {
      icon: getCategoryIcon(result.category),
      framework: getFrameworkDisplayName(result.framework),
      title: result.displayString,
      shortCode: result.shortDisplay,
      category: result.category,
      completedAt: new Date(result.completedAt).toLocaleDateString(),
    };
  }

  /**
   * Get badge-style display for profile chips.
   */
  getBadgeDisplay(result: DiscoveryResult): {
    label: string;
    icon: string;
    color: string;
  } {
    const colors: Record<DiscoveryCategory, string> = {
      personality: '#8B5CF6', // Purple
      strengths: '#10B981', // Green
      values: '#F59E0B', // Amber
      learning: '#3B82F6', // Blue
      emotional: '#EC4899', // Pink
      relational: '#F97316', // Orange
      vocational: '#6366F1', // Indigo
      spiritual: '#14B8A6', // Teal
    };

    return {
      label: result.shortDisplay,
      icon: getCategoryIcon(result.category),
      color: result.primaryType.color || colors[result.category],
    };
  }

  // ---------------------------------------------------------------------------
  // Private Helpers
  // ---------------------------------------------------------------------------

  private getCurrentHumanId(): string {
    // TODO: Get from IdentityService
    return localStorage.getItem('elohim:session-human-id') || 'anonymous';
  }

  private shouldAutoFeature(framework: DiscoveryFramework): boolean {
    // Auto-feature major personality assessments
    const autoFeatured: DiscoveryFramework[] = [
      'enneagram',
      'mbti',
      'clifton-strengths',
      'big-five',
    ];
    return autoFeatured.includes(framework);
  }

  private getNextDisplayOrder(): number {
    const current = this.attestationsSignal();
    if (current.length === 0) return 0;
    return Math.max(...current.map(a => a.displayOrder)) + 1;
  }

  private loadFromStorage(): void {
    try {
      const resultsJson = localStorage.getItem(STORAGE_KEYS.DISCOVERY_RESULTS);
      const attestationsJson = localStorage.getItem(STORAGE_KEYS.DISCOVERY_ATTESTATIONS);

      if (resultsJson) {
        this.resultsSignal.set(JSON.parse(resultsJson));
      }

      if (attestationsJson) {
        this.attestationsSignal.set(JSON.parse(attestationsJson));
      }
    } catch (err) {
      console.error('[DiscoveryAttestationService] Failed to load from storage:', err);
    }
  }

  private saveToStorage(): void {
    try {
      localStorage.setItem(STORAGE_KEYS.DISCOVERY_RESULTS, JSON.stringify(this.resultsSignal()));
      localStorage.setItem(
        STORAGE_KEYS.DISCOVERY_ATTESTATIONS,
        JSON.stringify(this.attestationsSignal())
      );
    } catch (err) {
      console.error('[DiscoveryAttestationService] Failed to save to storage:', err);
    }
  }
}
