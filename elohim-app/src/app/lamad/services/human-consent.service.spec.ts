import { TestBed } from '@angular/core/testing';
import { HumanConsentService } from './human-consent.service';
import { LocalSourceChainService } from './local-source-chain.service';
import { SessionHumanService } from './session-human.service';
import { IntimacyLevel, ConsentState } from '../models/human-consent.model';
import { LearningPath } from '../models/learning-path.model';

describe('HumanConsentService', () => {
  let service: HumanConsentService;
  let sourceChainSpy: jasmine.SpyObj<LocalSourceChainService>;
  let sessionHumanSpy: jasmine.SpyObj<SessionHumanService>;

  const AGENT_ID = 'test-agent-123';
  const OTHER_AGENT_ID = 'other-agent-456';

  beforeEach(() => {
    sourceChainSpy = jasmine.createSpyObj('LocalSourceChainService', [
      'isInitialized',
      'getAgentId',
      'getEntriesByType',
      'createEntry',
    ]);
    sessionHumanSpy = jasmine.createSpyObj('SessionHumanService', ['getCurrentSession']);

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
        HumanConsentService,
        { provide: LocalSourceChainService, useValue: sourceChainSpy },
        { provide: SessionHumanService, useValue: sessionHumanSpy },
      ],
    });

    service = TestBed.inject(HumanConsentService);
    service.initialize();
  });

  describe('sendRecognition', () => {
    it('should create a recognition relationship with not_required consent', (done) => {
      service.sendRecognition(OTHER_AGENT_ID, 'Great work!').subscribe({
        next: (consent) => {
          expect(consent.initiatorId).toBe(AGENT_ID);
          expect(consent.participantId).toBe(OTHER_AGENT_ID);
          expect(consent.intimacyLevel).toBe('recognition');
          expect(consent.consentState).toBe('not_required');
          expect(consent.requestMessage).toBe('Great work!');
          done();
        },
        error: done.fail,
      });
    });

    it('should error when sending recognition to self', (done) => {
      service.sendRecognition(AGENT_ID).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('yourself');
          done();
        },
      });
    });

    it('should error when relationship already exists', (done) => {
      // Create first relationship
      service.sendRecognition(OTHER_AGENT_ID).subscribe({
        next: () => {
          // Try to create second
          service.sendRecognition(OTHER_AGENT_ID).subscribe({
            next: () => done.fail('Should have errored'),
            error: (err) => {
              expect(err.message).toContain('already exists');
              done();
            },
          });
        },
        error: done.fail,
      });
    });
  });

  describe('requestConnection', () => {
    it('should create a pending connection request', (done) => {
      service.requestConnection(OTHER_AGENT_ID, 'Let\'s connect!').subscribe({
        next: (consent) => {
          expect(consent.initiatorId).toBe(AGENT_ID);
          expect(consent.participantId).toBe(OTHER_AGENT_ID);
          expect(consent.intimacyLevel).toBe('connection');
          expect(consent.consentState).toBe('pending');
          expect(consent.requestMessage).toBe('Let\'s connect!');
          done();
        },
        error: done.fail,
      });
    });

    it('should error when requesting connection with self', (done) => {
      service.requestConnection(AGENT_ID).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('yourself');
          done();
        },
      });
    });
  });

  describe('acceptConsent', () => {
    it('should accept a pending consent request', (done) => {
      // Create request as OTHER_AGENT (simulated)
      const mockConsent = {
        id: 'consent-123',
        initiatorId: OTHER_AGENT_ID,
        participantId: AGENT_ID, // Current agent is participant
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'pending' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      // Inject mock consent
      (service as any).consentsSubject.next([mockConsent]);

      service.acceptConsent('consent-123').subscribe({
        next: (consent) => {
          expect(consent.consentState).toBe('accepted');
          expect(consent.consentedAt).toBeDefined();
          done();
        },
        error: done.fail,
      });
    });

    it('should error when consent not found', (done) => {
      service.acceptConsent('nonexistent').subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('not found');
          done();
        },
      });
    });

    it('should error when current agent is not the participant', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: AGENT_ID, // Current agent is initiator, not participant
        participantId: OTHER_AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'pending' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.acceptConsent('consent-123').subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('participant');
          done();
        },
      });
    });
  });

  describe('declineConsent', () => {
    it('should decline a pending consent request', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: OTHER_AGENT_ID,
        participantId: AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'pending' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.declineConsent('consent-123', 'Not interested').subscribe({
        next: () => {
          const consents = (service as any).consentsSubject.value;
          const updated = consents.find((c: any) => c.id === 'consent-123');
          expect(updated.consentState).toBe('declined');
          expect(updated.responseMessage).toBe('Not interested');
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('revokeConsent', () => {
    it('should revoke an accepted consent', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'accepted' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.revokeConsent('consent-123', 'Changed my mind').subscribe({
        next: () => {
          const consents = (service as any).consentsSubject.value;
          const updated = consents.find((c: any) => c.id === 'consent-123');
          expect(updated.consentState).toBe('revoked');
          done();
        },
        error: done.fail,
      });
    });

    it('should error when consent is not accepted', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'pending' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.revokeConsent('consent-123').subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('not accepted');
          done();
        },
      });
    });
  });

  describe('proposeElevation', () => {
    it('should propose elevation from connection to trusted', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'accepted' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.proposeElevation({
        consentId: 'consent-123',
        newLevel: 'trusted',
        message: 'I trust you',
      }).subscribe({
        next: (consent) => {
          expect(consent.intimacyLevel).toBe('trusted');
          expect(consent.consentState).toBe('pending');
          expect(consent.elevationAttempts).toBe(1);
          done();
        },
        error: done.fail,
      });
    });

    it('should require attestation type for intimate level', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        intimacyLevel: 'trusted' as IntimacyLevel,
        consentState: 'accepted' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.proposeElevation({
        consentId: 'consent-123',
        newLevel: 'intimate',
      }).subscribe({
        next: () => done.fail('Should have errored'),
        error: (err) => {
          expect(err.message).toContain('attestation type');
          done();
        },
      });
    });
  });

  describe('getConsentWith', () => {
    it('should find consent where current agent is initiator', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'accepted' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.getConsentWith(OTHER_AGENT_ID).subscribe({
        next: (consent) => {
          expect(consent).toBeTruthy();
          expect(consent?.id).toBe('consent-123');
          done();
        },
        error: done.fail,
      });
    });

    it('should find consent where current agent is participant', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: OTHER_AGENT_ID,
        participantId: AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'accepted' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      service.getConsentWith(OTHER_AGENT_ID).subscribe({
        next: (consent) => {
          expect(consent).toBeTruthy();
          expect(consent?.id).toBe('consent-123');
          done();
        },
        error: done.fail,
      });
    });

    it('should return null when no consent exists', (done) => {
      service.getConsentWith('unknown-agent').subscribe({
        next: (consent) => {
          expect(consent).toBeNull();
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getPendingRequests', () => {
    it('should return pending requests where current agent is participant', (done) => {
      const pendingConsent = {
        id: 'consent-1',
        initiatorId: OTHER_AGENT_ID,
        participantId: AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'pending' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      const acceptedConsent = {
        id: 'consent-2',
        initiatorId: 'another-agent',
        participantId: AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'accepted' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([pendingConsent, acceptedConsent]);

      service.getPendingRequests().subscribe({
        next: (pending) => {
          expect(pending.length).toBe(1);
          expect(pending[0].id).toBe('consent-1');
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('canViewPath', () => {
    it('should allow viewing public paths', (done) => {
      const path = {
        visibility: 'public',
        createdBy: OTHER_AGENT_ID,
      } as LearningPath;

      service.canViewPath(path).subscribe({
        next: (canView) => {
          expect(canView).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should allow creator to view their own paths', (done) => {
      const path = {
        visibility: 'intimate',
        createdBy: AGENT_ID,
      } as LearningPath;

      service.canViewPath(path).subscribe({
        next: (canView) => {
          expect(canView).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should allow viewing paths where user is participant', (done) => {
      const path = {
        visibility: 'intimate',
        createdBy: OTHER_AGENT_ID,
        participantIds: [AGENT_ID],
      } as LearningPath;

      service.canViewPath(path).subscribe({
        next: (canView) => {
          expect(canView).toBe(true);
          done();
        },
        error: done.fail,
      });
    });

    it('should deny viewing connection paths without consent', (done) => {
      const path = {
        visibility: 'connections',
        createdBy: OTHER_AGENT_ID,
      } as LearningPath;

      service.canViewPath(path).subscribe({
        next: (canView) => {
          expect(canView).toBe(false);
          done();
        },
        error: done.fail,
      });
    });

    it('should allow viewing connection paths with accepted consent', (done) => {
      const mockConsent = {
        id: 'consent-123',
        initiatorId: AGENT_ID,
        participantId: OTHER_AGENT_ID,
        intimacyLevel: 'connection' as IntimacyLevel,
        consentState: 'accepted' as ConsentState,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [],
      };

      (service as any).consentsSubject.next([mockConsent]);

      const path = {
        visibility: 'connections',
        createdBy: OTHER_AGENT_ID,
      } as LearningPath;

      service.canViewPath(path).subscribe({
        next: (canView) => {
          expect(canView).toBe(true);
          done();
        },
        error: done.fail,
      });
    });
  });

  describe('getHumansAtLevel', () => {
    it('should return humans at or above the specified level', (done) => {
      const consents = [
        {
          id: 'c1',
          initiatorId: AGENT_ID,
          participantId: 'human-1',
          intimacyLevel: 'recognition' as IntimacyLevel,
          consentState: 'not_required' as ConsentState,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [],
        },
        {
          id: 'c2',
          initiatorId: AGENT_ID,
          participantId: 'human-2',
          intimacyLevel: 'connection' as IntimacyLevel,
          consentState: 'accepted' as ConsentState,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [],
        },
        {
          id: 'c3',
          initiatorId: AGENT_ID,
          participantId: 'human-3',
          intimacyLevel: 'trusted' as IntimacyLevel,
          consentState: 'accepted' as ConsentState,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [],
        },
      ];

      (service as any).consentsSubject.next(consents);

      service.getHumansAtLevel('connection').subscribe({
        next: (humans) => {
          expect(humans.length).toBe(2);
          expect(humans).toContain('human-2');
          expect(humans).toContain('human-3');
          expect(humans).not.toContain('human-1');
          done();
        },
        error: done.fail,
      });
    });
  });
});
