/**
 * HumanRelationshipService - Domain service for human social graph relationships.
 *
 * Provides high-level operations for managing human-to-human relationships:
 * - Fetching relationships for a party (as either A or B)
 * - Filtering by intimacy level, relationship type
 * - Managing consent and custody settings
 * - Finding custody-enabled contacts for recovery
 *
 * Uses StorageApiService for HTTP communication with elohim-storage.
 */

import { Injectable } from '@angular/core';

// @coverage: 100.0% (2026-02-04)

import { map } from 'rxjs/operators';

import { Observable, forkJoin } from 'rxjs';

import { HumanRelationshipView } from '@app/elohim/adapters/storage-types.adapter';
import { StorageApiService } from '@app/elohim/services/storage-api.service';
import {
  CreateHumanRelationshipInput,
  HumanRelationshipType,
  IntimacyLevel,
  INTIMACY_LEVEL_ORDER,
} from '@app/imagodei/models/human-relationship.model';

@Injectable({
  providedIn: 'root',
})
export class HumanRelationshipService {
  constructor(private readonly storageApi: StorageApiService) {}

  // ===========================================================================
  // Query Methods
  // ===========================================================================

  /**
   * Get all relationships for a person (as either party A or B).
   */
  getRelationshipsForPerson(personId: string): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({ partyId: personId });
  }

  /**
   * Get relationships where the person is specifically party A (initiator side).
   */
  getRelationshipsAsPartyA(personId: string): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({ partyAId: personId });
  }

  /**
   * Get relationships where the person is specifically party B (recipient side).
   */
  getRelationshipsAsPartyB(personId: string): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({ partyBId: personId });
  }

  /**
   * Get relationships of a specific type for a person.
   */
  getRelationshipsByType(
    personId: string,
    relationshipType: HumanRelationshipType
  ): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({
      partyId: personId,
      relationshipType,
    });
  }

  /**
   * Get relationships at or above a minimum intimacy level.
   */
  getRelationshipsByIntimacy(
    personId: string,
    minIntimacyLevel: IntimacyLevel
  ): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({
      partyId: personId,
      minIntimacyLevel,
    });
  }

  /**
   * Get only fully consented relationships.
   */
  getConsentedRelationships(personId: string): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({
      partyId: personId,
      fullyConsentedOnly: true,
    });
  }

  // ===========================================================================
  // Custody-Related Queries
  // ===========================================================================

  /**
   * Get relationships where custody is enabled (for recovery purposes).
   */
  getCustodyRelationships(personId: string): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({
      partyId: personId,
      custodyEnabledOnly: true,
    });
  }

  /**
   * Get relationships with auto-custody enabled (highest trust).
   */
  getAutoCustodyRelationships(personId: string): Observable<HumanRelationshipView[]> {
    return this.getCustodyRelationships(personId).pipe(
      map(relationships => relationships.filter(rel => rel.autoCustodyEnabled))
    );
  }

  /**
   * Get trusted contacts who can help with recovery.
   * Returns relationships that are:
   * - Fully consented
   * - Have custody enabled by the person
   * - At least 'trusted' intimacy level
   */
  getRecoveryContacts(personId: string): Observable<HumanRelationshipView[]> {
    return this.storageApi.getHumanRelationships({
      partyId: personId,
      fullyConsentedOnly: true,
      custodyEnabledOnly: true,
      minIntimacyLevel: 'trusted' as IntimacyLevel,
    });
  }

  // ===========================================================================
  // Mutation Methods
  // ===========================================================================

  /**
   * Create a new human relationship.
   */
  createRelationship(input: CreateHumanRelationshipInput): Observable<HumanRelationshipView> {
    return this.storageApi.createHumanRelationship(input);
  }

  /**
   * Update consent status on a relationship.
   */
  updateConsent(relationshipId: string, consent: boolean): Observable<void> {
    return this.storageApi.updateHumanRelationshipConsent(relationshipId, consent);
  }

  /**
   * Enable or disable custody on a relationship.
   *
   * @param relationshipId The relationship ID
   * @param enabled Whether to enable custody
   * @param autoCustody Whether to enable auto-custody (auto-verify recovery requests)
   */
  updateCustody(relationshipId: string, enabled: boolean, autoCustody = false): Observable<void> {
    return this.storageApi.updateHumanRelationshipCustody(relationshipId, enabled, autoCustody);
  }

  // ===========================================================================
  // Utility Methods
  // ===========================================================================

  /**
   * Check if a relationship exists between two people.
   */
  relationshipExists(personAId: string, personBId: string): Observable<boolean> {
    return this.storageApi
      .getHumanRelationships({
        partyAId: personAId,
        partyBId: personBId,
      })
      .pipe(map(relationships => relationships.length > 0));
  }

  /**
   * Get the relationship between two specific people (if any).
   */
  getRelationshipBetween(
    personAId: string,
    personBId: string
  ): Observable<HumanRelationshipView | null> {
    // Check both directions since the relationship could be stored either way
    return forkJoin([
      this.storageApi.getHumanRelationships({ partyAId: personAId, partyBId: personBId }),
      this.storageApi.getHumanRelationships({ partyAId: personBId, partyBId: personAId }),
    ]).pipe(
      map(([forward, reverse]) => {
        const all = [...forward, ...reverse];
        return all.length > 0 ? all[0] : null;
      })
    );
  }

  /**
   * Compare intimacy levels.
   * Returns true if level1 >= level2.
   */
  isIntimacyAtLeast(level1: IntimacyLevel, level2: IntimacyLevel): boolean {
    return INTIMACY_LEVEL_ORDER[level1] >= INTIMACY_LEVEL_ORDER[level2];
  }

  /**
   * Sort relationships by intimacy level (highest first).
   */
  sortByIntimacy(relationships: HumanRelationshipView[]): HumanRelationshipView[] {
    return [...relationships].sort((a, b) => {
      const levelA = INTIMACY_LEVEL_ORDER[a.intimacyLevel as IntimacyLevel] ?? 0;
      const levelB = INTIMACY_LEVEL_ORDER[b.intimacyLevel as IntimacyLevel] ?? 0;
      return levelB - levelA;
    });
  }
}
