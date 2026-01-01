/**
 * Presence Service Tests
 */

import { TestBed } from '@angular/core/testing';
import { PresenceService } from './presence.service';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { IdentityService } from './identity.service';
import { ContributorPresenceView, PresenceState } from '../models/presence.model';

describe('PresenceService', () => {
  let service: PresenceService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;

  const mockPresenceEntry = {
    id: 'presence-123',
    display_name: 'Test Contributor',
    presence_state: 'UNCLAIMED',
    external_identifiers_json: '[]',
    establishing_content_ids_json: '["content-1"]',
    established_at: new Date().toISOString(),
    affinity_total: 10,
    unique_engagers: 5,
    citation_count: 3,
    endorsements_json: '[]',
    recognition_score: 15,
    recognition_by_content_json: '{}',
    accumulating_since: new Date().toISOString(),
    last_recognition_at: new Date().toISOString(),
    steward_id: null,
    stewardship_started_at: null,
    stewardship_commitment_id: null,
    stewardship_quality_score: null,
    claim_initiated_at: null,
    claim_verified_at: null,
    claim_verification_method: null,
    claim_evidence_json: null,
    claimed_agent_id: null,
    claim_recognition_transferred_value: null,
    claim_recognition_transferred_unit: null,
    claim_facilitated_by: null,
    invitations_json: '[]',
    note: 'Test note',
    image: null,
    metadata_json: '{}',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  beforeEach(() => {
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', [
      'isConnected',
      'callZome',
    ]);

    mockIdentityService = jasmine.createSpyObj('IdentityService', [], {
      agentPubKey: jasmine.createSpy().and.returnValue('agent-pub-key-123'),
    });

    TestBed.configureTestingModule({
      providers: [
        PresenceService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: IdentityService, useValue: mockIdentityService },
      ],
    });

    service = TestBed.inject(PresenceService);
  });

  describe('initial state', () => {
    it('should start with empty presences cache', () => {
      expect(service.presences().size).toBe(0);
    });

    it('should start with empty stewarded presences', () => {
      expect(service.myStewardedPresences().length).toBe(0);
    });

    it('should start not loading', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should start without error', () => {
      expect(service.error()).toBeNull();
    });
  });

  describe('createPresence', () => {
    it('should throw when not connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);

      await expectAsync(service.createPresence({
        displayName: 'New Contributor',
      })).toBeRejectedWithError('Not connected to network');
    });

    it('should create presence when connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: {
          action_hash: new Uint8Array([1, 2, 3]),
          presence: mockPresenceEntry,
        },
      });

      const result = await service.createPresence({
        displayName: 'Test Contributor',
        note: 'Test note',
      });

      expect(result.displayName).toBe('Test Contributor');
      expect(result.presenceState).toBe('UNCLAIMED');
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'imagodei',
          fnName: 'create_contributor_presence',
          roleName: 'imagodei',
        })
      );
    });

    it('should cache created presence', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: {
          action_hash: new Uint8Array([1, 2, 3]),
          presence: mockPresenceEntry,
        },
      });

      await service.createPresence({ displayName: 'Test' });

      expect(service.presences().has('presence-123')).toBe(true);
    });

    it('should set loading state during creation', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      let capturedLoading = false;
      mockHolochainClient.callZome.and.callFake(async () => {
        capturedLoading = service.isLoading();
        return {
          success: true,
          data: { action_hash: new Uint8Array([1]), presence: mockPresenceEntry },
        };
      });

      await service.createPresence({ displayName: 'Test' });

      expect(capturedLoading).toBe(true);
      expect(service.isLoading()).toBe(false);
    });

    it('should set error on failure', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({
        success: false,
        error: 'Network error',
      });

      await expectAsync(service.createPresence({ displayName: 'Test' }))
        .toBeRejectedWithError('Network error');

      expect(service.error()).toBe('Network error');
    });
  });

  describe('beginStewardship', () => {
    it('should throw when not connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);

      await expectAsync(service.beginStewardship('presence-123'))
        .toBeRejectedWithError('Not connected to network');
    });

    it('should throw when not authenticated', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      (mockIdentityService.agentPubKey as jasmine.Spy).and.returnValue(null);

      await expectAsync(service.beginStewardship('presence-123'))
        .toBeRejectedWithError('Must be authenticated to begin stewardship');
    });

    it('should begin stewardship when authenticated', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      const stewardedEntry = {
        ...mockPresenceEntry,
        presence_state: 'STEWARDED',
        steward_id: 'agent-pub-key-123',
        stewardship_started_at: new Date().toISOString(),
      };

      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: { action_hash: new Uint8Array([1]), presence: stewardedEntry },
      });

      const result = await service.beginStewardship('presence-123', 'I will care for this');

      expect(result.presenceState).toBe('STEWARDED');
      expect(result.stewardId).toBe('agent-pub-key-123');
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'begin_stewardship',
          payload: jasmine.objectContaining({
            presence_id: 'presence-123',
            steward_agent_id: 'agent-pub-key-123',
            commitment_note: 'I will care for this',
          }),
        })
      );
    });
  });

  describe('getMyStewardedPresences', () => {
    it('should return empty when not connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);

      const result = await service.getMyStewardedPresences();

      expect(result).toEqual([]);
    });

    it('should return empty when not authenticated', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      (mockIdentityService.agentPubKey as jasmine.Spy).and.returnValue(null);

      const result = await service.getMyStewardedPresences();

      expect(result).toEqual([]);
    });

    it('should return stewarded presences', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: [
          { action_hash: new Uint8Array([1]), presence: mockPresenceEntry },
          { action_hash: new Uint8Array([2]), presence: { ...mockPresenceEntry, id: 'presence-456' } },
        ],
      });

      const result = await service.getMyStewardedPresences();

      expect(result.length).toBe(2);
      expect(service.myStewardedPresences().length).toBe(2);
    });
  });

  describe('initiateClaim', () => {
    it('should throw when not connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);

      await expectAsync(service.initiateClaim({
        presenceId: 'presence-123',
        claimEvidence: { proof: 'test' },
        verificationMethod: 'email',
      })).toBeRejectedWithError('Not connected to network');
    });

    it('should initiate claim', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      const claimingEntry = {
        ...mockPresenceEntry,
        claim_initiated_at: new Date().toISOString(),
        claim_verification_method: 'email',
      };

      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: { action_hash: new Uint8Array([1]), presence: claimingEntry },
      });

      const result = await service.initiateClaim({
        presenceId: 'presence-123',
        claimEvidence: { proof: 'test' },
        verificationMethod: 'email',
      });

      expect(result.claimInitiatedAt).not.toBeNull();
      expect(result.claimVerificationMethod).toBe('email');
    });
  });

  describe('verifyClaim', () => {
    it('should verify claim', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      const claimedEntry = {
        ...mockPresenceEntry,
        presence_state: 'CLAIMED',
        claim_verified_at: new Date().toISOString(),
        claimed_agent_id: 'agent-123',
      };

      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: { action_hash: new Uint8Array([1]), presence: claimedEntry },
      });

      const result = await service.verifyClaim('presence-123');

      expect(result.presenceState).toBe('CLAIMED');
      expect(result.claimVerifiedAt).not.toBeNull();
    });
  });

  describe('getPresenceById', () => {
    it('should return cached presence', async () => {
      // First, add to cache by creating
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: { action_hash: new Uint8Array([1]), presence: mockPresenceEntry },
      });

      await service.createPresence({ displayName: 'Test' });

      // Reset spy to verify cache hit
      mockHolochainClient.callZome.calls.reset();

      const result = await service.getPresenceById('presence-123');

      expect(result?.id).toBe('presence-123');
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('should fetch when not cached', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: { action_hash: new Uint8Array([1]), presence: mockPresenceEntry },
      });

      const result = await service.getPresenceById('presence-123');

      expect(result?.id).toBe('presence-123');
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_contributor_presence_by_id',
          payload: 'presence-123',
        })
      );
    });

    it('should return null when not connected and not cached', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);

      const result = await service.getPresenceById('presence-123');

      expect(result).toBeNull();
    });
  });

  describe('getPresencesByState', () => {
    it('should return empty when not connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);

      const result = await service.getPresencesByState('UNCLAIMED');

      expect(result).toEqual([]);
    });

    it('should return presences by state', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: [
          { action_hash: new Uint8Array([1]), presence: mockPresenceEntry },
        ],
      });

      const result = await service.getPresencesByState('UNCLAIMED');

      expect(result.length).toBe(1);
      expect(result[0].presenceState).toBe('UNCLAIMED');
    });
  });

  describe('cache management', () => {
    it('should clear error', () => {
      // Trigger an error first
      mockHolochainClient.isConnected.and.returnValue(false);
      service.createPresence({ displayName: 'Test' }).catch(() => {});

      service.clearError();

      expect(service.error()).toBeNull();
    });

    it('should clear cache', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({
        success: true,
        data: { action_hash: new Uint8Array([1]), presence: mockPresenceEntry },
      });

      await service.createPresence({ displayName: 'Test' });
      expect(service.presences().size).toBe(1);

      service.clearCache();

      expect(service.presences().size).toBe(0);
      expect(service.myStewardedPresences().length).toBe(0);
    });
  });
});
