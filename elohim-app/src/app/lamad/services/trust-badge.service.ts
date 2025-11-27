import { Injectable } from '@angular/core';
import { Observable, of, forkJoin } from 'rxjs';
import { map, switchMap, catchError } from 'rxjs/operators';
import { DataLoaderService } from './data-loader.service';
import { ContentService } from './content.service';
import { AgentService } from './agent.service';
import { ContentNode, ContentReach, ContentFlag } from '../models/content-node.model';
import {
  ContentAttestation,
  ContentAttestationType,
  CONTENT_REACH_LEVELS
} from '../models/content-attestation.model';
import {
  TrustBadge,
  CompactTrustBadge,
  BadgeDisplay,
  BadgeWarning,
  BadgeAction,
  TrustLevel,
  TrustIndicator,
  TrustIndicatorSet,
  ATTESTATION_BADGE_CONFIG,
  REACH_BADGE_CONFIG,
  WARNING_CONFIG,
  ATTESTATION_PRIORITY,
  calculateTrustLevel,
  generateTrustSummary,
  generateAriaLabel,
  toCompactBadge,
  badgeToIndicator,
  warningToIndicator
} from '../models/trust-badge.model';

/**
 * TrustBadgeService - Computes UI-ready trust badges for content.
 *
 * This service:
 * - Fetches attestations for content
 * - Computes trust scores and levels
 * - Returns pre-formatted badge data for UI binding
 * - Determines available user actions based on agent attestations
 *
 * Usage:
 * ```typescript
 * // Full badge for detail view
 * this.trustBadgeService.getBadge(contentId).subscribe(badge => {
 *   this.primaryBadge = badge.primary;
 *   this.trustLevel = badge.trustLevel;
 * });
 *
 * // Compact badge for list view
 * this.trustBadgeService.getCompactBadge(contentId).subscribe(badge => {
 *   this.badgeIcon = badge.icon;
 *   this.badgeColor = badge.color;
 * });
 *
 * // Bulk load for lists
 * this.trustBadgeService.getBadgesForContent(contentIds).subscribe(badges => {
 *   this.badges = badges;
 * });
 * ```
 */
@Injectable({ providedIn: 'root' })
export class TrustBadgeService {
  constructor(
    private dataLoader: DataLoaderService,
    private contentService: ContentService,
    private agentService: AgentService
  ) {}

  /**
   * Get full trust badge for a content node.
   * Includes all badge details, warnings, and available actions.
   */
  getBadge(contentId: string): Observable<TrustBadge> {
    return forkJoin({
      content: this.contentService.getContent(contentId),
      attestations: this.dataLoader.getAttestationsForContent(contentId)
    }).pipe(
      map(({ content, attestations }) => this.computeBadge(content, attestations)),
      catchError(err => {
        console.error(`[TrustBadgeService] Failed to get badge for ${contentId}`, err);
        return of(this.createUnverifiedBadge(contentId));
      })
    );
  }

  /**
   * Get compact badge for list/card views.
   */
  getCompactBadge(contentId: string): Observable<CompactTrustBadge> {
    return this.getBadge(contentId).pipe(
      map(badge => toCompactBadge(badge))
    );
  }

  /**
   * Get badges for multiple content nodes (bulk operation).
   * Returns a Map for easy lookup.
   */
  getBadgesForContent(contentIds: string[]): Observable<Map<string, TrustBadge>> {
    if (contentIds.length === 0) {
      return of(new Map());
    }

    const badgeRequests = contentIds.map(id =>
      this.getBadge(id).pipe(
        map(badge => ({ id, badge }))
      )
    );

    return forkJoin(badgeRequests).pipe(
      map(results => {
        const map = new Map<string, TrustBadge>();
        results.forEach(({ id, badge }) => map.set(id, badge));
        return map;
      })
    );
  }

  /**
   * Get compact badges for multiple content nodes.
   */
  getCompactBadgesForContent(contentIds: string[]): Observable<Map<string, CompactTrustBadge>> {
    return this.getBadgesForContent(contentIds).pipe(
      map(badges => {
        const compactMap = new Map<string, CompactTrustBadge>();
        badges.forEach((badge, id) => compactMap.set(id, toCompactBadge(badge)));
        return compactMap;
      })
    );
  }

  // ===========================================================================
  // Unified Trust Indicators (Badges + Flags)
  // ===========================================================================

  /**
   * Get unified trust indicators for a content node.
   * This combines badges (positive) and flags (negative) into a single sorted list.
   */
  getIndicators(contentId: string): Observable<TrustIndicatorSet> {
    return forkJoin({
      content: this.contentService.getContent(contentId),
      attestations: this.dataLoader.getAttestationsForContent(contentId)
    }).pipe(
      map(({ content, attestations }) => this.computeIndicatorSet(content, attestations)),
      catchError(err => {
        console.error(`[TrustBadgeService] Failed to get indicators for ${contentId}`, err);
        return of(this.createEmptyIndicatorSet(contentId));
      })
    );
  }

  /**
   * Get indicators for multiple content nodes.
   */
  getIndicatorsForContent(contentIds: string[]): Observable<Map<string, TrustIndicatorSet>> {
    if (contentIds.length === 0) {
      return of(new Map());
    }

    const requests = contentIds.map(id =>
      this.getIndicators(id).pipe(map(set => ({ id, set })))
    );

    return forkJoin(requests).pipe(
      map(results => {
        const map = new Map<string, TrustIndicatorSet>();
        results.forEach(({ id, set }) => map.set(id, set));
        return map;
      })
    );
  }

  /**
   * Compute unified indicator set from content and attestations.
   */
  private computeIndicatorSet(
    content: ContentNode,
    attestations: ContentAttestation[]
  ): TrustIndicatorSet {
    const activeAttestations = attestations.filter(a => a.status === 'active');
    const attestationTypes = activeAttestations.map(a => a.attestationType);
    const reach = content.reach || 'commons';
    const trustScore = content.trustScore ?? this.computeTrustScore(activeAttestations);
    const flags = content.flags || [];
    const hasWarnings = flags.length > 0;

    const trustLevel = calculateTrustLevel(reach, attestationTypes, hasWarnings);

    // Build positive indicators from attestations
    const badges: TrustIndicator[] = activeAttestations.map(att => {
      const config = ATTESTATION_BADGE_CONFIG[att.attestationType];
      const badge: BadgeDisplay = {
        ...config,
        attestationType: att.attestationType,
        grantedBy: att.grantedBy.grantorName || att.grantedBy.grantorId,
        grantedAt: att.grantedAt
      };
      return badgeToIndicator(badge, ATTESTATION_PRIORITY[att.attestationType]);
    });

    // Build negative indicators from flags
    const flagIndicators: TrustIndicator[] = flags.map(flag => {
      const config = WARNING_CONFIG[flag.type];
      const warning: BadgeWarning = {
        ...config,
        flaggedAt: flag.flaggedAt
      };
      return warningToIndicator(warning);
    });

    // Combine and sort by priority (flags first if present, then badges)
    const allIndicators = [...flagIndicators, ...badges].sort((a, b) => {
      // Negative indicators (flags) come first when present
      if (a.polarity !== b.polarity) {
        return a.polarity === 'negative' ? -1 : 1;
      }
      // Then sort by priority (higher first)
      return b.priority - a.priority;
    });

    // Primary is first indicator (highest priority flag if any, else highest badge)
    const primary = allIndicators[0] || null;

    // Generate text
    const summary = generateTrustSummary(reach, attestationTypes, trustScore);
    const ariaLabel = generateAriaLabel(content.title, reach, trustLevel, hasWarnings);

    return {
      contentId: content.id,
      indicators: allIndicators,
      badges,
      flags: flagIndicators,
      primary,
      trustLevel,
      trustPercentage: Math.round(trustScore * 100),
      reach,
      summary,
      ariaLabel
    };
  }

  /**
   * Create empty indicator set for error cases.
   */
  private createEmptyIndicatorSet(contentId: string): TrustIndicatorSet {
    return {
      contentId,
      indicators: [],
      badges: [],
      flags: [],
      primary: null,
      trustLevel: 'unverified',
      trustPercentage: 0,
      reach: 'commons',
      summary: 'No trust information available.',
      ariaLabel: 'Content with no trust indicators.'
    };
  }

  // ===========================================================================
  // Badge Computation
  // ===========================================================================

  /**
   * Compute full trust badge from content and attestations.
   */
  private computeBadge(content: ContentNode, attestations: ContentAttestation[]): TrustBadge {
    const activeAttestations = attestations.filter(a => a.status === 'active');
    const attestationTypes = activeAttestations.map(a => a.attestationType);
    const reach = content.reach || 'commons';
    const trustScore = content.trustScore ?? this.computeTrustScore(activeAttestations);
    const flags = content.flags || [];
    const hasWarnings = flags.length > 0;

    const trustLevel = calculateTrustLevel(reach, attestationTypes, hasWarnings);

    // Build primary badge (highest priority attestation or reach)
    const primary = this.getPrimaryBadge(reach, activeAttestations);

    // Build secondary badges
    const secondary = this.getSecondaryBadges(reach, activeAttestations, primary);

    // Build warnings
    const warnings = this.buildWarnings(flags);

    // Generate text content
    const summary = generateTrustSummary(reach, attestationTypes, trustScore);
    const ariaLabel = generateAriaLabel(content.title, reach, trustLevel, hasWarnings);

    // Determine available actions
    const actions = this.getAvailableActions(content, activeAttestations);

    return {
      contentId: content.id,
      primary,
      secondary,
      trustLevel,
      trustPercentage: Math.round(trustScore * 100),
      reach,
      reachLabel: REACH_BADGE_CONFIG[reach].label,
      hasWarnings,
      warnings,
      summary,
      ariaLabel,
      actions
    };
  }

  /**
   * Get the primary (most important) badge to display.
   */
  private getPrimaryBadge(reach: ContentReach, attestations: ContentAttestation[]): BadgeDisplay {
    // Priority order for primary badge
    const priorityOrder: ContentAttestationType[] = [
      'governance-ratified',
      'curriculum-canonical',
      'peer-reviewed',
      'steward-approved',
      'safety-reviewed',
      'community-endorsed',
      'author-verified'
    ];

    // Find highest priority attestation
    for (const type of priorityOrder) {
      const attestation = attestations.find(a => a.attestationType === type);
      if (attestation) {
        const config = ATTESTATION_BADGE_CONFIG[type];
        return {
          ...config,
          attestationType: type,
          grantedBy: attestation.grantedBy.grantorName || attestation.grantedBy.grantorId,
          grantedAt: attestation.grantedAt
        };
      }
    }

    // No attestations - use reach as primary
    const reachConfig = REACH_BADGE_CONFIG[reach];
    return {
      type: 'reach',
      icon: reachConfig.icon,
      label: reachConfig.label,
      description: reachConfig.description,
      color: reachConfig.color,
      verified: false
    };
  }

  /**
   * Get secondary badges (other attestations besides primary).
   */
  private getSecondaryBadges(
    reach: ContentReach,
    attestations: ContentAttestation[],
    primary: BadgeDisplay
  ): BadgeDisplay[] {
    const secondary: BadgeDisplay[] = [];

    // Add reach badge if not primary
    if (primary.type !== 'reach') {
      const reachConfig = REACH_BADGE_CONFIG[reach];
      secondary.push({
        type: 'reach',
        icon: reachConfig.icon,
        label: reachConfig.label,
        description: reachConfig.description,
        color: reachConfig.color,
        verified: false
      });
    }

    // Add other attestation badges
    for (const attestation of attestations) {
      if (attestation.attestationType === primary.attestationType) {
        continue; // Skip primary
      }

      const config = ATTESTATION_BADGE_CONFIG[attestation.attestationType];
      secondary.push({
        ...config,
        attestationType: attestation.attestationType,
        grantedBy: attestation.grantedBy.grantorName || attestation.grantedBy.grantorId,
        grantedAt: attestation.grantedAt
      });
    }

    return secondary;
  }

  /**
   * Build warning displays from content flags.
   */
  private buildWarnings(flags: ContentFlag[]): BadgeWarning[] {
    return flags.map(flag => {
      const config = WARNING_CONFIG[flag.type];
      return {
        ...config,
        flaggedAt: flag.flaggedAt
      };
    });
  }

  /**
   * Compute trust score from attestations.
   */
  private computeTrustScore(attestations: ContentAttestation[]): number {
    if (attestations.length === 0) {
      return 0;
    }

    // Weight each attestation type
    const weights: Record<ContentAttestationType, number> = {
      'governance-ratified': 0.5,
      'curriculum-canonical': 0.4,
      'peer-reviewed': 0.4,
      'steward-approved': 0.3,
      'safety-reviewed': 0.2,
      'accuracy-verified': 0.2,
      'community-endorsed': 0.15,
      'accessibility-checked': 0.1,
      'license-cleared': 0.1,
      'author-verified': 0.1
    };

    const totalWeight = Object.values(weights).reduce((sum, w) => sum + w, 0);
    let earnedWeight = 0;

    for (const attestation of attestations) {
      earnedWeight += weights[attestation.attestationType] || 0;
    }

    // Cap at 1.0
    return Math.min(earnedWeight / totalWeight * 2, 1.0);
  }

  /**
   * Determine what actions are available to the current user.
   */
  private getAvailableActions(
    content: ContentNode,
    attestations: ContentAttestation[]
  ): BadgeAction[] {
    const actions: BadgeAction[] = [];
    const currentAgentId = this.agentService.getCurrentAgentId();
    const agentAttestations = this.agentService.getAttestations();

    // View trust profile - always available
    actions.push({
      action: 'view-trust-profile',
      label: 'View Trust Profile',
      icon: '🔍',
      available: true,
      route: `/lamad/resource/${content.id}/trust`
    });

    // Endorse - available to community members
    const hasEndorsed = attestations.some(
      a => a.attestationType === 'community-endorsed' &&
           a.grantedBy.grantorId === currentAgentId
    );

    if (!hasEndorsed && currentAgentId) {
      actions.push({
        action: 'endorse',
        label: 'Endorse',
        icon: '👍',
        available: agentAttestations.includes('community-member'),
        unavailableReason: !agentAttestations.includes('community-member')
          ? 'Requires community membership'
          : undefined
      });
    }

    // Request attestation - available to content author
    if (content.authorId === currentAgentId) {
      actions.push({
        action: 'request-attestation',
        label: 'Request Review',
        icon: '📝',
        available: true,
        route: `/lamad/resource/${content.id}/attestation/request`
      });
    }

    // Report - available to authenticated users
    if (currentAgentId) {
      actions.push({
        action: 'report',
        label: 'Report Issue',
        icon: '🚩',
        available: true
      });
    }

    return actions;
  }

  /**
   * Create a default badge for content with no attestations.
   */
  private createUnverifiedBadge(contentId: string): TrustBadge {
    const reach: ContentReach = 'commons'; // Default for legacy
    const reachConfig = REACH_BADGE_CONFIG[reach];

    return {
      contentId,
      primary: {
        type: 'reach',
        icon: reachConfig.icon,
        label: reachConfig.label,
        description: reachConfig.description,
        color: reachConfig.color,
        verified: false
      },
      secondary: [],
      trustLevel: 'unverified',
      trustPercentage: 0,
      reach,
      reachLabel: reachConfig.label,
      hasWarnings: false,
      warnings: [],
      summary: 'Content not yet verified.',
      ariaLabel: 'Unverified content. No trust attestations available.',
      actions: [{
        action: 'view-trust-profile',
        label: 'View Trust Profile',
        icon: '🔍',
        available: true,
        route: `/lamad/resource/${contentId}/trust`
      }]
    };
  }

  // ===========================================================================
  // Reach Helpers
  // ===========================================================================

  /**
   * Check if a reach level meets a minimum requirement.
   */
  meetsReachRequirement(contentReach: ContentReach, requiredReach: ContentReach): boolean {
    return CONTENT_REACH_LEVELS[contentReach] >= CONTENT_REACH_LEVELS[requiredReach];
  }

  /**
   * Get the next reach level (for "earn more trust" UI hints).
   */
  getNextReachLevel(currentReach: ContentReach): ContentReach | null {
    const levels: ContentReach[] = ['private', 'invited', 'local', 'community', 'federated', 'commons'];
    const currentIndex = levels.indexOf(currentReach);
    if (currentIndex < levels.length - 1) {
      return levels[currentIndex + 1];
    }
    return null; // Already at commons
  }

  /**
   * Get attestations needed to reach the next level.
   */
  getAttestationsNeededForNextLevel(
    currentReach: ContentReach,
    currentAttestations: ContentAttestationType[]
  ): ContentAttestationType[] {
    const nextReach = this.getNextReachLevel(currentReach);
    if (!nextReach) {
      return []; // Already at max
    }

    // Map of reach levels to required attestations
    const requirements: Record<ContentReach, ContentAttestationType[]> = {
      'private': [],
      'invited': ['author-verified'],
      'local': ['author-verified'],
      'community': ['steward-approved', 'community-endorsed', 'safety-reviewed'],
      'federated': ['peer-reviewed', 'safety-reviewed'],
      'commons': ['governance-ratified', 'safety-reviewed', 'license-cleared']
    };

    const needed = requirements[nextReach] || [];
    return needed.filter(type => !currentAttestations.includes(type));
  }
}
