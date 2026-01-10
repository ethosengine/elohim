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

import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable, throwError, of } from 'rxjs';
import { map, catchError, timeout } from 'rxjs/operators';
import { environment } from '../../../environments/environment';

// Wire types from generated (ts-rs from Diesel models)
import type {
  Relationship,
  HumanRelationship,
  ContributorPresence,
  EconomicEvent,
  ContentMastery,
} from '@elohim/storage-client/generated';

// View types and transformers from adapter
import {
  RelationshipView,
  transformRelationshipFromWire,
  HumanRelationshipView,
  transformHumanRelationshipFromWire,
  ContributorPresenceView,
  transformContributorPresenceFromWire,
  EconomicEventView,
  transformEconomicEventFromWire,
  ContentMasteryView,
  transformContentMasteryFromWire,
} from '@app/elohim/adapters/storage-types.adapter';

// Query and input types from domain models (these remain domain-specific)
import { RelationshipQuery, CreateRelationshipInput } from '@app/lamad/models/content-node.model';
import { HumanRelationshipQuery, CreateHumanRelationshipInput } from '@app/imagodei/models/human-relationship.model';
import { EventQuery } from '@app/elohim/models/economic-event.model';
import {
  StewardshipAllocation,
  ContentStewardship,
  AllocationQuery,
  CreateAllocationInput,
  UpdateAllocationInput,
  FileDisputeInput,
  ResolveDisputeInput,
  BulkAllocationResult,
  fromWireStewardshipAllocation,
  fromWireContentStewardship,
  toWireCreateAllocationInput,
} from '@app/lamad/models/stewardship-allocation.model';

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
 * Input for creating a presence
 */
export interface CreatePresenceInput {
  displayName: string;
  establishingContentIds: string[];
  externalIdentifiers?: Array<{ platform: string; identifier: string }>;
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

/**
 * Input for creating/updating mastery
 */
export interface CreateMasteryInput {
  humanId: string;
  contentId: string;
  masteryLevel?: string;
  engagementType?: string;
}

/**
 * Input for creating an economic event
 */
export interface CreateEventInput {
  action: string;
  provider: string;
  receiver: string;
  resourceConformsTo?: string;
  resourceQuantity?: { value: number; unit: string };
  lamadEventType?: string;
  contentId?: string;
  pathId?: string;
  contributorPresenceId?: string;
  note?: string;
  metadata?: Record<string, unknown>;
}

@Injectable({
  providedIn: 'root',
})
export class StorageApiService {
  /** Base URL for elohim-storage API */
  private baseUrl: string;

  /** App ID for multi-tenant scoping */
  private appId = 'lamad';

  /** Default request timeout in milliseconds */
  private defaultTimeoutMs = 30000;

  constructor(private http: HttpClient) {
    // Use storageUrl from environment or fall back to doorway URL
    this.baseUrl = environment.holochain?.storageUrl || environment.client?.doorwayUrl || '';
  }

  // ==========================================================================
  // Content Relationships
  // ==========================================================================

  /**
   * List content relationships with optional filters.
   */
  getRelationships(query?: RelationshipQuery): Observable<RelationshipView[]> {
    let params = new HttpParams().set('app_id', this.appId);

    if (query?.sourceId) params = params.set('source_id', query.sourceId);
    if (query?.targetId) params = params.set('target_id', query.targetId);
    if (query?.relationshipType) params = params.set('relationship_type', query.relationshipType);
    if (query?.minConfidence) params = params.set('min_confidence', query.minConfidence.toString());
    if (query?.inferenceSource) params = params.set('inference_source', query.inferenceSource);
    if (query?.bidirectionalOnly) params = params.set('bidirectional_only', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<Relationship[]>(`${this.baseUrl}/db/relationships`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => transformRelationshipFromWire(w))),
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
   */
  createRelationship(input: CreateRelationshipInput): Observable<RelationshipView> {
    const body = {
      source_id: input.sourceId,
      target_id: input.targetId,
      relationship_type: input.relationshipType,
      confidence: input.confidence ?? 1.0,
      inference_source: input.inferenceSource ?? 'author',
      create_inverse: input.createInverse ?? false,
      inverse_type: input.inverseType,
      provenance_chain_json: input.provenanceChain ? JSON.stringify(input.provenanceChain) : null,
      metadata_json: input.metadataJson,
    };

    return this.http.post<Relationship>(`${this.baseUrl}/db/relationships`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => transformRelationshipFromWire(w)),
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
    let params = new HttpParams().set('app_id', 'imagodei');

    if (query?.partyId) params = params.set('party_id', query.partyId);
    if (query?.partyAId) params = params.set('party_a_id', query.partyAId);
    if (query?.partyBId) params = params.set('party_b_id', query.partyBId);
    if (query?.relationshipType) params = params.set('relationship_type', query.relationshipType);
    if (query?.minIntimacyLevel) params = params.set('min_intimacy_level', query.minIntimacyLevel);
    if (query?.fullyConsentedOnly) params = params.set('fully_consented_only', 'true');
    if (query?.custodyEnabledOnly) params = params.set('custody_enabled_only', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<HumanRelationship[]>(`${this.baseUrl}/db/human-relationships`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => transformHumanRelationshipFromWire(w))),
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
   */
  createHumanRelationship(input: CreateHumanRelationshipInput): Observable<HumanRelationshipView> {
    const body = {
      party_a_id: input.partyAId,
      party_b_id: input.partyBId,
      relationship_type: input.relationshipType,
      intimacy_level: input.intimacyLevel ?? 'recognition',
      is_bidirectional: input.isBidirectional ?? false,
      context_json: input.context ? JSON.stringify(input.context) : null,
    };

    return this.http.post<HumanRelationship>(`${this.baseUrl}/db/human-relationships`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => transformHumanRelationshipFromWire(w)),
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
    let params = new HttpParams().set('app_id', this.appId);

    if (query?.presenceState) params = params.set('presence_state', query.presenceState);
    if (query?.stewardId) params = params.set('steward_id', query.stewardId);
    if (query?.claimedAgentId) params = params.set('claimed_agent_id', query.claimedAgentId);
    if (query?.minRecognitionScore) params = params.set('min_recognition_score', query.minRecognitionScore.toString());
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<ContributorPresence[]>(`${this.baseUrl}/db/presences`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => transformContributorPresenceFromWire(w))),
      catchError(error => this.handleError('getContributorPresences', error))
    );
  }

  /**
   * Get a specific contributor presence.
   */
  getContributorPresence(presenceId: string): Observable<ContributorPresenceView | null> {
    return this.http.get<ContributorPresence | null>(`${this.baseUrl}/db/presences/${presenceId}`).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => w ? transformContributorPresenceFromWire(w) : null),
      catchError(error => {
        if ((error as any).status === 404) {
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
      map(presences => presences.filter(p =>
        p.establishingContentIds?.includes(contentId)
      ))
    );
  }

  /**
   * Create a new contributor presence.
   */
  createContributorPresence(input: CreatePresenceInput): Observable<ContributorPresenceView> {
    const body = {
      display_name: input.displayName,
      establishing_content_ids_json: JSON.stringify(input.establishingContentIds),
      external_identifiers_json: input.externalIdentifiers ? JSON.stringify(input.externalIdentifiers) : null,
    };

    return this.http.post<ContributorPresence>(`${this.baseUrl}/db/presences`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => transformContributorPresenceFromWire(w)),
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
    let params = new HttpParams().set('app_id', 'shefa');

    if (query?.agentId) {
      params = params.set(query.agentRole === 'provider' ? 'provider' : 'receiver', query.agentId);
    }
    if (query?.actions?.length) params = params.set('action', query.actions.join(','));
    if (query?.eventTypes?.length) params = params.set('lamad_event_type', query.eventTypes.join(','));
    if (query?.contentId) params = params.set('content_id', query.contentId);
    if (query?.pathId) params = params.set('path_id', query.pathId);
    if (query?.from) params = params.set('from', query.from);
    if (query?.to) params = params.set('to', query.to);
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<EconomicEvent[]>(`${this.baseUrl}/db/events`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => transformEconomicEventFromWire(w))),
      catchError(error => this.handleError('getEconomicEvents', error))
    );
  }

  /**
   * Get events for a specific agent.
   */
  getEventsForAgent(agentId: string, role?: 'provider' | 'receiver' | 'either'): Observable<EconomicEventView[]> {
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
    let params = new HttpParams().set('app_id', this.appId);

    if (query?.humanId) params = params.set('human_id', query.humanId);
    if (query?.contentId) params = params.set('content_id', query.contentId);
    if (query?.minLevel) params = params.set('min_level', query.minLevel);
    if (query?.needsRefresh) params = params.set('needs_refresh', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<ContentMastery[]>(`${this.baseUrl}/db/mastery`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => transformContentMasteryFromWire(w))),
      catchError(error => this.handleError('getMasteryRecords', error))
    );
  }

  /**
   * Get mastery for a specific human.
   */
  getMasteryForHuman(humanId: string): Observable<ContentMasteryView[]> {
    return this.http.get<ContentMastery[]>(`${this.baseUrl}/db/mastery/human/${humanId}`).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => transformContentMasteryFromWire(w))),
      catchError(error => this.handleError('getMasteryForHuman', error))
    );
  }

  /**
   * Get mastery state for specific content IDs.
   */
  getMasteryState(humanId: string, contentIds: string[]): Observable<Map<string, ContentMasteryView>> {
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
   */
  upsertMastery(input: CreateMasteryInput): Observable<ContentMasteryView> {
    const body = {
      human_id: input.humanId,
      content_id: input.contentId,
      mastery_level: input.masteryLevel ?? 'seen',
      engagement_type: input.engagementType ?? 'view',
    };

    return this.http.post<ContentMastery>(`${this.baseUrl}/db/mastery`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => transformContentMasteryFromWire(w)),
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
    return this.http.post<void>(
      `${this.baseUrl}/db/human-relationships/${relationshipId}/consent`,
      { consent }
    ).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('updateHumanRelationshipConsent', error))
    );
  }

  /**
   * Update custody setting on a human relationship.
   */
  updateHumanRelationshipCustody(
    relationshipId: string,
    enabled: boolean,
    autoCustody: boolean = false
  ): Observable<void> {
    return this.http.post<void>(
      `${this.baseUrl}/db/human-relationships/${relationshipId}/custody`,
      { enabled, auto_custody: autoCustody }
    ).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('updateHumanRelationshipCustody', error))
    );
  }

  // ==========================================================================
  // Contributor Presence Actions
  // ==========================================================================

  /**
   * Initiate stewardship of a contributor presence.
   */
  initiatePresenceStewardship(presenceId: string, stewardId: string): Observable<void> {
    return this.http.post<void>(
      `${this.baseUrl}/db/presences/${presenceId}/stewardship`,
      { steward_id: stewardId }
    ).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('initiatePresenceStewardship', error))
    );
  }

  /**
   * Initiate a claim on a contributor presence.
   */
  initiatePresenceClaim(presenceId: string, agentId: string): Observable<void> {
    return this.http.post<void>(
      `${this.baseUrl}/db/presences/${presenceId}/claim`,
      { agent_id: agentId }
    ).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('initiatePresenceClaim', error))
    );
  }

  /**
   * Verify a presence claim with evidence.
   */
  verifyPresenceClaim(
    presenceId: string,
    evidence: { method: string; data: Record<string, unknown> }
  ): Observable<void> {
    return this.http.post<void>(
      `${this.baseUrl}/db/presences/${presenceId}/verify-claim`,
      {
        verification_method: evidence.method,
        evidence_json: JSON.stringify(evidence.data),
      }
    ).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('verifyPresenceClaim', error))
    );
  }

  // ==========================================================================
  // Economic Event Creation
  // ==========================================================================

  /**
   * Create a new economic event.
   */
  createEconomicEvent(input: CreateEventInput): Observable<EconomicEventView> {
    const body = {
      action: input.action,
      provider: input.provider,
      receiver: input.receiver,
      resource_conforms_to: input.resourceConformsTo,
      resource_quantity_value: input.resourceQuantity?.value,
      resource_quantity_unit: input.resourceQuantity?.unit,
      lamad_event_type: input.lamadEventType,
      content_id: input.contentId,
      path_id: input.pathId,
      contributor_presence_id: input.contributorPresenceId,
      note: input.note,
      metadata_json: input.metadata ? JSON.stringify(input.metadata) : null,
    };

    return this.http.post<EconomicEvent>(`${this.baseUrl}/db/events`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => transformEconomicEventFromWire(w)),
      catchError(error => this.handleError('createEconomicEvent', error))
    );
  }

  // ==========================================================================
  // Stewardship Allocations
  // ==========================================================================

  /**
   * List stewardship allocations with optional filters.
   */
  getStewardshipAllocations(query?: AllocationQuery): Observable<StewardshipAllocation[]> {
    let params = new HttpParams().set('app_id', this.appId);

    if (query?.contentId) params = params.set('content_id', query.contentId);
    if (query?.stewardPresenceId) params = params.set('steward_presence_id', query.stewardPresenceId);
    if (query?.governanceState) params = params.set('governance_state', query.governanceState);
    if (query?.activeOnly) params = params.set('active_only', 'true');
    if (query?.limit) params = params.set('limit', query.limit.toString());
    if (query?.offset) params = params.set('offset', query.offset.toString());

    return this.http.get<Record<string, unknown>[]>(`${this.baseUrl}/db/allocations`, { params }).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => fromWireStewardshipAllocation(w))),
      catchError(error => this.handleError('getStewardshipAllocations', error))
    );
  }

  /**
   * Get a specific stewardship allocation by ID.
   */
  getStewardshipAllocation(allocationId: string): Observable<StewardshipAllocation | null> {
    return this.http.get<Record<string, unknown> | null>(`${this.baseUrl}/db/allocations/${allocationId}`).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => w ? fromWireStewardshipAllocation(w) : null),
      catchError(error => {
        if ((error as any).status === 404) {
          return of(null);
        }
        return this.handleError('getStewardshipAllocation', error);
      })
    );
  }

  /**
   * Get all stewardship data for a content piece (aggregate view).
   */
  getContentStewardship(contentId: string): Observable<ContentStewardship> {
    return this.http.get<Record<string, unknown>>(`${this.baseUrl}/db/allocations/content/${contentId}`).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => fromWireContentStewardship(w)),
      catchError(error => this.handleError('getContentStewardship', error))
    );
  }

  /**
   * Get all allocations for a steward.
   */
  getAllocationsForSteward(stewardPresenceId: string): Observable<StewardshipAllocation[]> {
    return this.http.get<Record<string, unknown>[]>(`${this.baseUrl}/db/allocations/steward/${stewardPresenceId}`).pipe(
      timeout(this.defaultTimeoutMs),
      map(wires => wires.map(w => fromWireStewardshipAllocation(w))),
      catchError(error => this.handleError('getAllocationsForSteward', error))
    );
  }

  /**
   * Create a new stewardship allocation.
   */
  createStewardshipAllocation(input: CreateAllocationInput): Observable<StewardshipAllocation> {
    const body = toWireCreateAllocationInput(input);

    return this.http.post<Record<string, unknown>>(`${this.baseUrl}/db/allocations`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => fromWireStewardshipAllocation(w)),
      catchError(error => this.handleError('createStewardshipAllocation', error))
    );
  }

  /**
   * Update a stewardship allocation.
   */
  updateStewardshipAllocation(allocationId: string, input: UpdateAllocationInput): Observable<StewardshipAllocation> {
    const body: Record<string, unknown> = {};
    if (input.allocationRatio !== undefined) body['allocation_ratio'] = input.allocationRatio;
    if (input.allocationMethod) body['allocation_method'] = input.allocationMethod;
    if (input.contributionType) body['contribution_type'] = input.contributionType;
    if (input.contributionEvidenceJson) body['contribution_evidence_json'] = input.contributionEvidenceJson;
    if (input.governanceState) body['governance_state'] = input.governanceState;
    if (input.disputeId) body['dispute_id'] = input.disputeId;
    if (input.disputeReason) body['dispute_reason'] = input.disputeReason;
    if (input.elohimRatifiedAt) body['elohim_ratified_at'] = input.elohimRatifiedAt;
    if (input.elohimRatifierId) body['elohim_ratifier_id'] = input.elohimRatifierId;
    if (input.note) body['note'] = input.note;

    return this.http.put<Record<string, unknown>>(`${this.baseUrl}/db/allocations/${allocationId}`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => fromWireStewardshipAllocation(w)),
      catchError(error => this.handleError('updateStewardshipAllocation', error))
    );
  }

  /**
   * Delete a stewardship allocation.
   */
  deleteStewardshipAllocation(allocationId: string): Observable<void> {
    return this.http.delete<void>(`${this.baseUrl}/db/allocations/${allocationId}`).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('deleteStewardshipAllocation', error))
    );
  }

  /**
   * File a dispute on a stewardship allocation.
   */
  fileAllocationDispute(allocationId: string, input: FileDisputeInput): Observable<StewardshipAllocation> {
    const body = {
      dispute_id: input.disputeId,
      disputed_by: input.disputedBy,
      reason: input.reason,
    };

    return this.http.post<Record<string, unknown>>(`${this.baseUrl}/db/allocations/${allocationId}/dispute`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => fromWireStewardshipAllocation(w)),
      catchError(error => this.handleError('fileAllocationDispute', error))
    );
  }

  /**
   * Resolve a dispute on a stewardship allocation (Elohim ratification).
   */
  resolveAllocationDispute(allocationId: string, input: ResolveDisputeInput): Observable<StewardshipAllocation> {
    const body = {
      ratifier_id: input.ratifierId,
      new_state: input.newState,
    };

    return this.http.post<Record<string, unknown>>(`${this.baseUrl}/db/allocations/${allocationId}/resolve`, body).pipe(
      timeout(this.defaultTimeoutMs),
      map(w => fromWireStewardshipAllocation(w)),
      catchError(error => this.handleError('resolveAllocationDispute', error))
    );
  }

  /**
   * Bulk create stewardship allocations.
   */
  bulkCreateAllocations(inputs: CreateAllocationInput[]): Observable<BulkAllocationResult> {
    const body = inputs.map(toWireCreateAllocationInput);

    return this.http.post<BulkAllocationResult>(`${this.baseUrl}/db/allocations/bulk`, body).pipe(
      timeout(this.defaultTimeoutMs * 2),  // Double timeout for bulk operations
      catchError(error => this.handleError('bulkCreateAllocations', error))
    );
  }

  // ==========================================================================
  // Error Handling
  // ==========================================================================

  private handleError(operation: string, error: any): Observable<never> {
    console.error(`[StorageApiService] ${operation} failed:`, error);
    return throwError(() => new Error(`${operation} failed: ${error.message || error}`));
  }
}
