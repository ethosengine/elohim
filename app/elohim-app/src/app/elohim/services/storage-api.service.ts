/**
 * Storage API Service - HTTP Client for elohim-storage DB endpoints
 *
 * Provides access to the SQLite-backed storage layer endpoints:
 * - /db/relationships - Content relationships with full metadata
 * - /db/human-relationships - Human social graph relationships
 * - /db/presences - Contributor presence lifecycle
 * - /db/events - Economic events (hREA)
 * - /db/mastery - Content mastery tracking (Bloom's)
 *
 * ## Architecture
 *
 * ```
 * Angular (thin UI)
 *     │
 *     ▼
 * StorageApiService  ◄── This service
 *     │
 *     ▼
 * elohim-storage (Rust) ──► SQLite DB
 *     │
 *     ▼
 * Diesel ORM
 * ```
 *
 * ## Type Strategy
 *
 * Wire types (snake_case) come from @elohim/storage-client/generated (ts-rs from Diesel)
 * View types (camelCase) and transforms come from @app/elohim/adapters/storage-types.adapter
 * Query/input types come from domain model files (domain-specific concerns)
 */

import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

// @coverage: 67.9% (2026-02-24)

import { catchError, map, tap, timeout } from 'rxjs/operators';

import { Observable, of, throwError } from 'rxjs';

import {
  HumanRelationshipView,
  ContributorPresenceView,
  withFullyConsentedFlag,
  withFullyConsentedFlags,
  withEstablishingContentIds,
  withEstablishingContentIdsArray,
} from '@app/elohim/adapters/storage-types.adapter';
import { EventQuery } from '@app/elohim/models/economic-event.model';
import { extractGateFromResponse } from '@app/elohim/models/gated-response.model';
import { handleGateError } from '@app/elohim/services/gate-error.handler';
import { GateService } from '@app/elohim/services/gate.service';
import {
  HumanRelationshipQuery,
  CreateHumanRelationshipInput,
} from '@app/imagodei/models/human-relationship.model';
import { CreateRelationshipInput } from '@app/lamad/models/content-node.model';
import {
  AllocationQuery,
  CreateAllocationInput,
  UpdateAllocationInput,
  FileDisputeInput,
  ResolveDisputeInput,
  BulkAllocationResult,
} from '@app/lamad/models/stewardship-allocation.model';

import { environment } from '../../../environments/environment';
import {
  type IStorageApi,
  type IStorageWriter,
  type ContentFilters,
  type PathFilters,
  type RelationshipFilters,
  type UpdateContentPatch,
  type CreatePresenceInput,
  type CreateMasteryInput,
  type CreateEventInput,
} from '../interfaces';

import type {
  RelationshipView,
  RelationshipWithContentView,
  HumanRelationshipView as HumanRelationshipViewBase,
  ContributorPresenceView as ContributorPresenceViewBase,
  EconomicEventView,
  ContentMasteryView,
  ContentWithTagsView,
  CreateContentInputView,
  PathView,
  PathWithDetailsView,
  StewardshipAllocationView,
  ContentStewardshipView,
} from '@elohim/storage-client/generated';

/**
 * Query parameters for presences
 */
export interface PresenceQuery {
  presenceState?: string;
  stewardId?: string;
  claimedAgentId?: string;
  minRecognitionScore?: number;
  limit?: number;
  offset?: number;
}

/**
 * Query parameters for mastery
 */
export interface MasteryQuery {
  humanId?: string;
  contentId?: string;
  minLevel?: string;
  needsRefresh?: boolean;
  limit?: number;
  offset?: number;
}

@Injectable({
  providedIn: 'root',
})
export class StorageApiService implements IStorageApi, IStorageWriter {
  /** Base URL for elohim-storage API */
  private readonly baseUrl: string;

  /** App ID for multi-tenant scoping */
  private readonly appId = 'lamad';

  /** Default request timeout in milliseconds */
  private readonly defaultTimeoutMs = 30000;

  private readonly http = inject(HttpClient);
  private readonly gateService = inject(GateService);

  constructor() {
    // Use storageUrl from environment or fall back to doorway URL
    this.baseUrl =
      environment.holochain?.storageUrl ??
      (environment as unknown as { client?: { doorwayUrl?: string } }).client?.doorwayUrl ??
      '';
  }

  // ==========================================================================
  // IStorageApi — Read-oriented interface methods
  // ==========================================================================

  /** Get a single content item by ID, with tags. */
  getContent(id: string): Observable<ContentWithTagsView | null> {
    return this.http
      .get<ContentWithTagsView>(`${this.baseUrl}/db/content/${encodeURIComponent(id)}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => {
          if ((error as Record<string, unknown>)['status'] === 404) {
            return of(null);
          }
          return this.handleError('getContent', error);
        })
      );
  }

  /** List content items with optional filters. */
  getContents(filters?: ContentFilters): Observable<ContentWithTagsView[]> {
    let params = new HttpParams().set('appId', this.appId);

    if (filters?.contentType) params = params.set('contentType', filters.contentType);
    if (filters?.tags?.length) params = params.set('tags', filters.tags.join(','));
    if (filters?.reach) params = params.set('reach', filters.reach);
    if (filters?.search) params = params.set('search', filters.search);
    if (filters?.limit) params = params.set('limit', filters.limit.toString());
    if (filters?.offset) params = params.set('offset', filters.offset.toString());

    return this.http.get<ContentWithTagsView[]>(`${this.baseUrl}/db/content`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getContents', error))
    );
  }

  /** Create a new content node. Returns the created item with tags. */
  createContent(input: CreateContentInputView): Observable<ContentWithTagsView> {
    return this.http.post<ContentWithTagsView>(`${this.baseUrl}/db/content`, input).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('createContent', error))
    );
  }

  /** Partially update a content node (PATCH). Returns the updated item with tags. */
  updateContent(id: string, patch: UpdateContentPatch): Observable<ContentWithTagsView> {
    return this.http
      .patch<ContentWithTagsView>(`${this.baseUrl}/db/content/${encodeURIComponent(id)}`, patch)
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('updateContent', error))
      );
  }

  /** Get a single learning path by ID, with full details. */
  getPath(id: string): Observable<PathWithDetailsView | null> {
    return this.http
      .get<PathWithDetailsView>(`${this.baseUrl}/db/paths/${encodeURIComponent(id)}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => {
          if ((error as Record<string, unknown>)['status'] === 404) {
            return of(null);
          }
          return this.handleError('getPath', error);
        })
      );
  }

  /** List learning paths with optional filters. */
  getPaths(filters?: PathFilters): Observable<PathView[]> {
    let params = new HttpParams().set('appId', this.appId);

    if (filters?.pathType) params = params.set('pathType', filters.pathType);
    if (filters?.difficulty) params = params.set('difficulty', filters.difficulty);
    if (filters?.visibility) params = params.set('visibility', filters.visibility);
    if (filters?.tags?.length) params = params.set('tags', filters.tags.join(','));
    if (filters?.limit) params = params.set('limit', filters.limit.toString());
    if (filters?.offset) params = params.set('offset', filters.offset.toString());

    return this.http.get<PathView[]>(`${this.baseUrl}/db/paths`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getPaths', error))
    );
  }

  /** Get relationships for a content node, with joined content. */
  getRelatedContent(id: string): Observable<RelationshipWithContentView[]> {
    const params = new HttpParams()
      .set('appId', this.appId)
      .set('sourceId', id)
      .set('includeContent', 'true');

    return this.http
      .get<RelationshipWithContentView[]>(`${this.baseUrl}/db/relationships`, { params })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('getRelatedContent', error))
      );
  }

  /** Get mastery state for a human on specific content. */
  getMastery(humanId: string, contentId: string): Observable<ContentMasteryView | null> {
    const params = new HttpParams()
      .set('appId', this.appId)
      .set('humanId', humanId)
      .set('contentId', contentId);

    return this.http.get<ContentMasteryView[]>(`${this.baseUrl}/db/mastery`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      map(results => results[0] ?? null),
      catchError(error => this.handleError('getMastery', error))
    );
  }

  /** Get stewardship data for a content piece. */
  getStewardship(contentId: string): Observable<ContentStewardshipView | null> {
    return this.http
      .get<ContentStewardshipView>(`${this.baseUrl}/db/allocations/content/${contentId}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => {
          if ((error as Record<string, unknown>)['status'] === 404) {
            return of(null);
          }
          return this.handleError('getStewardship', error);
        })
      );
  }

  // ==========================================================================
  // Content Relationships
  // ==========================================================================

  /**
   * List content relationships with optional filters.
   * Query params use camelCase.
   */
  getRelationships(query?: RelationshipFilters): Observable<RelationshipView[]> {
    let params = new HttpParams().set('appId', this.appId);

    if (query?.sourceId) params = params.set('sourceId', query.sourceId);
    if (query?.targetId) params = params.set('targetId', query.targetId);
    if (query?.relationshipType) params = params.set('relationshipType', query.relationshipType);
    if (query?.minConfidence) params = params.set('minConfidence', query.minConfidence.toString());
    if (query?.inferenceSource) params = params.set('inferenceSource', query.inferenceSource);
    if (query?.bidirectionalOnly) params = params.set('bidirectionalOnly', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<RelationshipView[]>(`${this.baseUrl}/db/relationships`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getRelationships', error))
    );
  }

  /**
   * Get relationships for a specific content node.
   */
  getRelationshipsForContent(contentId: string): Observable<RelationshipView[]> {
    return this.getRelationships({ sourceId: contentId }).pipe(
      map(outgoing => {
        // Also get incoming relationships in parallel (could optimize with separate endpoint)
        return outgoing;
      })
    );
  }

  /**
   * Create a new relationship.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  createRelationship(input: CreateRelationshipInput): Observable<RelationshipView> {
    const body = {
      sourceId: input.sourceId,
      targetId: input.targetId,
      relationshipType: input.relationshipType,
      confidence: input.confidence ?? 1,
      inferenceSource: input.inferenceSource ?? 'author',
      createInverse: input.createInverse ?? false,
      inverseType: input.inverseType,
      provenanceChain: input.provenanceChain ?? null,
      metadata: input.metadataJson
        ? (JSON.parse(input.metadataJson) as Record<string, unknown>)
        : null,
    };

    return this.http.post<RelationshipView>(`${this.baseUrl}/db/relationships`, body).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('createRelationship', error))
    );
  }

  // ==========================================================================
  // Human Relationships
  // ==========================================================================

  /**
   * List human relationships with optional filters.
   */
  getHumanRelationships(query?: HumanRelationshipQuery): Observable<HumanRelationshipView[]> {
    let params = new HttpParams().set('appId', 'imagodei');

    if (query?.partyId) params = params.set('partyId', query.partyId);
    if (query?.partyAId) params = params.set('partyAId', query.partyAId);
    if (query?.partyBId) params = params.set('partyBId', query.partyBId);
    if (query?.relationshipType) params = params.set('relationshipType', query.relationshipType);
    if (query?.minIntimacyLevel) params = params.set('minIntimacyLevel', query.minIntimacyLevel);
    if (query?.fullyConsentedOnly) params = params.set('fullyConsentedOnly', 'true');
    if (query?.custodyEnabledOnly) params = params.set('custodyEnabledOnly', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http
      .get<HumanRelationshipViewBase[]>(`${this.baseUrl}/db/human-relationships`, { params })
      .pipe(
        timeout(this.defaultTimeoutMs),
        map(views => withFullyConsentedFlags(views)),
        catchError(error => this.handleError('getHumanRelationships', error))
      );
  }

  /**
   * Get human relationships for a specific person.
   */
  getRelationshipsForPerson(personId: string): Observable<HumanRelationshipView[]> {
    return this.getHumanRelationships({ partyId: personId });
  }

  /**
   * Create a new human relationship.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  createHumanRelationship(input: CreateHumanRelationshipInput): Observable<HumanRelationshipView> {
    const body = {
      partyAId: input.partyAId,
      partyBId: input.partyBId,
      relationshipType: input.relationshipType,
      intimacyLevel: input.intimacyLevel ?? 'recognition',
      isBidirectional: input.isBidirectional ?? false,
      context: input.context ?? null,
    };

    return this.http
      .post<HumanRelationshipViewBase>(`${this.baseUrl}/db/human-relationships`, body)
      .pipe(
        timeout(this.defaultTimeoutMs),
        map(view => withFullyConsentedFlag(view)),
        catchError(error => this.handleError('createHumanRelationship', error))
      );
  }

  // ==========================================================================
  // Contributor Presences
  // ==========================================================================

  /**
   * List contributor presences with optional filters.
   */
  getContributorPresences(query?: PresenceQuery): Observable<ContributorPresenceView[]> {
    let params = new HttpParams().set('appId', this.appId);

    if (query?.presenceState) params = params.set('presenceState', query.presenceState);
    if (query?.stewardId) params = params.set('stewardId', query.stewardId);
    if (query?.claimedAgentId) params = params.set('claimedAgentId', query.claimedAgentId);
    if (query?.minRecognitionScore)
      params = params.set('minRecognitionScore', query.minRecognitionScore.toString());
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http
      .get<ContributorPresenceViewBase[]>(`${this.baseUrl}/db/presences`, { params })
      .pipe(
        timeout(this.defaultTimeoutMs),
        map(views => withEstablishingContentIdsArray(views)),
        catchError(error => this.handleError('getContributorPresences', error))
      );
  }

  /**
   * Get a specific contributor presence.
   */
  getContributorPresence(presenceId: string): Observable<ContributorPresenceView | null> {
    return this.http
      .get<ContributorPresenceViewBase | null>(`${this.baseUrl}/db/presences/${presenceId}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        map(view => (view ? withEstablishingContentIds(view) : null)),
        catchError(error => {
          if ((error as Record<string, unknown>)['status'] === 404) {
            return of(null);
          }
          return this.handleError('getContributorPresence', error);
        })
      );
  }

  /**
   * Get presence for content (by establishing content ID).
   */
  getPresenceForContent(contentId: string): Observable<ContributorPresenceView[]> {
    // Query presences that have this content in their establishing_content_ids
    // This would need a backend query enhancement, for now get all and filter
    return this.getContributorPresences().pipe(
      map(presences => presences.filter(p => p.establishingContentIds?.includes(contentId)))
    );
  }

  /**
   * Create a new contributor presence.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  createContributorPresence(input: CreatePresenceInput): Observable<ContributorPresenceView> {
    const body = {
      displayName: input.displayName,
      establishingContentIds: input.establishingContentIds,
      externalIdentifiers: input.externalIdentifiers ?? null,
    };

    return this.http.post<ContributorPresenceViewBase>(`${this.baseUrl}/db/presences`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(view => withEstablishingContentIds(view)),
      catchError(error => this.handleError('createContributorPresence', error))
    );
  }

  // ==========================================================================
  // Economic Events
  // ==========================================================================

  /**
   * List economic events with optional filters.
   */
  getEconomicEvents(query?: EventQuery): Observable<EconomicEventView[]> {
    let params = new HttpParams().set('appId', 'shefa');

    if (query?.agentId) {
      params = params.set(query.agentRole === 'provider' ? 'provider' : 'receiver', query.agentId);
    }
    if (query?.actions?.length) params = params.set('action', query.actions.join(','));
    if (query?.eventTypes?.length)
      params = params.set('lamadEventType', query.eventTypes.join(','));
    if (query?.contentId) params = params.set('contentId', query.contentId);
    if (query?.pathId) params = params.set('pathId', query.pathId);
    if (query?.from) params = params.set('from', query.from);
    if (query?.to) params = params.set('to', query.to);
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<EconomicEventView[]>(`${this.baseUrl}/db/events`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getEconomicEvents', error))
    );
  }

  /**
   * Get events for a specific agent.
   */
  getEventsForAgent(
    agentId: string,
    role?: 'provider' | 'receiver' | 'either'
  ): Observable<EconomicEventView[]> {
    return this.getEconomicEvents({ agentId, agentRole: role ?? 'either' });
  }

  /**
   * Get events for a specific content.
   */
  getEventsForContent(contentId: string): Observable<EconomicEventView[]> {
    return this.getEconomicEvents({ contentId });
  }

  // ==========================================================================
  // Content Mastery
  // ==========================================================================

  /**
   * List content mastery records with optional filters.
   */
  getMasteryRecords(query?: MasteryQuery): Observable<ContentMasteryView[]> {
    let params = new HttpParams().set('appId', this.appId);

    if (query?.humanId) params = params.set('humanId', query.humanId);
    if (query?.contentId) params = params.set('contentId', query.contentId);
    if (query?.minLevel) params = params.set('minLevel', query.minLevel);
    if (query?.needsRefresh) params = params.set('needsRefresh', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<ContentMasteryView[]>(`${this.baseUrl}/db/mastery`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getMasteryRecords', error))
    );
  }

  /**
   * Get mastery for a specific human.
   */
  getMasteryForHuman(humanId: string): Observable<ContentMasteryView[]> {
    return this.http.get<ContentMasteryView[]>(`${this.baseUrl}/db/mastery/human/${humanId}`).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getMasteryForHuman', error))
    );
  }

  /**
   * Get mastery state for specific content IDs.
   */
  getMasteryState(
    humanId: string,
    contentIds: string[]
  ): Observable<Map<string, ContentMasteryView>> {
    return this.getMasteryForHuman(humanId).pipe(
      map(masteries => {
        const masteryMap = new Map<string, ContentMasteryView>();
        for (const m of masteries) {
          if (contentIds.includes(m.contentId)) {
            masteryMap.set(m.contentId, m);
          }
        }
        return masteryMap;
      })
    );
  }

  /**
   * Create or update mastery record.
   * API accepts camelCase InputView.
   */
  upsertMastery(input: CreateMasteryInput): Observable<ContentMasteryView> {
    const body = {
      humanId: input.humanId,
      contentId: input.contentId,
      masteryLevel: input.masteryLevel ?? 'seen',
      engagementType: input.engagementType ?? 'view',
    };

    return this.http.post<ContentMasteryView>(`${this.baseUrl}/db/mastery`, body).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('upsertMastery', error))
    );
  }

  // ==========================================================================
  // Human Relationship Actions
  // ==========================================================================

  /**
   * Update consent on a human relationship.
   */
  updateHumanRelationshipConsent(relationshipId: string, consent: boolean): Observable<void> {
    return this.http
      .post<void>(`${this.baseUrl}/db/human-relationships/${relationshipId}/consent`, { consent })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('updateHumanRelationshipConsent', error))
      );
  }

  /**
   * Update custody setting on a human relationship.
   * API accepts camelCase.
   */
  updateHumanRelationshipCustody(
    relationshipId: string,
    enabled: boolean,
    autoCustody = false
  ): Observable<void> {
    return this.http
      .post<void>(`${this.baseUrl}/db/human-relationships/${relationshipId}/custody`, {
        enabled,
        autoCustody,
      })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('updateHumanRelationshipCustody', error))
      );
  }

  // ==========================================================================
  // Contributor Presence Actions
  // ==========================================================================

  /**
   * Initiate stewardship of a contributor presence.
   * API accepts camelCase.
   */
  initiatePresenceStewardship(presenceId: string, stewardId: string): Observable<void> {
    return this.http
      .post<void>(`${this.baseUrl}/db/presences/${presenceId}/stewardship`, { stewardId })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('initiatePresenceStewardship', error))
      );
  }

  /**
   * Initiate a claim on a contributor presence.
   */
  initiatePresenceClaim(presenceId: string, agentId: string): Observable<void> {
    // API accepts camelCase InputView
    return this.http
      .post<void>(`${this.baseUrl}/db/presences/${presenceId}/claim`, { agentId })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('initiatePresenceClaim', error))
      );
  }

  /**
   * Verify a presence claim with evidence.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  verifyPresenceClaim(
    presenceId: string,
    evidence: { method: string; data: Record<string, unknown> }
  ): Observable<void> {
    return this.http
      .post<void>(`${this.baseUrl}/db/presences/${presenceId}/verify-claim`, {
        verificationMethod: evidence.method,
        evidence: evidence.data,
      })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('verifyPresenceClaim', error))
      );
  }

  // ==========================================================================
  // Economic Event Creation
  // ==========================================================================

  /**
   * Create a new economic event.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  createEconomicEvent(input: CreateEventInput): Observable<EconomicEventView> {
    const body = {
      action: input.action,
      provider: input.provider,
      receiver: input.receiver,
      resourceConformsTo: input.resourceConformsTo,
      resourceQuantityValue: input.resourceQuantity?.value,
      resourceQuantityUnit: input.resourceQuantity?.unit,
      lamadEventType: input.lamadEventType,
      contentId: input.contentId,
      pathId: input.pathId,
      contributorPresenceId: input.contributorPresenceId,
      note: input.note,
      metadata: input.metadata ?? null,
    };

    return this.http.post<EconomicEventView>(`${this.baseUrl}/db/events`, body).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('createEconomicEvent', error))
    );
  }

  // ==========================================================================
  // Stewardship Allocations
  // ==========================================================================

  /**
   * List stewardship allocations with optional filters.
   */
  getStewardshipAllocations(query?: AllocationQuery): Observable<StewardshipAllocationView[]> {
    let params = new HttpParams().set('appId', this.appId);

    if (query?.contentId) params = params.set('contentId', query.contentId);
    if (query?.stewardPresenceId) params = params.set('stewardPresenceId', query.stewardPresenceId);
    if (query?.governanceState) params = params.set('governanceState', query.governanceState);
    if (query?.activeOnly) params = params.set('activeOnly', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http
      .get<StewardshipAllocationView[]>(`${this.baseUrl}/db/allocations`, { params })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('getStewardshipAllocations', error))
      );
  }

  /**
   * Get a specific stewardship allocation by ID.
   */
  getStewardshipAllocation(allocationId: string): Observable<StewardshipAllocationView | null> {
    return this.http
      .get<StewardshipAllocationView | null>(`${this.baseUrl}/db/allocations/${allocationId}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => {
          if ((error as Record<string, unknown>)['status'] === 404) {
            return of(null);
          }
          return this.handleError('getStewardshipAllocation', error);
        })
      );
  }

  /**
   * Get all stewardship data for a content piece (aggregate view).
   */
  getContentStewardship(contentId: string): Observable<ContentStewardshipView> {
    return this.http
      .get<ContentStewardshipView>(`${this.baseUrl}/db/allocations/content/${contentId}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('getContentStewardship', error))
      );
  }

  /**
   * Get all allocations for a steward.
   */
  getAllocationsForSteward(stewardPresenceId: string): Observable<StewardshipAllocationView[]> {
    return this.http
      .get<
        StewardshipAllocationView[]
      >(`${this.baseUrl}/db/allocations/steward/${stewardPresenceId}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('getAllocationsForSteward', error))
      );
  }

  /**
   * Create a new stewardship allocation.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  createStewardshipAllocation(input: CreateAllocationInput): Observable<StewardshipAllocationView> {
    const body = {
      contentId: input.contentId,
      stewardPresenceId: input.stewardPresenceId,
      allocationRatio: input.allocationRatio ?? 1,
      allocationMethod: input.allocationMethod ?? 'manual',
      contributionType: input.contributionType ?? 'inherited',
      contributionEvidence: input.contributionEvidence ?? null,
      note: input.note,
      metadata: input.metadata ?? null,
    };

    return this.http.post<unknown>(`${this.baseUrl}/api/v1/stewardship/allocations`, body).pipe(
      timeout(this.defaultTimeoutMs),
      tap(response => {
        const gate = extractGateFromResponse(response);
        if (gate) this.gateService.handleGateResponse(gate);
      }),
      map(
        response =>
          (response as { data: StewardshipAllocationView }).data ??
          (response as StewardshipAllocationView)
      ),
      catchError(error => handleGateError(error, this.gateService)),
      catchError(error => this.handleError('createStewardshipAllocation', error))
    );
  }

  /**
   * Update a stewardship allocation.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  updateStewardshipAllocation(
    allocationId: string,
    input: UpdateAllocationInput
  ): Observable<StewardshipAllocationView> {
    const body: Record<string, unknown> = {};
    if (input.allocationRatio !== undefined) body['allocationRatio'] = input.allocationRatio;
    if (input.allocationMethod) body['allocationMethod'] = input.allocationMethod;
    if (input.contributionType) body['contributionType'] = input.contributionType;
    if (input.contributionEvidence !== undefined)
      body['contributionEvidence'] = input.contributionEvidence;
    if (input.governanceState) body['governanceState'] = input.governanceState;
    if (input.disputeId) body['disputeId'] = input.disputeId;
    if (input.disputeReason) body['disputeReason'] = input.disputeReason;
    if (input.elohimRatifiedAt) body['elohimRatifiedAt'] = input.elohimRatifiedAt;
    if (input.elohimRatifierId) body['elohimRatifierId'] = input.elohimRatifierId;
    if (input.note) body['note'] = input.note;

    return this.http
      .put<unknown>(`${this.baseUrl}/api/v1/stewardship/allocations/${allocationId}`, body)
      .pipe(
        timeout(this.defaultTimeoutMs),
        tap(response => {
          const gate = extractGateFromResponse(response);
          if (gate) this.gateService.handleGateResponse(gate);
        }),
        map(
          response =>
            (response as { data: StewardshipAllocationView }).data ??
            (response as StewardshipAllocationView)
        ),
        catchError(error => handleGateError(error, this.gateService)),
        catchError(error => this.handleError('updateStewardshipAllocation', error))
      );
  }

  /**
   * Delete a stewardship allocation.
   */
  deleteStewardshipAllocation(allocationId: string): Observable<void> {
    return this.http
      .delete<unknown>(`${this.baseUrl}/api/v1/stewardship/allocations/${allocationId}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        tap(response => {
          const gate = extractGateFromResponse(response);
          if (gate) this.gateService.handleGateResponse(gate);
        }),
        map(() => undefined as void),
        catchError(error => handleGateError(error, this.gateService)),
        catchError(error => this.handleError('deleteStewardshipAllocation', error))
      );
  }

  /**
   * File a dispute on a stewardship allocation.
   * API accepts camelCase InputView.
   */
  fileAllocationDispute(
    allocationId: string,
    input: FileDisputeInput
  ): Observable<StewardshipAllocationView> {
    const body = {
      disputeId: input.disputeId,
      disputedBy: input.disputedBy,
      reason: input.reason,
    };

    return this.http
      .post<unknown>(`${this.baseUrl}/api/v1/stewardship/allocations/${allocationId}/dispute`, body)
      .pipe(
        timeout(this.defaultTimeoutMs),
        tap(response => {
          const gate = extractGateFromResponse(response);
          if (gate) this.gateService.handleGateResponse(gate);
        }),
        map(
          response =>
            (response as { data: StewardshipAllocationView }).data ??
            (response as StewardshipAllocationView)
        ),
        catchError(error => handleGateError(error, this.gateService)),
        catchError(error => this.handleError('fileAllocationDispute', error))
      );
  }

  /**
   * Resolve a dispute on a stewardship allocation (Elohim ratification).
   * API accepts camelCase InputView.
   */
  resolveAllocationDispute(
    allocationId: string,
    input: ResolveDisputeInput
  ): Observable<StewardshipAllocationView> {
    const body = {
      ratifierId: input.ratifierId,
      newState: input.newState,
    };

    return this.http
      .post<unknown>(`${this.baseUrl}/api/v1/stewardship/allocations/${allocationId}/resolve`, body)
      .pipe(
        timeout(this.defaultTimeoutMs),
        tap(response => {
          const gate = extractGateFromResponse(response);
          if (gate) this.gateService.handleGateResponse(gate);
        }),
        map(
          response =>
            (response as { data: StewardshipAllocationView }).data ??
            (response as StewardshipAllocationView)
        ),
        catchError(error => handleGateError(error, this.gateService)),
        catchError(error => this.handleError('resolveAllocationDispute', error))
      );
  }

  /**
   * Bulk create stewardship allocations.
   * API accepts camelCase InputView with parsed JSON objects.
   */
  bulkCreateAllocations(inputs: CreateAllocationInput[]): Observable<BulkAllocationResult> {
    const body = inputs.map(input => ({
      contentId: input.contentId,
      stewardPresenceId: input.stewardPresenceId,
      allocationRatio: input.allocationRatio ?? 1,
      allocationMethod: input.allocationMethod ?? 'manual',
      contributionType: input.contributionType ?? 'inherited',
      contributionEvidence: input.contributionEvidence ?? null,
      note: input.note,
      metadata: input.metadata ?? null,
    }));

    return this.http
      .post<unknown>(`${this.baseUrl}/api/v1/stewardship/allocations/bulk`, body)
      .pipe(
        timeout(this.defaultTimeoutMs * 2), // Double timeout for bulk operations
        tap(response => {
          const gate = extractGateFromResponse(response);
          if (gate) this.gateService.handleGateResponse(gate);
        }),
        map(
          response =>
            (response as { data: BulkAllocationResult }).data ?? (response as BulkAllocationResult)
        ),
        catchError(error => handleGateError(error, this.gateService)),
        catchError(error => this.handleError('bulkCreateAllocations', error))
      );
  }

  // ==========================================================================
  // Comments
  // ==========================================================================

  /**
   * Create a new comment on content.
   * Gated mutation — extracts gate evaluation from response envelope.
   */
  createComment(contentId: string, body: string): Observable<unknown> {
    return this.http
      .post<unknown>(`${this.baseUrl}/api/v1/comments`, { contentId, body })
      .pipe(
        timeout(this.defaultTimeoutMs),
        tap(response => {
          const gate = extractGateFromResponse(response);
          if (gate) this.gateService.handleGateResponse(gate);
        }),
        map(
          response =>
            (response as { data: unknown }).data ?? response
        ),
        catchError(error => handleGateError(error, this.gateService)),
        catchError(error => this.handleError('createComment', error))
      );
  }

  /**
   * Get comments for a content item.
   * Read-only — no gate handling needed.
   */
  getComments(contentId: string): Observable<unknown[]> {
    const params = new HttpParams().set('contentId', contentId);

    return this.http
      .get<unknown[]>(`${this.baseUrl}/api/v1/comments`, { params })
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('getComments', error))
      );
  }

  // ==========================================================================
  // Error Handling
  // ==========================================================================

  private handleError(operation: string, error: unknown): Observable<never> {
    const message = error instanceof Error ? error.message : String(error);

    return throwError(() => new Error(`${operation} failed: ${message}`));
  }
}
