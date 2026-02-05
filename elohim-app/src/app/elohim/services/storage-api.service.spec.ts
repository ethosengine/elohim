import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import {
  HttpClientTestingModule,
  HttpTestingController,
} from '@angular/common/http/testing';

import { StorageApiService } from './storage-api.service';

/**
 * Comprehensive tests for StorageApiService
 *
 * Tests coverage:
 * - Content relationships (GET, POST)
 * - Human relationships (GET, POST)
 * - Contributor presences (GET, POST)
 * - Economic events (GET, POST)
 * - Content mastery (GET, POST)
 * - Stewardship allocations (CRUD)
 * - Error handling and timeouts
 * - URL construction and parameter building
 */
describe('StorageApiService', () => {
  let service: StorageApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [StorageApiService],
    });

    service = TestBed.inject(StorageApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // ===========================================================================
  // Service Creation
  // ===========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should initialize baseUrl from environment', () => {
      expect((service as any).baseUrl).toBeDefined();
    });
  });

  // ===========================================================================
  // Content Relationships
  // ===========================================================================

  describe('getRelationships', () => {
    it('should fetch relationships without query', fakeAsync(() => {
      const mockRelationships = [
        {
          id: 'rel-1',
          sourceId: 'content-1',
          targetId: 'content-2',
          relationshipType: 'prerequisite',
          confidence: 1,
        },
      ];

      service.getRelationships().subscribe(rels => {
        expect(rels.length).toBe(1);
        expect(rels[0].id).toBe('rel-1');
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/db/relationships')
      );
      expect(req.request.params.get('appId')).toBe('lamad');
      req.flush(mockRelationships);
      tick();
    }));

    it('should fetch relationships with query filters', fakeAsync(() => {
      service
        .getRelationships({
          sourceId: 'content-1',
          relationshipType: 'prerequisite',
          minConfidence: 0.8,
          limit: 10,
        })
        .subscribe();

      const req = httpMock.expectOne(request => {
        const params = request.params;
        return (
          params.get('sourceId') === 'content-1' &&
          params.get('relationshipType') === 'prerequisite' &&
          params.get('minConfidence') === '0.8' &&
          params.get('limit') === '10'
        );
      });
      req.flush([]);
      tick();
    }));

    it('should handle relationship fetch errors', fakeAsync(() => {
      let error: Error | null = null;
      service.getRelationships().subscribe({
        error: (err: Error) => {
          error = err;
        },
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/db/relationships')
      );
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('getRelationships failed');
    }));
  });

  describe('getRelationshipsForContent', () => {
    it('should fetch outgoing relationships for content', fakeAsync(() => {
      const mockRelationships = [
        { id: 'rel-1', sourceId: 'content-1', targetId: 'content-2' },
      ];

      service.getRelationshipsForContent('content-1').subscribe(rels => {
        expect(rels.length).toBe(1);
      });

      const req = httpMock.expectOne(request =>
        request.params.get('sourceId') === 'content-1'
      );
      req.flush(mockRelationships);
      tick();
    }));
  });

  describe('createRelationship', () => {
    it('should create relationship with minimal input', fakeAsync(() => {
      const input = {
        sourceId: 'content-1',
        targetId: 'content-2',
        relationshipType: 'prerequisite',
      };

      service.createRelationship(input).subscribe(rel => {
        expect(rel.sourceId).toBe('content-1');
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/db/relationships') &&
          request.method === 'POST'
        );
      });

      expect(req.request.body.confidence).toBe(1);
      expect(req.request.body.inferenceSource).toBe('author');
      expect(req.request.body.createInverse).toBe(false);

      req.flush({ id: 'rel-new', ...input });
      tick();
    }));

    it('should create relationship with full options', fakeAsync(() => {
      const input = {
        sourceId: 'content-1',
        targetId: 'content-2',
        relationshipType: 'prerequisite',
        confidence: 0.9,
        inferenceSource: 'system' as const,
        createInverse: true,
        inverseType: 'prerequisiteOf',
        provenanceChain: ['prov-1'],
        metadataJson: '{"custom": "value"}',
      };

      service.createRelationship(input).subscribe();

      const req = httpMock.expectOne(request =>
        request.method === 'POST'
      );

      expect(req.request.body.confidence).toBe(0.9);
      expect(req.request.body.createInverse).toBe(true);
      expect(req.request.body.metadata).toEqual({ custom: 'value' });

      req.flush({ id: 'rel-new', ...input });
      tick();
    }));
  });

  // ===========================================================================
  // Human Relationships
  // ===========================================================================

  describe('getHumanRelationships', () => {
    it('should fetch human relationships', fakeAsync(() => {
      const mockRelationships = [
        {
          id: 'hrel-1',
          appId: 'imagodei',
          partyAId: 'human-1',
          partyBId: 'human-2',
          relationshipType: 'connection',
          intimacyLevel: 'connection',
          isBidirectional: true,
          consentGivenByA: true,
          consentGivenByB: true,
          custodyEnabledByA: false,
          custodyEnabledByB: false,
          autoCustodyEnabled: false,
          emergencyAccessEnabled: false,
          initiatedBy: 'human-1',
          verifiedAt: null,
          governanceLayer: null,
          reach: 'peer',
          context: null,
          createdAt: '2024-01-01T00:00:00Z',
          updatedAt: '2024-01-01T00:00:00Z',
          expiresAt: null,
        },
      ];

      service.getHumanRelationships({ partyId: 'human-1' }).subscribe(rels => {
        expect(rels.length).toBe(1);
        expect(rels[0].isFullyConsented).toBe(true);
      });

      const req = httpMock.expectOne(request => {
        return (
          request.params.get('appId') === 'imagodei' &&
          request.params.get('partyId') === 'human-1'
        );
      });
      req.flush(mockRelationships);
      tick();
    }));

    it('should apply all query filters', fakeAsync(() => {
      service
        .getHumanRelationships({
          partyAId: 'human-1',
          relationshipType: 'friend',
          minIntimacyLevel: 'connection',
          fullyConsentedOnly: true,
          custodyEnabledOnly: true,
          limit: 20,
        })
        .subscribe();

      const req = httpMock.expectOne(request => {
        const params = request.params;
        return (
          params.get('partyAId') === 'human-1' &&
          params.get('relationshipType') === 'friend' &&
          params.get('fullyConsentedOnly') === 'true' &&
          params.get('custodyEnabledOnly') === 'true'
        );
      });
      req.flush([]);
      tick();
    }));
  });

  describe('createHumanRelationship', () => {
    it('should create human relationship with defaults', fakeAsync(() => {
      const input = {
        partyAId: 'human-1',
        partyBId: 'human-2',
        relationshipType: 'friend' as const,
      };

      service.createHumanRelationship(input).subscribe();

      const req = httpMock.expectOne(request => request.method === 'POST');

      expect(req.request.body.intimacyLevel).toBe('recognition');
      expect(req.request.body.isBidirectional).toBe(false);
      expect(req.request.body.context).toBe(null);

      req.flush({ id: 'hrel-new', ...input, partyAConsented: true, partyBConsented: false });
      tick();
    }));
  });

  // ===========================================================================
  // Contributor Presences
  // ===========================================================================

  describe('getContributorPresences', () => {
    it('should fetch presences without filters', fakeAsync(() => {
      const mockPresences = [
        {
          id: 'presence-1',
          appId: 'lamad',
          displayName: 'Contributor',
          presenceState: 'recognized',
          externalIdentifiers: null,
          establishingContentIds: ['content-1'],
          affinityTotal: 0,
          uniqueEngagers: 0,
          citationCount: 0,
          recognitionScore: 0,
          recognitionByContent: null,
          lastRecognitionAt: null,
          stewardId: null,
          stewardshipStartedAt: null,
          stewardshipCommitmentId: null,
          stewardshipQualityScore: null,
          claimInitiatedAt: null,
          claimVerifiedAt: null,
          claimVerificationMethod: null,
          claimEvidence: null,
          claimedAgentId: null,
          claimRecognitionTransferredValue: null,
          claimFacilitatedBy: null,
          image: null,
          note: null,
          metadata: null,
          createdAt: '2024-01-01T00:00:00Z',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      ];

      service.getContributorPresences().subscribe(presences => {
        expect(presences.length).toBe(1);
        expect(presences[0].establishingContentIds).toEqual(['content-1']);
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/db/presences')
      );
      req.flush(mockPresences);
      tick();
    }));
  });

  describe('getContributorPresence', () => {
    it('should fetch single presence by ID', fakeAsync(() => {
      const mockPresence = {
        id: 'presence-1',
        appId: 'lamad',
        displayName: 'Test',
        presenceState: 'recognized',
        externalIdentifiers: null,
        establishingContentIds: ['content-1'],
        affinityTotal: 0,
        uniqueEngagers: 0,
        citationCount: 0,
        recognitionScore: 0,
        recognitionByContent: null,
        lastRecognitionAt: null,
        stewardId: null,
        stewardshipStartedAt: null,
        stewardshipCommitmentId: null,
        stewardshipQualityScore: null,
        claimInitiatedAt: null,
        claimVerifiedAt: null,
        claimVerificationMethod: null,
        claimEvidence: null,
        claimedAgentId: null,
        claimRecognitionTransferredValue: null,
        claimFacilitatedBy: null,
        image: null,
        note: null,
        metadata: null,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
      };

      service.getContributorPresence('presence-1').subscribe(presence => {
        expect(presence?.id).toBe('presence-1');
        expect(presence?.establishingContentIds).toEqual(['content-1']);
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/db/presences/presence-1')
      );
      req.flush(mockPresence);
      tick();
    }));

    it('should return null on 404', fakeAsync(() => {
      service.getContributorPresence('not-found').subscribe(presence => {
        expect(presence).toBeNull();
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/db/presences/not-found')
      );
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();
    }));
  });

  describe('createContributorPresence', () => {
    it('should create presence with minimal input', fakeAsync(() => {
      const input = {
        displayName: 'New Contributor',
        establishingContentIds: ['content-1'],
      };

      service.createContributorPresence(input).subscribe(presence => {
        expect(presence.displayName).toBe('New Contributor');
      });

      const req = httpMock.expectOne(request =>
        request.method === 'POST' && request.url.includes('/db/presences')
      );

      expect(req.request.body.displayName).toBe('New Contributor');
      expect(req.request.body.externalIdentifiers).toBe(null);

      req.flush({
        id: 'presence-new',
        displayName: 'New Contributor',
        establishingContentIds: '["content-1"]',
      });
      tick();
    }));
  });

  // ===========================================================================
  // Economic Events
  // ===========================================================================

  describe('getEconomicEvents', () => {
    it('should fetch events without filters', fakeAsync(() => {
      const mockEvents = [
        {
          id: 'event-1',
          action: 'work',
          provider: 'agent-1',
          receiver: 'agent-2',
        },
      ];

      service.getEconomicEvents().subscribe(events => {
        expect(events.length).toBe(1);
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/db/events') &&
          request.params.get('appId') === 'shefa'
        );
      });
      req.flush(mockEvents);
      tick();
    }));

    it('should apply event query filters', fakeAsync(() => {
      service
        .getEconomicEvents({
          agentId: 'agent-1',
          agentRole: 'provider',
          actions: ['work', 'use'],
          eventTypes: ['content-create'],
          contentId: 'content-1',
          limit: 25,
        })
        .subscribe();

      const req = httpMock.expectOne(request => {
        const params = request.params;
        return (
          params.get('provider') === 'agent-1' &&
          params.get('action') === 'work,use' &&
          params.get('contentId') === 'content-1'
        );
      });
      req.flush([]);
      tick();
    }));

    it('should handle receiver role correctly', fakeAsync(() => {
      service
        .getEconomicEvents({
          agentId: 'agent-1',
          agentRole: 'receiver',
        })
        .subscribe();

      const req = httpMock.expectOne(request =>
        request.params.get('receiver') === 'agent-1'
      );
      req.flush([]);
      tick();
    }));
  });

  describe('createEconomicEvent', () => {
    it('should create event with minimal input', fakeAsync(() => {
      const input = {
        action: 'work',
        provider: 'agent-1',
        receiver: 'agent-2',
      };

      service.createEconomicEvent(input).subscribe();

      const req = httpMock.expectOne(request =>
        request.method === 'POST' && request.url.includes('/db/events')
      );

      expect(req.request.body.action).toBe('work');
      expect(req.request.body.metadata).toBe(null);

      req.flush({ id: 'event-new', ...input });
      tick();
    }));
  });

  // ===========================================================================
  // Content Mastery
  // ===========================================================================

  describe('getMasteryRecords', () => {
    it('should fetch mastery records', fakeAsync(() => {
      const mockMastery = [
        {
          id: 'mastery-1',
          humanId: 'human-1',
          contentId: 'content-1',
          masteryLevel: 'proficient',
        },
      ];

      service.getMasteryRecords({ humanId: 'human-1' }).subscribe(records => {
        expect(records.length).toBe(1);
      });

      const req = httpMock.expectOne(request =>
        request.params.get('humanId') === 'human-1'
      );
      req.flush(mockMastery);
      tick();
    }));
  });

  describe('getMasteryForHuman', () => {
    it('should fetch mastery for specific human', fakeAsync(() => {
      service.getMasteryForHuman('human-1').subscribe();

      const req = httpMock.expectOne(request =>
        request.url.includes('/db/mastery/human/human-1')
      );
      req.flush([]);
      tick();
    }));
  });

  describe('getMasteryState', () => {
    it('should build mastery map for content IDs', fakeAsync(() => {
      const mockMastery = [
        { id: 'm1', humanId: 'h1', contentId: 'c1', masteryLevel: 'seen' },
        { id: 'm2', humanId: 'h1', contentId: 'c2', masteryLevel: 'proficient' },
        { id: 'm3', humanId: 'h1', contentId: 'c3', masteryLevel: 'mastered' },
      ];

      service.getMasteryState('h1', ['c1', 'c2']).subscribe(map => {
        expect(map.size).toBe(2);
        expect(map.get('c1')?.masteryLevel).toBe('seen');
        expect(map.get('c2')?.masteryLevel).toBe('proficient');
        expect(map.has('c3')).toBe(false);
      });

      const req = httpMock.expectOne(() => true);
      req.flush(mockMastery);
      tick();
    }));
  });

  describe('upsertMastery', () => {
    it('should create mastery with defaults', fakeAsync(() => {
      const input = {
        humanId: 'human-1',
        contentId: 'content-1',
      };

      service.upsertMastery(input).subscribe();

      const req = httpMock.expectOne(request =>
        request.method === 'POST' && request.url.includes('/db/mastery')
      );

      expect(req.request.body.masteryLevel).toBe('seen');
      expect(req.request.body.engagementType).toBe('view');

      req.flush({ id: 'mastery-new', ...input });
      tick();
    }));
  });

  // ===========================================================================
  // Stewardship Allocations
  // ===========================================================================

  describe('getStewardshipAllocations', () => {
    it('should fetch allocations', fakeAsync(() => {
      const mockAllocations = [
        {
          id: 'alloc-1',
          content_id: 'content-1',
          steward_presence_id: 'presence-1',
          allocation_ratio: 1,
          governance_state: 'active',
        },
      ];

      service.getStewardshipAllocations({ contentId: 'content-1' }).subscribe(allocs => {
        expect(allocs.length).toBe(1);
      });

      const req = httpMock.expectOne(request =>
        request.params.get('contentId') === 'content-1'
      );
      req.flush(mockAllocations);
      tick();
    }));
  });

  describe('createStewardshipAllocation', () => {
    it('should create allocation with defaults', fakeAsync(() => {
      const input = {
        contentId: 'content-1',
        stewardPresenceId: 'presence-1',
      };

      service.createStewardshipAllocation(input).subscribe();

      const req = httpMock.expectOne(request =>
        request.method === 'POST' && request.url.includes('/db/allocations')
      );

      expect(req.request.body.allocationRatio).toBe(1);
      expect(req.request.body.allocationMethod).toBe('manual');
      expect(req.request.body.contributionType).toBe('inherited');

      req.flush({ id: 'alloc-new', content_id: 'content-1' });
      tick();
    }));
  });

  describe('updateStewardshipAllocation', () => {
    it('should update allocation fields', fakeAsync(() => {
      const update = {
        allocationRatio: 0.75,
        governanceState: 'active' as const,
        note: 'Updated',
      };

      service.updateStewardshipAllocation('alloc-1', update).subscribe();

      const req = httpMock.expectOne(request =>
        request.method === 'PUT' && request.url.includes('/db/allocations/alloc-1')
      );

      expect(req.request.body.allocationRatio).toBe(0.75);
      expect(req.request.body.governanceState).toBe('active');
      expect(req.request.body.note).toBe('Updated');

      req.flush({ id: 'alloc-1', allocation_ratio: 0.75 });
      tick();
    }));
  });

  describe('deleteStewardshipAllocation', () => {
    it('should delete allocation', fakeAsync(() => {
      service.deleteStewardshipAllocation('alloc-1').subscribe();

      const req = httpMock.expectOne(request =>
        request.method === 'DELETE' && request.url.includes('/db/allocations/alloc-1')
      );

      req.flush(null);
      tick();
    }));
  });

  // ===========================================================================
  // Error Handling
  // ===========================================================================

  describe('error handling', () => {
    it('should handle network errors', fakeAsync(() => {
      let error: Error | null = null;

      service.getRelationships().subscribe({
        error: (err: Error) => {
          error = err;
        },
      });

      const req = httpMock.expectOne(() => true);
      req.error(new ProgressEvent('Network error'));
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('failed');
    }));

    it('should handle HTTP error responses', fakeAsync(() => {
      let errorReceived = false;

      service.getHumanRelationships().subscribe({
        error: () => {
          errorReceived = true;
        },
      });

      const req = httpMock.expectOne(() => true);
      req.flush({ message: 'Server error' }, { status: 500, statusText: 'Internal Server Error' });
      tick();

      expect(errorReceived).toBe(true);
    }));
  });
});
