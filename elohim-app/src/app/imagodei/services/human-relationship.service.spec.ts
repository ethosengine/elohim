/**
 * HumanRelationshipService Tests
 *
 * Tests domain service for managing human-to-human relationships.
 */

import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';

import { HumanRelationshipService } from './human-relationship.service';
import { StorageApiService } from '@app/elohim/services/storage-api.service';
import type { HumanRelationshipView } from '@app/elohim/adapters/storage-types.adapter';
import type { CreateHumanRelationshipInput, IntimacyLevel } from '@app/imagodei/models/human-relationship.model';

describe('HumanRelationshipService', () => {
  let service: HumanRelationshipService;
  let mockStorageApi: jasmine.SpyObj<StorageApiService>;

  // Mock relationship data
  const mockRelationship: HumanRelationshipView = {
    id: 'rel-123',
    appId: 'elohim',
    partyAId: 'human-1',
    partyBId: 'human-2',
    relationshipType: 'friend',
    intimacyLevel: 'trusted',
    isBidirectional: true,
    consentGivenByA: true,
    consentGivenByB: true,
    custodyEnabledByA: true,
    custodyEnabledByB: false,
    autoCustodyEnabled: false,
    emergencyAccessEnabled: false,
    initiatedBy: 'human-1',
    verifiedAt: null,
    governanceLayer: null,
    reach: 'private',
    context: null,
    expiresAt: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    isFullyConsented: true,
  };

  const mockRelationship2: HumanRelationshipView = {
    id: 'rel-456',
    appId: 'elohim',
    partyAId: 'human-2',
    partyBId: 'human-1',
    relationshipType: 'family',
    intimacyLevel: 'intimate',
    isBidirectional: true,
    consentGivenByA: true,
    consentGivenByB: true,
    custodyEnabledByA: true,
    custodyEnabledByB: true,
    autoCustodyEnabled: true,
    emergencyAccessEnabled: true,
    initiatedBy: 'human-2',
    verifiedAt: null,
    governanceLayer: null,
    reach: 'private',
    context: null,
    expiresAt: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    isFullyConsented: true,
  };

  beforeEach(() => {
    // Create mock storage API
    mockStorageApi = jasmine.createSpyObj('StorageApiService', [
      'getHumanRelationships',
      'createHumanRelationship',
      'updateHumanRelationshipConsent',
      'updateHumanRelationshipCustody',
    ]);

    TestBed.configureTestingModule({
      providers: [
        HumanRelationshipService,
        { provide: StorageApiService, useValue: mockStorageApi },
      ],
    });

    service = TestBed.inject(HumanRelationshipService);
  });

  // ==========================================================================
  // Query Methods
  // ==========================================================================

  describe('getRelationshipsForPerson', () => {
    it('should fetch all relationships for a person', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship, mockRelationship2]));

      service.getRelationshipsForPerson('human-1').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship, mockRelationship2]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({ partyId: 'human-1' });
          done();
        },
        error: done.fail,
      });
    });

    it('should return empty array when no relationships exist', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([]));

      service.getRelationshipsForPerson('human-3').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([]);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getRelationshipsAsPartyA', () => {
    it('should fetch relationships where person is party A', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship]));

      service.getRelationshipsAsPartyA('human-1').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({ partyAId: 'human-1' });
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getRelationshipsAsPartyB', () => {
    it('should fetch relationships where person is party B', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship]));

      service.getRelationshipsAsPartyB('human-2').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({ partyBId: 'human-2' });
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getRelationshipsByType', () => {
    it('should filter relationships by type', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship]));

      service.getRelationshipsByType('human-1', 'friend').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({
            partyId: 'human-1',
            relationshipType: 'friend',
          });
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getRelationshipsByIntimacy', () => {
    it('should filter relationships by minimum intimacy level', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship2]));

      service.getRelationshipsByIntimacy('human-1', 'trusted').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship2]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({
            partyId: 'human-1',
            minIntimacyLevel: 'trusted',
          });
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getConsentedRelationships', () => {
    it('should filter relationships requiring full consent', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship, mockRelationship2]));

      service.getConsentedRelationships('human-1').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship, mockRelationship2]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({
            partyId: 'human-1',
            fullyConsentedOnly: true,
          });
          done();
        },
        error: done.fail,
      });
    });
  });

  // ==========================================================================
  // Custody-Related Queries
  // ==========================================================================

  describe('getCustodyRelationships', () => {
    it('should fetch relationships with custody enabled', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship]));

      service.getCustodyRelationships('human-1').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({
            partyId: 'human-1',
            custodyEnabledOnly: true,
          });
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getAutoCustodyRelationships', () => {
    it('should filter relationships with auto-custody enabled', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship, mockRelationship2]));

      service.getAutoCustodyRelationships('human-1').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship2]);
          expect(relationships[0].autoCustodyEnabled).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should return empty array when no auto-custody relationships exist', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship]));

      service.getAutoCustodyRelationships('human-1').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([]);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getRecoveryContacts', () => {
    it('should fetch recovery-enabled contacts with full criteria', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship2]));

      service.getRecoveryContacts('human-1').subscribe({
        next: (relationships) => {
          expect(relationships).toEqual([mockRelationship2]);
          expect(mockStorageApi.getHumanRelationships).toHaveBeenCalledWith({
            partyId: 'human-1',
            fullyConsentedOnly: true,
            custodyEnabledOnly: true,
            minIntimacyLevel: 'trusted',
          });
          done();
        },
        error: done.fail,
      });
    });
  });

  // ==========================================================================
  // Mutation Methods
  // ==========================================================================

  describe('createRelationship', () => {
    it('should create a new relationship', (done) => {
      const input: CreateHumanRelationshipInput = {
        partyAId: 'human-1',
        partyBId: 'human-2',
        relationshipType: 'friend',
        intimacyLevel: 'acquaintance',
      };

      mockStorageApi.createHumanRelationship.and.returnValue(of(mockRelationship));

      service.createRelationship(input).subscribe({
        next: (relationship) => {
          expect(relationship).toEqual(mockRelationship);
          expect(mockStorageApi.createHumanRelationship).toHaveBeenCalledWith(input);
          done();
        },
        error: done.fail,
      });
    });

    it('should handle creation errors', (done) => {
      const input: CreateHumanRelationshipInput = {
        partyAId: 'human-1',
        partyBId: 'human-2',
        relationshipType: 'friend',
        intimacyLevel: 'acquaintance',
      };

      mockStorageApi.createHumanRelationship.and.returnValue(
        throwError(() => new Error('Network error'))
      );

      service.createRelationship(input).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('Network error');
          done();
        },
      });
    });
  });

  describe('updateConsent', () => {
    it('should update consent status', (done) => {
      mockStorageApi.updateHumanRelationshipConsent.and.returnValue(of(void 0));

      service.updateConsent('rel-123', true).subscribe({
        next: () => {
          expect(mockStorageApi.updateHumanRelationshipConsent).toHaveBeenCalledWith('rel-123', true);
          done();
        },
        error: done.fail,
      });
    });

    it('should handle consent update errors', (done) => {
      mockStorageApi.updateHumanRelationshipConsent.and.returnValue(
        throwError(() => new Error('Update failed'))
      );

      service.updateConsent('rel-123', true).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('Update failed');
          done();
        },
      });
    });
  });

  describe('updateCustody', () => {
    it('should update custody settings', (done) => {
      mockStorageApi.updateHumanRelationshipCustody.and.returnValue(of(void 0));

      service.updateCustody('rel-123', true, false).subscribe({
        next: () => {
          expect(mockStorageApi.updateHumanRelationshipCustody).toHaveBeenCalledWith(
            'rel-123',
            true,
            false
          );
          done();
        },
        error: done.fail,
      });
    });

    it('should enable auto-custody when requested', (done) => {
      mockStorageApi.updateHumanRelationshipCustody.and.returnValue(of(void 0));

      service.updateCustody('rel-123', true, true).subscribe({
        next: () => {
          expect(mockStorageApi.updateHumanRelationshipCustody).toHaveBeenCalledWith(
            'rel-123',
            true,
            true
          );
          done();
        },
        error: done.fail,
      });
    });
  });

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  describe('relationshipExists', () => {
    it('should return true when relationship exists', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([mockRelationship]));

      service.relationshipExists('human-1', 'human-2').subscribe({
        next: (exists) => {
          expect(exists).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should return false when no relationship exists', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValue(of([]));

      service.relationshipExists('human-1', 'human-3').subscribe({
        next: (exists) => {
          expect(exists).toBe(false);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getRelationshipBetween', () => {
    it('should find relationship in forward direction', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValues(of([mockRelationship]), of([]));

      service.getRelationshipBetween('human-1', 'human-2').subscribe({
        next: (relationship) => {
          expect(relationship).toEqual(mockRelationship);
          done();
        },
        error: done.fail,
      });
    });

    it('should find relationship in reverse direction', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValues(of([]), of([mockRelationship2]));

      service.getRelationshipBetween('human-1', 'human-2').subscribe({
        next: (relationship) => {
          expect(relationship).toEqual(mockRelationship2);
          done();
        },
        error: done.fail,
      });
    });

    it('should return null when no relationship exists in either direction', (done) => {
      mockStorageApi.getHumanRelationships.and.returnValues(of([]), of([]));

      service.getRelationshipBetween('human-1', 'human-3').subscribe({
        next: (relationship) => {
          expect(relationship).toBeNull();
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('isIntimacyAtLeast', () => {
    it('should return true when level1 >= level2', () => {
      expect(service.isIntimacyAtLeast('intimate', 'trusted')).toBe(true);
      expect(service.isIntimacyAtLeast('trusted', 'trusted')).toBe(true);
      expect(service.isIntimacyAtLeast('trusted', 'acquaintance')).toBe(true);
    });

    it('should return false when level1 < level2', () => {
      expect(service.isIntimacyAtLeast('acquaintance', 'trusted')).toBe(false);
      expect(service.isIntimacyAtLeast('recognition', 'acquaintance')).toBe(false);
    });
  });

  describe('sortByIntimacy', () => {
    it('should sort relationships by intimacy level (highest first)', () => {
      const relationships: HumanRelationshipView[] = [
        { ...mockRelationship, intimacyLevel: 'acquaintance' },
        { ...mockRelationship2, intimacyLevel: 'intimate' },
        { ...mockRelationship, id: 'rel-789', intimacyLevel: 'trusted' },
      ];

      const sorted = service.sortByIntimacy(relationships);

      expect(sorted[0].intimacyLevel).toBe('intimate');
      expect(sorted[1].intimacyLevel).toBe('trusted');
      expect(sorted[2].intimacyLevel).toBe('acquaintance');
    });

    it('should not modify the original array', () => {
      const relationships: HumanRelationshipView[] = [
        { ...mockRelationship, intimacyLevel: 'acquaintance' },
        { ...mockRelationship2, intimacyLevel: 'intimate' },
      ];

      const sorted = service.sortByIntimacy(relationships);

      expect(sorted).not.toBe(relationships);
      expect(relationships[0].intimacyLevel).toBe('acquaintance');
    });

    it('should handle empty array', () => {
      const sorted = service.sortByIntimacy([]);
      expect(sorted).toEqual([]);
    });
  });
});
