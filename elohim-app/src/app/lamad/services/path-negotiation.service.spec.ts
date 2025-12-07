import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { PathNegotiationService } from './path-negotiation.service';
import { LocalSourceChainService } from './local-source-chain.service';
import { HumanConsentService } from './human-consent.service';
import { AffinityTrackingService } from '@app/shared/services/affinity-tracking.service';
import { HumanConsent, IntimacyLevel, ConsentState } from '../models/human-consent.model';
import { NegotiationStatus, BridgingStrategy } from '../models/path-negotiation.model';

describe('PathNegotiationService', () => {
  let service: PathNegotiationService;
  let sourceChainSpy: jasmine.SpyObj<LocalSourceChainService>;
  let consentServiceSpy: jasmine.SpyObj<HumanConsentService>;
  let affinityServiceSpy: jasmine.SpyObj<AffinityTrackingService>;

  const AGENT_ID = 'test-agent-123';
  const OTHER_AGENT_ID = 'other-agent-456';

  const mockIntimateConsent: HumanConsent = {
    id: 'consent-123',
    initiatorId: AGENT_ID,
    participantId: OTHER_AGENT_ID,
    intimacyLevel: 'intimate',
    consentState: 'accepted',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    consentedAt: new Date().toISOString(),
    validatingAttestationIds: ['attest-1', 'attest-2'],
    stateHistory: [],
  };

  beforeEach(() => {
    sourceChainSpy = jasmine.createSpyObj('LocalSourceChainService', [
      'isInitialized',
      'getAgentId',
      'getEntriesByType',
      'createEntry',
    ]);
    consentServiceSpy = jasmine.createSpyObj('HumanConsentService', [
      'getConsentWith',
    ]);
    affinityServiceSpy = jasmine.createSpyObj('AffinityTrackingService', [
      'getAffinities',
    ]);

    sourceChainSpy.isInitialized.and.returnValue(true);
    sourceChainSpy.getAgentId.and.returnValue(AGENT_ID);
    sourceChainSpy.getEntriesByType.and.returnValue([]);
    sourceChainSpy.createEntry.and.callFake((type, content) => ({
      entryHash: 'test-hash',
      authorAgent: AGENT_ID,
      entryType: type,
      content,
      timestamp: new Date().toISOString(),
    }));

    TestBed.configureTestingModule({
      providers: [
        PathNegotiationService,
        { provide: LocalSourceChainService, useValue: sourceChainSpy },
        { provide: HumanConsentService, useValue: consentServiceSpy },
        { provide: AffinityTrackingService, useValue: affinityServiceSpy },
      ],
    });

    service = TestBed.inject(PathNegotiationService);
    service.initialize();
  });

  describe('proposeNegotiation', () => {
    it('should create a negotiation with intimate consent', (done) => {
      consentServiceSpy.getConsentWith.and.returnValue(of(mockIntimateConsent));

      service.proposeNegotiation({
        participantId: OTHER_AGENT_ID,
        consentId: 'consent-123',
        message: 'Let\'s create a love map!',
      }).subscribe({
        next: (negotiation) => {
          expect(negotiation.initiatorId).toBe(AGENT_ID);
          expect(negotiation.participantId).toBe(OTHER_AGENT_ID);
          expect(negotiation.status).toBe('proposed');
          expect(negotiation.consentId).toBe('consent-123');
          expect(negotiation.validatingAttestationIds).toEqual(['attest-1', 'attest-2']);
          done();
        },
        error: done.fail,
      });
    });

    it('should error when negotiating with self', (done) => {
      service.proposeNegotiation({
        participantId: AGENT_ID,
        consentId: 'consent-123',
      }).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('yourself');
          done();
        },
      });
    });

    it('should error when no consent exists', (done) => {
      consentServiceSpy.getConsentWith.and.returnValue(of(null));

      service.proposeNegotiation({
        participantId: OTHER_AGENT_ID,
        consentId: 'consent-123',
      }).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('No consent');
          done();
        },
      });
    });

    it('should error when consent is not at intimate level', (done) => {
      const connectionConsent = {
        ...mockIntimateConsent,
        intimacyLevel: 'connection' as IntimacyLevel,
      };
      consentServiceSpy.getConsentWith.and.returnValue(of(connectionConsent));

      service.proposeNegotiation({
        participantId: OTHER_AGENT_ID,
        consentId: 'consent-123',
      }).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('Intimate-level');
          done();
        },
      });
    });

    it('should error when consent is not accepted', (done) => {
      const pendingConsent = {
        ...mockIntimateConsent,
        consentState: 'pending' as ConsentState,
      };
      consentServiceSpy.getConsentWith.and.returnValue(of(pendingConsent));

      service.proposeNegotiation({
        participantId: OTHER_AGENT_ID,
        consentId: 'consent-123',
      }).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('accepted');
          done();
        },
      });
    });
  });

  describe('acceptNegotiation', () => {
    it('should accept a proposed negotiation', (done) => {
      // First create a negotiation
      consentServiceSpy.getConsentWith.and.returnValue(of(mockIntimateConsent));

      service.proposeNegotiation({
        participantId: OTHER_AGENT_ID,
        consentId: 'consent-123',
      }).subscribe({
        next: (negotiation) => {
          // Switch agent to participant
          sourceChainSpy.getAgentId.and.returnValue(OTHER_AGENT_ID);

          service.acceptNegotiation(negotiation.id).subscribe({
            next: (accepted) => {
              // Should be in analyzing or negotiating state after acceptance
              expect(['analyzing', 'negotiating']).toContain(accepted.status);
              done();
            },
            error: done.fail,
          });
        },
        error: done.fail,
      });
    });

    it('should error when negotiation not found', (done) => {
      service.acceptNegotiation('nonexistent').subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('not found');
          done();
        },
      });
    });
  });

  describe('declineNegotiation', () => {
    it('should decline a proposed negotiation', (done) => {
      consentServiceSpy.getConsentWith.and.returnValue(of(mockIntimateConsent));

      service.proposeNegotiation({
        participantId: OTHER_AGENT_ID,
        consentId: 'consent-123',
      }).subscribe({
        next: (negotiation) => {
          // Switch agent to participant
          sourceChainSpy.getAgentId.and.returnValue(OTHER_AGENT_ID);

          service.declineNegotiation(negotiation.id, 'Not ready').subscribe({
            next: () => {
              const negotiations = (service as any).negotiationsSubject.value;
              const declined = negotiations.find((n: any) => n.id === negotiation.id);
              expect(declined.status).toBe('declined');
              done();
            },
            error: done.fail,
          });
        },
        error: done.fail,
      });
    });
  });

  describe('generateBridgingPath', () => {
    it('should generate a path structure', (done) => {
      // Create mock negotiation in negotiating state
      const mockNegotiation = {
        id: 'neg-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        status: 'negotiating' as NegotiationStatus,
        consentId: 'consent-123',
        requiredIntimacyLevel: 'intimate' as IntimacyLevel,
        validatingAttestationIds: [],
        sharedAffinityNodes: ['concept-1', 'concept-2', 'concept-3'],
        divergentNodes: {
          initiator: ['concept-4', 'concept-5'],
          participant: ['concept-6', 'concept-7'],
        },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        negotiationLog: [],
      };

      (service as any).negotiationsSubject.next([mockNegotiation]);

      service.generateBridgingPath('neg-123', 'maximum_overlap').subscribe({
        next: (proposedPath) => {
          expect(proposedPath.title).toBe('Common Ground Path');
          expect(proposedPath.conceptIds).toEqual(['concept-1', 'concept-2', 'concept-3']);
          expect(proposedPath.stats.sharedConcepts).toBe(3);
          done();
        },
        error: done.fail,
      });
    });

    it('should generate complementary path with teaching concepts', (done) => {
      const mockNegotiation = {
        id: 'neg-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        status: 'negotiating' as NegotiationStatus,
        consentId: 'consent-123',
        requiredIntimacyLevel: 'intimate' as IntimacyLevel,
        validatingAttestationIds: [],
        sharedAffinityNodes: ['shared-1', 'shared-2', 'shared-3', 'shared-4'],
        divergentNodes: {
          initiator: ['init-1', 'init-2', 'init-3', 'init-4'],
          participant: ['part-1', 'part-2', 'part-3', 'part-4'],
        },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        negotiationLog: [],
      };

      (service as any).negotiationsSubject.next([mockNegotiation]);

      service.generateBridgingPath('neg-123', 'complementary').subscribe({
        next: (proposedPath) => {
          expect(proposedPath.title).toBe('Mutual Learning Path');
          // Should have mix of initiator, shared, and participant concepts
          expect(proposedPath.stats.initiatorTeaching).toBeGreaterThan(0);
          expect(proposedPath.stats.participantTeaching).toBeGreaterThan(0);
          done();
        },
        error: done.fail,
      });
    });

    it('should error when negotiation not in negotiating state', (done) => {
      const mockNegotiation = {
        id: 'neg-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        status: 'proposed' as NegotiationStatus,
        consentId: 'consent-123',
        requiredIntimacyLevel: 'intimate' as IntimacyLevel,
        validatingAttestationIds: [],
        sharedAffinityNodes: [],
        divergentNodes: { initiator: [], participant: [] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        negotiationLog: [],
      };

      (service as any).negotiationsSubject.next([mockNegotiation]);

      service.generateBridgingPath('neg-123', 'maximum_overlap').subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('negotiating state');
          done();
        },
      });
    });
  });

  describe('acceptGeneratedPath', () => {
    it('should accept path and finalize negotiation', (done) => {
      const mockNegotiation = {
        id: 'neg-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        status: 'negotiating' as NegotiationStatus,
        consentId: 'consent-123',
        requiredIntimacyLevel: 'intimate' as IntimacyLevel,
        validatingAttestationIds: [],
        sharedAffinityNodes: ['concept-1'],
        divergentNodes: { initiator: [], participant: [] },
        proposedPathStructure: {
          title: 'Test Path',
          description: 'A test path',
          stepCount: 1,
          estimatedDuration: '15 minutes',
          conceptIds: ['concept-1'],
          stats: { sharedConcepts: 1, initiatorTeaching: 0, participantTeaching: 0, novelConcepts: 0 },
        },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        negotiationLog: [],
      };

      (service as any).negotiationsSubject.next([mockNegotiation]);

      service.acceptGeneratedPath({
        negotiationId: 'neg-123',
        accept: true,
      }).subscribe({
        next: (negotiation) => {
          expect(negotiation.status).toBe('accepted');
          expect(negotiation.generatedPathId).toBeDefined();
          expect(negotiation.resolvedAt).toBeDefined();
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getMyNegotiations', () => {
    it('should return all negotiations for current agent', (done) => {
      const negotiations = [
        {
          id: 'neg-1',
          initiatorId: AGENT_ID,
          participantId: OTHER_AGENT_ID,
          status: 'proposed' as NegotiationStatus,
          consentId: 'c1',
          requiredIntimacyLevel: 'intimate' as IntimacyLevel,
          validatingAttestationIds: [],
          sharedAffinityNodes: [],
          divergentNodes: { initiator: [], participant: [] },
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          negotiationLog: [],
        },
        {
          id: 'neg-2',
          initiatorId: 'another-agent',
          participantId: AGENT_ID,
          status: 'accepted' as NegotiationStatus,
          consentId: 'c2',
          requiredIntimacyLevel: 'intimate' as IntimacyLevel,
          validatingAttestationIds: [],
          sharedAffinityNodes: [],
          divergentNodes: { initiator: [], participant: [] },
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          negotiationLog: [],
        },
      ];

      (service as any).negotiationsSubject.next(negotiations);

      service.getMyNegotiations().subscribe({
        next: (result) => {
          expect(result.length).toBe(2);
          done();
        },
        error: done.fail,
      });
    });

    it('should filter by status', (done) => {
      const negotiations = [
        {
          id: 'neg-1',
          initiatorId: AGENT_ID,
          participantId: OTHER_AGENT_ID,
          status: 'proposed' as NegotiationStatus,
          consentId: 'c1',
          requiredIntimacyLevel: 'intimate' as IntimacyLevel,
          validatingAttestationIds: [],
          sharedAffinityNodes: [],
          divergentNodes: { initiator: [], participant: [] },
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          negotiationLog: [],
        },
        {
          id: 'neg-2',
          initiatorId: AGENT_ID,
          participantId: 'another-agent',
          status: 'accepted' as NegotiationStatus,
          consentId: 'c2',
          requiredIntimacyLevel: 'intimate' as IntimacyLevel,
          validatingAttestationIds: [],
          sharedAffinityNodes: [],
          divergentNodes: { initiator: [], participant: [] },
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          negotiationLog: [],
        },
      ];

      (service as any).negotiationsSubject.next(negotiations);

      service.getMyNegotiations({ status: 'proposed' }).subscribe({
        next: (result) => {
          expect(result.length).toBe(1);
          expect(result[0].id).toBe('neg-1');
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('sendMessage', () => {
    it('should add a message to active negotiation', (done) => {
      const mockNegotiation = {
        id: 'neg-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        status: 'negotiating' as NegotiationStatus,
        consentId: 'consent-123',
        requiredIntimacyLevel: 'intimate' as IntimacyLevel,
        validatingAttestationIds: [],
        sharedAffinityNodes: [],
        divergentNodes: { initiator: [], participant: [] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        negotiationLog: [],
      };

      (service as any).negotiationsSubject.next([mockNegotiation]);

      service.sendMessage('neg-123', 'Hello!', 'comment').subscribe({
        next: (negotiation) => {
          expect(negotiation.negotiationLog.length).toBe(1);
          expect(negotiation.negotiationLog[0].content).toBe('Hello!');
          expect(negotiation.negotiationLog[0].type).toBe('comment');
          done();
        },
        error: done.fail,
      });
    });

    it('should error on resolved negotiation', (done) => {
      const mockNegotiation = {
        id: 'neg-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        status: 'accepted' as NegotiationStatus,
        consentId: 'consent-123',
        requiredIntimacyLevel: 'intimate' as IntimacyLevel,
        validatingAttestationIds: [],
        sharedAffinityNodes: [],
        divergentNodes: { initiator: [], participant: [] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        negotiationLog: [],
      };

      (service as any).negotiationsSubject.next([mockNegotiation]);

      service.sendMessage('neg-123', 'Hello!').subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('resolved');
          done();
        },
      });
    });
  });
});
