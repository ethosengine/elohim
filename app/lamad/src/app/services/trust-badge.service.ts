import { Injectable, inject } from '@angular/core';

// @coverage: 93.1% (2026-03-03)

import { catchError, map } from 'rxjs/operators';

import { forkJoin, Observable, of } from 'rxjs';

import {
  ATTESTATION_BADGE_CONFIG,
  ATTESTATION_PRIORITY,
  BadgeAction,
  BadgeDisplay,
  BadgeWarning,
  CompactTrustBadge,
  ContentAttestationType,
  ContentReach,
  REACH_BADGE_CONFIG,
  TrustBadge,
  TrustIndicator,
  TrustIndicatorSet,
  WARNING_CONFIG,
  badgeToIndicator,
  calculateTrustLevel,
  generateAriaLabel,
  generateTrustSummary,
  toCompactBadge,
  warningToIndicator,
} from '../models/trust-badge.model';
import {
  ContentAttestation,
  CONTENT_REACH_LEVELS,
} from '../models/content-attestation.model';
import { ContentFlag, ContentNode } from '../models/content-node.model';
import { ContentService } from './content.service';
import { LAMAD_AGENT, type ILamadAgent } from '../interfaces/agent.interface';
import { DataLoaderService } from './data-loader.service';

/**
 * TrustBadgeService - Computes UI-ready trust badges for content.
 *
 * P-migrated from @app/elohim/services/trust-badge.service (Slice 2.1c).
 * Moved here because all consumers were in lamad and the service imports
 * only lamad models. AgentService dep replaced with LAMAD_AGENT token.
 */
const ATT_GOVERNANCE_RATIFIED: ContentAttestationType = 'governance-ratified';
const ATT_PEER_REVIEWED: ContentAttestationType = 'peer-reviewed';
const ATT_STEWARD_APPROVED: ContentAttestationType = 'steward-approved';
const ATT_SAFETY_REVIEWED: ContentAttestationType = 'safety-reviewed';
const ATT_COMMUNITY_ENDORSED: ContentAttestationType = 'community-endorsed';
const ATT_AUTHOR_VERIFIED: ContentAttestationType = 'author-verified';

@Injectable({ providedIn: 'root' })
export class TrustBadgeService {
  private readonly dataLoader = inject(DataLoaderService);
  private readonly contentService = inject(ContentService);
  private readonly agentService: ILamadAgent = inject(LAMAD_AGENT);

  getBadge(contentId: string): Observable<TrustBadge> {
    return forkJoin({
      content: this.contentService.getContent(contentId),
      attestations: this.dataLoader.getAttestationsForContent(contentId),
    }).pipe(
      map(({ content, attestations }) => this.computeBadge(content, attestations)),
      catchError(_err => {
        return of(this.createUnverifiedBadge(contentId));
      })
    );
  }

  getCompactBadge(contentId: string): Observable<CompactTrustBadge> {
    return this.getBadge(contentId).pipe(map(badge => toCompactBadge(badge)));
  }

  getBadgesForContent(contentIds: string[]): Observable<Map<string, TrustBadge>> {
    if (contentIds.length === 0) {
      return of(new Map());
    }

    const badgeRequests = contentIds.map(id =>
      this.getBadge(id).pipe(map(badge => ({ id, badge })))
    );

    return forkJoin(badgeRequests).pipe(
      map(results => {
        const map = new Map<string, TrustBadge>();
        results.forEach(({ id, badge }) => map.set(id, badge));
        return map;
      })
    );
  }

  getCompactBadgesForContent(contentIds: string[]): Observable<Map<string, CompactTrustBadge>> {
    return this.getBadgesForContent(contentIds).pipe(
      map(badges => {
        const compactMap = new Map<string, CompactTrustBadge>();
        badges.forEach((badge, id) => compactMap.set(id, toCompactBadge(badge)));
        return compactMap;
      })
    );
  }

  getIndicators(contentId: string): Observable<TrustIndicatorSet> {
    return forkJoin({
      content: this.contentService.getContent(contentId),
      attestations: this.dataLoader.getAttestationsForContent(contentId),
    }).pipe(
      map(({ content, attestations }) => this.computeIndicatorSet(content, attestations)),
      catchError(_err => {
        return of(this.createEmptyIndicatorSet(contentId));
      })
    );
  }

  getIndicatorsForContent(contentIds: string[]): Observable<Map<string, TrustIndicatorSet>> {
    if (contentIds.length === 0) {
      return of(new Map());
    }

    const requests = contentIds.map(id => this.getIndicators(id).pipe(map(set => ({ id, set }))));

    return forkJoin(requests).pipe(
      map(results => {
        const map = new Map<string, TrustIndicatorSet>();
        results.forEach(({ id, set }) => map.set(id, set));
        return map;
      })
    );
  }

  private computeIndicatorSet(
    content: ContentNode,
    attestations: ContentAttestation[]
  ): TrustIndicatorSet {
    const activeAttestations = attestations.filter(a => a.status === 'active');
    const attestationTypes = activeAttestations.map(a => a.attestationType);
    const reach = content.reach ?? 'commons';
    const trustScore = content.trustScore ?? this.computeTrustScore(activeAttestations);
    const flags = content.flags ?? [];
    const hasWarnings = flags.length > 0;

    const trustLevel = calculateTrustLevel(reach, attestationTypes, hasWarnings);

    const badges: TrustIndicator[] = activeAttestations.map(att => {
      const config = ATTESTATION_BADGE_CONFIG[att.attestationType];
      const badge: BadgeDisplay = {
        ...config,
        attestationType: att.attestationType,
        grantedBy: att.grantedBy.grantorName ?? att.grantedBy.grantorId,
        grantedAt: att.grantedAt,
      };
      return badgeToIndicator(badge, ATTESTATION_PRIORITY[att.attestationType]);
    });

    const flagIndicators: TrustIndicator[] = flags.map(flag => {
      const config = WARNING_CONFIG[flag.type];
      const warning: BadgeWarning = {
        ...config,
        flaggedAt: flag.flaggedAt,
      };
      return warningToIndicator(warning);
    });

    const allIndicators = [...flagIndicators, ...badges].sort((a, b) => {
      if (a.polarity !== b.polarity) {
        return a.polarity === 'negative' ? -1 : 1;
      }
      return b.priority - a.priority;
    });

    const primary = allIndicators[0] || null;

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
      ariaLabel,
    };
  }

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
      ariaLabel: 'Content with no trust indicators.',
    };
  }

  private computeBadge(content: ContentNode, attestations: ContentAttestation[]): TrustBadge {
    const activeAttestations = attestations.filter(a => a.status === 'active');
    const attestationTypes = activeAttestations.map(a => a.attestationType);
    const reach = content.reach ?? 'commons';
    const trustScore = content.trustScore ?? this.computeTrustScore(activeAttestations);
    const flags = content.flags ?? [];
    const hasWarnings = flags.length > 0;

    const trustLevel = calculateTrustLevel(reach, attestationTypes, hasWarnings);
    const primary = this.getPrimaryBadge(reach, activeAttestations);
    const secondary = this.getSecondaryBadges(reach, activeAttestations, primary);
    const warnings = this.buildWarnings(flags);
    const summary = generateTrustSummary(reach, attestationTypes, trustScore);
    const ariaLabel = generateAriaLabel(content.title, reach, trustLevel, hasWarnings);
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
      actions,
    };
  }

  private getPrimaryBadge(reach: ContentReach, attestations: ContentAttestation[]): BadgeDisplay {
    const priorityOrder: ContentAttestationType[] = [
      ATT_GOVERNANCE_RATIFIED,
      'curriculum-canonical',
      ATT_PEER_REVIEWED,
      ATT_STEWARD_APPROVED,
      ATT_SAFETY_REVIEWED,
      ATT_COMMUNITY_ENDORSED,
      ATT_AUTHOR_VERIFIED,
    ];

    for (const type of priorityOrder) {
      const attestation = attestations.find(a => a.attestationType === type);
      if (attestation) {
        const config = ATTESTATION_BADGE_CONFIG[type];
        return {
          ...config,
          attestationType: type,
          grantedBy: attestation.grantedBy.grantorName ?? attestation.grantedBy.grantorId,
          grantedAt: attestation.grantedAt,
        };
      }
    }

    const reachConfig = REACH_BADGE_CONFIG[reach];
    return {
      type: 'reach',
      icon: reachConfig.icon,
      label: reachConfig.label,
      description: reachConfig.description,
      color: reachConfig.color,
      verified: false,
    };
  }

  private getSecondaryBadges(
    reach: ContentReach,
    attestations: ContentAttestation[],
    primary: BadgeDisplay
  ): BadgeDisplay[] {
    const secondary: BadgeDisplay[] = [];

    if (primary.type !== 'reach') {
      const reachConfig = REACH_BADGE_CONFIG[reach];
      secondary.push({
        type: 'reach',
        icon: reachConfig.icon,
        label: reachConfig.label,
        description: reachConfig.description,
        color: reachConfig.color,
        verified: false,
      });
    }

    for (const attestation of attestations) {
      if (attestation.attestationType === primary.attestationType) {
        continue;
      }

      const config = ATTESTATION_BADGE_CONFIG[attestation.attestationType];
      secondary.push({
        ...config,
        attestationType: attestation.attestationType,
        grantedBy: attestation.grantedBy.grantorName ?? attestation.grantedBy.grantorId,
        grantedAt: attestation.grantedAt,
      });
    }

    return secondary;
  }

  private buildWarnings(flags: ContentFlag[]): BadgeWarning[] {
    return flags.map(flag => {
      const config = WARNING_CONFIG[flag.type];
      return {
        ...config,
        flaggedAt: flag.flaggedAt,
      };
    });
  }

  private computeTrustScore(attestations: ContentAttestation[]): number {
    if (attestations.length === 0) {
      return 0;
    }

    const weights: Partial<Record<ContentAttestationType, number>> = {
      [ATT_GOVERNANCE_RATIFIED]: 0.5,
      'curriculum-canonical': 0.4,
      [ATT_PEER_REVIEWED]: 0.4,
      [ATT_STEWARD_APPROVED]: 0.3,
      [ATT_SAFETY_REVIEWED]: 0.2,
      'accuracy-verified': 0.2,
      [ATT_COMMUNITY_ENDORSED]: 0.15,
      'accessibility-checked': 0.1,
      'license-cleared': 0.1,
      [ATT_AUTHOR_VERIFIED]: 0.1,
    };

    const totalWeight = Object.values(weights).reduce((sum, w) => sum + w, 0);
    let earnedWeight = 0;

    for (const attestation of attestations) {
      earnedWeight += weights[attestation.attestationType] ?? 0;
    }

    return Math.min((earnedWeight / totalWeight) * 2, 1);
  }

  private getAvailableActions(
    content: ContentNode,
    attestations: ContentAttestation[]
  ): BadgeAction[] {
    const actions: BadgeAction[] = [];
    const currentAgentId = this.agentService.getCurrentAgentId();
    const agentAttestations = this.agentService.getAttestations();

    actions.push({
      action: 'view-trust-profile',
      label: 'View Trust Profile',
      icon: '🔍',
      available: true,
      route: `/resource/${content.id}/trust`,
    });

    const hasEndorsed = attestations.some(
      a => a.attestationType === ATT_COMMUNITY_ENDORSED && a.grantedBy.grantorId === currentAgentId
    );

    if (!hasEndorsed && currentAgentId) {
      actions.push({
        action: 'endorse',
        label: 'Endorse',
        icon: '👍',
        available: agentAttestations.includes('community-member'),
        unavailableReason: agentAttestations.includes('community-member')
          ? undefined
          : 'Requires community membership',
      });
    }

    if (content.authorId === currentAgentId) {
      actions.push({
        action: 'request-attestation',
        label: 'Request Review',
        icon: '📝',
        available: true,
        route: `/resource/${content.id}/attestation/request`,
      });
    }

    if (currentAgentId) {
      actions.push({
        action: 'report',
        label: 'Report Issue',
        icon: '🚩',
        available: true,
      });
    }

    return actions;
  }

  private createUnverifiedBadge(contentId: string): TrustBadge {
    const reach: ContentReach = 'commons';
    const reachConfig = REACH_BADGE_CONFIG[reach];

    return {
      contentId,
      primary: {
        type: 'reach',
        icon: reachConfig.icon,
        label: reachConfig.label,
        description: reachConfig.description,
        color: reachConfig.color,
        verified: false,
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
      actions: [
        {
          action: 'view-trust-profile',
          label: 'View Trust Profile',
          icon: '🔍',
          available: true,
          route: `/resource/${contentId}/trust`,
        },
      ],
    };
  }

  meetsReachRequirement(contentReach: ContentReach, requiredReach: ContentReach): boolean {
    return CONTENT_REACH_LEVELS[contentReach] >= CONTENT_REACH_LEVELS[requiredReach];
  }

  getNextReachLevel(currentReach: ContentReach): ContentReach | null {
    const levels: ContentReach[] = [
      'private',
      'invited',
      'local',
      'neighborhood',
      'municipal',
      'bioregional',
      'regional',
      'commons',
    ];
    const currentIndex = levels.indexOf(currentReach);
    if (currentIndex < levels.length - 1) {
      return levels[currentIndex + 1];
    }
    return null;
  }

  getAttestationsNeededForNextLevel(
    currentReach: ContentReach,
    currentAttestations: ContentAttestationType[]
  ): ContentAttestationType[] {
    const nextReach = this.getNextReachLevel(currentReach);
    if (!nextReach) {
      return [];
    }

    const requirements: Record<ContentReach, ContentAttestationType[]> = {
      private: [],
      invited: [ATT_AUTHOR_VERIFIED],
      local: [ATT_AUTHOR_VERIFIED],
      neighborhood: [ATT_AUTHOR_VERIFIED, ATT_COMMUNITY_ENDORSED],
      municipal: [ATT_STEWARD_APPROVED, ATT_COMMUNITY_ENDORSED, ATT_SAFETY_REVIEWED],
      bioregional: [ATT_STEWARD_APPROVED, ATT_PEER_REVIEWED, ATT_SAFETY_REVIEWED],
      regional: [ATT_PEER_REVIEWED, ATT_GOVERNANCE_RATIFIED],
      commons: [ATT_GOVERNANCE_RATIFIED, ATT_SAFETY_REVIEWED, 'license-cleared'],
    };

    const needed = requirements[nextReach] || [];
    return needed.filter(type => !currentAttestations.includes(type));
  }
}
